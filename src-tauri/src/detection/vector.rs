//! Offline semantic scripture detection: stage 2 of the pipeline (FR-15).
//!
//! A spoken phrase is embedded by the same static model that built
//! `verse-index.bin`, then matched against all 31,102 verses by brute-force
//! cosine. Both sides are int8 and every row is unit length, so the search is
//! an integer dot product — no float maths, no index structure to corrupt, and
//! nothing to rebuild.
//!
//! Static embeddings mean "embedding" here is a token lookup and a mean. There
//! is no transformer, no ONNX runtime and no GPU, which is what keeps this
//! inside the 40 MB installer (ADR 0006, ADR 0003).
//!
//! Assets are produced by `scripts/build-verse-index.py`:
//!   encoder/encoder.bin     int8 token matrix with a per-row scale
//!   encoder/tokenizer.json  the matching tokenizer
//!   verse-index.bin         int8 verse vectors plus their references

use std::path::Path;

use tokenizers::Tokenizer;

use crate::bible::books;
use crate::error::{Error, Result};

const ENCODER_MAGIC: &[u8; 8] = b"SAIENC01";
const INDEX_MAGIC: &[u8; 8] = b"SAIVIDX1";

/// model2vec drops the unknown token before pooling; matching that exactly is
/// what keeps Rust query vectors in the same space as the Python-built index.
const UNK_TOKEN_ID: u32 = 1;

/// A verse the index can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerseRef {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
}

impl VerseRef {
    /// Operator-facing reference, e.g. "John 3:16".
    pub fn display(&self) -> String {
        let book = books::name(self.book).unwrap_or("?");
        format!("{book} {}:{}", self.chapter, self.verse)
    }

    /// USFM form used by the Bible cache, e.g. "JHN 3:16".
    pub fn usfm(&self) -> String {
        let book = books::code(self.book).unwrap_or("???");
        format!("{book} {}:{}", self.chapter, self.verse)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Match {
    pub verse: VerseRef,
    /// Cosine similarity in 0.0..=1.0.
    pub score: f32,
    /// Set when the hit is a two-verse passage (`verse` and the next), from
    /// the neighbouring-verse search. `None` for a single verse.
    pub end_verse: Option<u8>,
}

impl Match {
    /// Whether `want` is this verse or inside this passage.
    pub fn covers(&self, want: &VerseRef) -> bool {
        self.verse.book == want.book
            && self.verse.chapter == want.chapter
            && self.verse.verse <= want.verse
            && want.verse <= self.end_verse.unwrap_or(self.verse.verse)
    }
}

/// How the synonym variants of a query are searched (encoder upgrade
/// Phase 1). Measured in `examples/measure_vector.rs`; the pipeline uses
/// whichever the measurement kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SynonymMode {
    /// The query as spoken.
    #[default]
    Off,
    /// Embed every variant, average the vectors, one search. Costs one
    /// search however many variants there are.
    Blend,
    /// Search every variant, keep each verse's best score. Costs a search
    /// per variant.
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchOptions {
    pub synonyms: SynonymMode,
    /// Also search two-verse passages, so a paraphrase that runs across a
    /// verse boundary scores against both verses together.
    pub neighbours: bool,
}

/// The bundled static sentence encoder.
pub struct Encoder {
    dims: usize,
    /// Per-token scale, undoing the int8 quantization of that row.
    scales: Vec<f32>,
    matrix: Vec<i8>,
    tokenizer: Tokenizer,
}

impl Encoder {
    pub fn load(assets_dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(assets_dir.join("encoder/encoder.bin"))?;
        if bytes.len() < 20 || &bytes[..8] != ENCODER_MAGIC {
            return Err(Error::Detection(
                "the bundled encoder is missing or corrupt. Reinstall SermonAI".into(),
            ));
        }

        let dims = read_u32(&bytes, 12)? as usize;
        let vocab = read_u32(&bytes, 16)? as usize;

        let scales_start = 20;
        let matrix_start = scales_start + vocab * 4;
        let expected = matrix_start + vocab * dims;
        if bytes.len() < expected {
            return Err(Error::Detection(format!(
                "the bundled encoder is truncated: {} bytes, expected {expected}",
                bytes.len()
            )));
        }

        let scales = bytes[scales_start..matrix_start]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let matrix = bytes[matrix_start..expected]
            .iter()
            .map(|&b| b as i8)
            .collect();

        let tokenizer = Tokenizer::from_file(assets_dir.join("encoder/tokenizer.json"))
            .map_err(|e| Error::Detection(format!("could not load the tokenizer: {e}")))?;

        Ok(Self {
            dims,
            scales,
            matrix,
            tokenizer,
        })
    }

    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Embed a phrase into a unit-length vector.
    ///
    /// This mirrors model2vec's `encode`: tokenize without special tokens,
    /// drop unknown tokens, mean-pool the token rows, then L2-normalise. Any
    /// deviation here silently moves queries out of the index's space, so the
    /// tests compare against vectors the Python build script produced.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| Error::Detection(format!("could not tokenize the phrase: {e}")))?;

        let mut sum = vec![0f32; self.dims];
        let mut counted = 0usize;

        for &id in encoding.get_ids() {
            if id == UNK_TOKEN_ID {
                continue;
            }
            let row = id as usize;
            if row >= self.scales.len() {
                continue;
            }

            let scale = self.scales[row] / 127.0;
            let start = row * self.dims;
            for (out, &q) in sum.iter_mut().zip(&self.matrix[start..start + self.dims]) {
                *out += q as f32 * scale;
            }
            counted += 1;
        }

        if counted == 0 {
            return Ok(sum); // all-zero: nothing recognisable was said
        }

        let inv = 1.0 / counted as f32;
        for value in &mut sum {
            *value *= inv;
        }
        normalize(&mut sum);
        Ok(sum)
    }

    /// Embed and quantize, ready for the integer dot product in `VerseIndex`.
    pub fn embed_quantized(&self, text: &str) -> Result<Vec<i8>> {
        Ok(quantize(&self.embed(text)?))
    }
}

/// Every verse embedded by the same model, quantized to int8.
pub struct VerseIndex {
    dims: usize,
    refs: Vec<VerseRef>,
    vectors: Vec<i8>,
    /// Row `i` of `vectors` averaged with row `i + 1` when that is the next
    /// verse of the same chapter, re-normalised and quantized: a two-verse
    /// passage in the same space. `pair_rows[j]` is the first verse's row.
    /// Built in memory at load, about 7.7 MB; nothing new ships.
    pair_vectors: Vec<i8>,
    pair_rows: Vec<usize>,
}

impl VerseIndex {
    pub fn load(assets_dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(assets_dir.join("verse-index.bin"))?;
        if bytes.len() < 52 || &bytes[..8] != INDEX_MAGIC {
            return Err(Error::Detection(
                "the bundled verse index is missing or corrupt. Reinstall SermonAI".into(),
            ));
        }

        let dims = read_u32(&bytes, 12)? as usize;
        let count = read_u32(&bytes, 16)? as usize;

        // 8 magic + 12 header + 32 model id
        let refs_start = 52;
        let vectors_start = refs_start + count * 4;
        let expected = vectors_start + count * dims;
        if bytes.len() < expected {
            return Err(Error::Detection(format!(
                "the bundled verse index is truncated: {} bytes, expected {expected}",
                bytes.len()
            )));
        }

        let refs: Vec<VerseRef> = bytes[refs_start..vectors_start]
            .chunks_exact(4)
            .map(|c| VerseRef {
                book: c[0],
                chapter: c[1],
                verse: c[2],
            })
            .collect();

        let vectors: Vec<i8> = bytes[vectors_start..expected]
            .iter()
            .map(|&b| b as i8)
            .collect();

        let (pair_vectors, pair_rows) = build_pairs(dims, &refs, &vectors);

        Ok(Self {
            dims,
            refs,
            vectors,
            pair_vectors,
            pair_rows,
        })
    }

    /// Top `k` two-verse passages for a quantized query, best first. Same
    /// brute force as [`search`], over one row fewer per chapter.
    pub fn search_pairs(&self, query: &[i8], k: usize) -> Vec<Match> {
        if query.len() != self.dims || self.pair_rows.is_empty() || k == 0 {
            return Vec::new();
        }
        const SCALE: f32 = 1.0 / (127.0 * 127.0);
        let mut best: Vec<(i32, usize)> = Vec::with_capacity(k + 1);
        for (j, chunk) in self.pair_vectors.chunks_exact(self.dims).enumerate() {
            let dot = dot_product(query, chunk);
            if best.len() < k {
                best.push((dot, j));
                best.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
            } else if dot > best[k - 1].0 {
                best[k - 1] = (dot, j);
                best.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
            }
        }
        best.into_iter()
            .map(|(dot, j)| {
                let first = self.refs[self.pair_rows[j]];
                Match {
                    verse: first,
                    score: (dot as f32 * SCALE).clamp(-1.0, 1.0),
                    end_verse: Some(first.verse + 1),
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Top `k` verses for an already-quantized query, best first.
    ///
    /// Brute force over every verse. At 31,102 x 256 that is about 8 million
    /// multiply-adds, measured at roughly 2.5 ms per query against FR-15's 5 ms
    /// budget, and it avoids any index structure that could go stale or corrupt.
    pub fn search(&self, query: &[i8], k: usize) -> Vec<Match> {
        if query.len() != self.dims || self.refs.is_empty() || k == 0 {
            return Vec::new();
        }

        // Both sides were unit length before quantizing, so the dot product of
        // two int8 rows is cosine scaled by 127^2.
        const SCALE: f32 = 1.0 / (127.0 * 127.0);

        let mut best: Vec<(i32, usize)> = Vec::with_capacity(k + 1);

        for (row, chunk) in self.vectors.chunks_exact(self.dims).enumerate() {
            let dot = dot_product(query, chunk);

            if best.len() < k {
                best.push((dot, row));
                best.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
            } else if dot > best[k - 1].0 {
                best[k - 1] = (dot, row);
                best.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
            }
        }

        best.into_iter()
            .map(|(dot, row)| Match {
                verse: self.refs[row],
                score: (dot as f32 * SCALE).clamp(-1.0, 1.0),
                end_verse: None,
            })
            .collect()
    }
}

/// Average each verse's vector with the next verse of the same chapter.
///
/// The encoder mean-pools tokens, so the mean of two unit verse vectors is
/// close to what embedding the two verses as one text would give, without
/// the text. Re-normalised so the dot product stays a cosine.
fn build_pairs(dims: usize, refs: &[VerseRef], vectors: &[i8]) -> (Vec<i8>, Vec<usize>) {
    let mut pair_vectors = Vec::with_capacity(vectors.len());
    let mut pair_rows = Vec::with_capacity(refs.len());
    let mut sum = vec![0f32; dims];
    for row in 0..refs.len().saturating_sub(1) {
        let (a, b) = (refs[row], refs[row + 1]);
        if a.book != b.book || a.chapter != b.chapter || b.verse != a.verse + 1 {
            continue;
        }
        let ra = &vectors[row * dims..(row + 1) * dims];
        let rb = &vectors[(row + 1) * dims..(row + 2) * dims];
        for ((out, &x), &y) in sum.iter_mut().zip(ra).zip(rb) {
            *out = f32::from(x) + f32::from(y);
        }
        normalize(&mut sum);
        pair_vectors.extend(quantize(&sum));
        pair_rows.push(row);
    }
    (pair_vectors, pair_rows)
}

/// Encoder plus index: what the detection pipeline actually holds.
pub struct SemanticSearch {
    encoder: Encoder,
    index: VerseIndex,
}

impl SemanticSearch {
    pub fn load(assets_dir: &Path) -> Result<Self> {
        let encoder = Encoder::load(assets_dir)?;
        let index = VerseIndex::load(assets_dir)?;

        if encoder.dims() != index.dims() {
            return Err(Error::Detection(format!(
                "encoder is {} dims but the verse index is {} — these assets are from different builds",
                encoder.dims(),
                index.dims()
            )));
        }

        Ok(Self { encoder, index })
    }

    pub fn search(&self, phrase: &str, k: usize) -> Result<Vec<Match>> {
        Ok(self.index.search(&self.encoder.embed_quantized(phrase)?, k))
    }

    /// [`search`] with the Phase 1 options: synonym variants of the query
    /// and/or two-verse passages. With the defaults this is exactly
    /// [`search`].
    pub fn search_with(
        &self,
        phrase: &str,
        k: usize,
        options: &SearchOptions,
    ) -> Result<Vec<Match>> {
        let variants = super::synonyms::Variants::of(phrase);
        let texts = match options.synonyms {
            SynonymMode::Off => vec![phrase],
            _ => variants.distinct(),
        };

        // One or more quantized queries, depending on the mode.
        let queries: Vec<Vec<i8>> = match options.synonyms {
            SynonymMode::Blend if texts.len() > 1 => {
                let mut sum = vec![0f32; self.encoder.dims()];
                for text in &texts {
                    for (out, v) in sum.iter_mut().zip(self.encoder.embed(text)?) {
                        *out += v;
                    }
                }
                normalize(&mut sum);
                vec![quantize(&sum)]
            }
            _ => texts
                .iter()
                .map(|t| self.encoder.embed_quantized(t))
                .collect::<Result<_>>()?,
        };

        // Best score per distinct verse-or-passage across every query and
        // both indexes; then the top k of those.
        let mut best: Vec<Match> = Vec::new();
        for query in &queries {
            let mut hits = self.index.search(query, k);
            if options.neighbours {
                hits.extend(self.index.search_pairs(query, k));
            }
            for hit in hits {
                match best
                    .iter_mut()
                    .find(|m| m.verse == hit.verse && m.end_verse == hit.end_verse)
                {
                    Some(existing) => existing.score = existing.score.max(hit.score),
                    None => best.push(hit),
                }
            }
        }
        best.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        best.truncate(k);
        Ok(best)
    }

    pub fn index(&self) -> &VerseIndex {
        &self.index
    }

    pub fn encoder(&self) -> &Encoder {
        &self.encoder
    }
}

/// Integer dot product of two int8 vectors of equal length.
///
/// This runs 31,102 times per query — about 8 million multiply-adds — and is
/// the whole cost of the vector stage, so it is written to vectorise. Four
/// independent accumulators break the loop-carried dependency chain that
/// otherwise serialises the adds, and the fixed-size chunks give the compiler
/// something it can widen into SIMD.
///
/// Measured on the M0 reference machine, whole vector stage per query:
///   15.98 ms  opt-level = "s", single accumulator
///    5.77 ms  opt-level = 3, four accumulators
///    2.46 ms  opt-level = 3, eight accumulators
/// against FR-15's 5 ms budget.
#[inline]
fn dot_product(a: &[i8], b: &[i8]) -> i32 {
    const LANES: usize = 8;

    let mut acc = [0i32; LANES];
    let mut chunks_a = a.chunks_exact(LANES);
    let mut chunks_b = b.chunks_exact(LANES);

    for (ca, cb) in chunks_a.by_ref().zip(chunks_b.by_ref()) {
        for lane in 0..LANES {
            acc[lane] += ca[lane] as i32 * cb[lane] as i32;
        }
    }

    let mut total: i32 = acc.iter().sum();
    for (&x, &y) in chunks_a.remainder().iter().zip(chunks_b.remainder()) {
        total += x as i32 * y as i32;
    }
    total
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| Error::Detection("asset header is truncated".into()))
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn quantize(vector: &[f32]) -> Vec<i8> {
    vector
        .iter()
        .map(|&v| (v * 127.0).round().clamp(-127.0, 127.0) as i8)
        .collect()
}
