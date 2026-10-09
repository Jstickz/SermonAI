//! Modern preaching vocabulary mapped onto the words the KJV uses for the
//! same idea, so a paraphrase can be searched in the index's own language
//! (FR-15; encoder upgrade Phase 1).
//!
//! The bundled encoder is a static model: it averages a vector per token and
//! has no way to know that "plans to prosper you" and "thoughts of peace" say
//! the same thing. The verse index is built from the KJV, so a preacher's
//! modern wording is a different bag of tokens from the verse it quotes. This
//! module rewrites the query towards the KJV's bag before embedding.
//!
//! Three rewrites, each a separate variant so the measurement can tell them
//! apart (`examples/measure_vector.rs`):
//!
//! 1. **Contractions expanded** — "don't" tokenizes as `don ' t`, which
//!    matches nothing; "do not" matches the KJV's "not".
//! 2. **Vocabulary** — single words and short idioms replaced by the KJV's
//!    term for the idea ("worry" → "careful", "wounds" → "stripes",
//!    "good news" → "gospel"). Not a thesaurus: each entry is a word the KJV
//!    actually uses where a preacher today uses the other.
//! 3. **Pronouns** — "you" → "thee", "your" → "thy", since the KJV addresses
//!    the reader that way far more often than not.
//!
//! The table is ours, written from the KJV text against the paraphrases a
//! preacher says. Nothing here is taken from another project (CLAUDE.md:
//! ideas only from Rhema, no code or data). Every entry is one-directional —
//! modern → KJV — because the index is KJV and the query is not.
//!
//! How the variants are combined is a search option, not decided here; see
//! `vector::SynonymMode`.

/// `(modern, kjv)` — the modern word or short phrase and what the KJV says
/// instead. Matched on whole words, case-insensitively; longer phrases are
/// applied before shorter ones so "heavy loads" wins over "loads".
const VOCABULARY: &[(&str, &str)] = &[
    // Jeremiah 29:11 and the language of providence
    ("plans", "thoughts"),
    ("prosper you", "give you peace"),
    ("prosper", "prosper"),
    ("a future", "an expected end"),
    ("succeed", "prosper"),
    // Anxiety: "be careful for nothing", "take no thought"
    ("anxious", "careful"),
    ("worry", "thought"),
    ("worried", "careful"),
    ("worrying", "careful"),
    ("stress", "care"),
    // Matthew 11:28
    ("tired", "weary"),
    ("exhausted", "weary"),
    ("heavy loads", "heavy laden"),
    ("burdened", "heavy laden"),
    ("burdens", "burden"),
    // Courage and fear
    ("brave", "of good courage"),
    ("courageous", "of good courage"),
    ("scared", "afraid"),
    ("terrified", "afraid"),
    ("discouraged", "dismayed"),
    // Salvation
    ("rescue", "deliver"),
    ("rescued", "delivered"),
    ("rescues", "delivereth"),
    ("wrongdoing", "iniquity"),
    ("wrongs", "transgressions"),
    ("failures", "sins"),
    ("compassion", "mercy"),
    ("kindness", "mercy"),
    ("forgiveness", "forgive"),
    // Strength: Isaiah 40:31, Philippians 4:13
    ("new strength", "renew their strength"),
    ("soar", "mount up"),
    ("strengthens", "strengtheneth"),
    ("gives me strength", "strengtheneth me"),
    ("everything", "all things"),
    // Isaiah 53:5, 1 Peter 2:24
    ("pierced", "wounded"),
    ("crushed", "bruised"),
    ("wounds", "stripes"),
    // Psalm 139:13
    ("knit me together", "covered me"),
    ("knit together", "covered"),
    // Luke 4:18, Matthew 28
    ("good news", "gospel"),
    ("end of the age", "end of the world"),
    // John 10:10, John 16:33
    ("to the full", "more abundantly"),
    ("take heart", "be of good cheer"),
    ("cheer up", "be of good cheer"),
    // Galatians 5:22, Ephesians 6
    ("patience", "longsuffering"),
    ("self control", "temperance"),
    ("schemes", "wiles"),
    ("armour", "armour"),
    // Philippians 4:8
    ("noble", "honest"),
    ("admirable", "of good report"),
    // Timothy, Hebrews, James
    ("god breathed", "given by inspiration of god"),
    ("meeting together", "assembling of ourselves together"),
    ("generously", "liberally"),
    ("quick to listen", "swift to hear"),
    ("slow to speak", "slow to speak"),
    ("trials", "temptations"),
    ("hardships", "tribulation"),
    ("chosen people", "chosen generation"),
    ("drives out", "casteth out"),
    ("disciplines", "chasteneth"),
    ("freedom", "liberty"),
    // Proverbs
    ("guard your heart", "keep thy heart"),
    ("guard", "keep"),
    ("three strands", "threefold"),
    ("press on", "press"),
    ("the goal", "the mark"),
    ("active", "powerful"),
    ("outside", "outward appearance"),
    // Matthew 6, 9; John 1
    ("money", "mammon"),
    ("workers", "labourers"),
    ("lived among us", "dwelt among us"),
    ("new creation", "new creature"),
    ("test me", "prove me"),
    ("withers", "withereth"),
    ("bring everything", "in every thing"),
    ("prayer and petition", "prayer and supplication"),
    ("anything", "nothing"),
];

/// Contractions a speech-to-text engine writes, which tokenize as noise.
const CONTRACTIONS: &[(&str, &str)] = &[
    ("don't", "do not"),
    ("doesn't", "does not"),
    ("didn't", "did not"),
    ("won't", "will not"),
    ("can't", "cannot"),
    ("isn't", "is not"),
    ("aren't", "are not"),
    ("wasn't", "was not"),
    ("weren't", "were not"),
    ("haven't", "have not"),
    ("hasn't", "has not"),
    ("i'm", "i am"),
    ("i've", "i have"),
    ("i'll", "i will"),
    ("you're", "you are"),
    ("you'll", "you will"),
    ("he's", "he is"),
    ("she's", "she is"),
    ("it's", "it is"),
    ("we're", "we are"),
    ("we'll", "we will"),
    ("they're", "they are"),
    ("they'll", "they will"),
    ("there's", "there is"),
    ("that's", "that is"),
    ("what's", "what is"),
    ("let's", "let us"),
];

/// Second person as the KJV writes it.
const PRONOUNS: &[(&str, &str)] = &[
    ("yourselves", "yourselves"),
    ("yourself", "thyself"),
    ("your", "thy"),
    ("yours", "thine"),
    ("you", "thee"),
];

/// A query rewritten towards the KJV's vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variants {
    /// The query as spoken, lower-cased, contractions expanded.
    pub plain: String,
    /// `plain` with the vocabulary map applied.
    pub vocabulary: String,
    /// `vocabulary` with second-person pronouns in the KJV's form.
    pub archaic: String,
}

impl Variants {
    pub fn of(text: &str) -> Self {
        let plain = apply(&text.to_lowercase(), CONTRACTIONS);
        let vocabulary = apply(&plain, VOCABULARY);
        let archaic = apply(&vocabulary, PRONOUNS);
        Self {
            plain,
            vocabulary,
            archaic,
        }
    }

    /// Only the rewrites that changed something, so a query the map does not
    /// touch costs one embedding rather than three.
    pub fn distinct(&self) -> Vec<&str> {
        let mut out = vec![self.plain.as_str()];
        if self.vocabulary != self.plain {
            out.push(self.vocabulary.as_str());
        }
        if self.archaic != self.vocabulary {
            out.push(self.archaic.as_str());
        }
        out
    }
}

/// Replace whole-word matches of each `(from, to)` pair, longest `from`
/// first so an idiom is not broken by one of its own words.
fn apply(text: &str, table: &[(&str, &str)]) -> String {
    let mut pairs: Vec<(&str, &str)> = table.to_vec();
    pairs.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));

    // Work on a word-boundary-padded copy so " plans " cannot match inside
    // "airplanes". Punctuation is split off its word first so "you," still
    // reads as the word "you"; apostrophes stay inside a word so contractions
    // match. The split is undone at the end.
    const PUNCT: [char; 6] = [',', '.', ';', ':', '!', '?'];
    let mut spaced = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        if PUNCT.contains(&ch) {
            spaced.push(' ');
        }
        spaced.push(ch);
    }
    let mut padded = format!(
        " {} ",
        spaced.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    for (from, to) in pairs {
        if from == to {
            continue;
        }
        let needle = format!(" {from} ");
        if padded.contains(&needle) {
            padded = padded.replace(&needle, &format!(" {to} "));
        }
    }
    let mut out = padded.trim().to_string();
    for p in PUNCT {
        out = out.replace(&format!(" {p}"), &p.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wireframe_example_is_rewritten_towards_the_kjv() {
        let v = Variants::of("I know the plans I have for you, plans to prosper you");
        assert_eq!(
            v.vocabulary,
            "i know the thoughts i have for you, thoughts to give you peace"
        );
        assert_eq!(
            v.archaic,
            "i know the thoughts i have for thee, thoughts to give thee peace"
        );
    }

    #[test]
    fn idioms_win_over_their_own_words() {
        // "guard your heart" must become "keep thy heart", not "keep thy
        // heart" via two separate hits with a pronoun pass in between.
        let v = Variants::of("Above all else guard your heart");
        assert_eq!(v.vocabulary, "above all else keep thy heart");
        assert_eq!(v.archaic, "above all else keep thy heart");
    }

    #[test]
    fn whole_words_only() {
        let v = Variants::of("the airplanes guarded the money");
        assert_eq!(v.vocabulary, "the airplanes guarded the mammon");
    }

    #[test]
    fn contractions_are_expanded_before_anything_else() {
        let v = Variants::of("Don't be anxious about anything");
        assert_eq!(v.plain, "do not be anxious about anything");
        assert_eq!(v.vocabulary, "do not be careful about nothing");
    }

    #[test]
    fn an_untouched_query_yields_one_variant() {
        let v = Variants::of("The LORD is my shepherd");
        assert_eq!(v.distinct(), vec!["the lord is my shepherd"]);
    }

    #[test]
    fn pronouns_only_change_the_archaic_variant() {
        let v = Variants::of("trust in the Lord with all your heart");
        assert_eq!(v.vocabulary, v.plain);
        assert_eq!(v.archaic, "trust in the lord with all thy heart");
        assert_eq!(v.distinct().len(), 2);
    }
}
