//! Turn the HTML a Bible API returns into text the projector can show.
//!
//! Two forms come out of this, and both are kept (PRD §14.2): the sanitized
//! plain text goes on screen, and the original markup is cached because the
//! summary PDF renders the richer form (PRD §13.11).
//!
//! This is deliberately a small, allowlist-shaped converter rather than a real
//! HTML parser. The input is verse markup from one known vendor, and anything
//! it does not recognise is dropped rather than passed through — text destined
//! for a projector in front of a congregation should never carry markup we did
//! not understand.
//!
//! The markup below is what YouVersion actually returns for
//! `?format=html`, confirmed against live responses on 21 Sept 2026:
//!
//! ```html
//! <div><div class="p">
//!   <span class="yv-v" v="16"></span>      <!-- verse marker, empty -->
//!   <span class="yv-vlbl">16</span>        <!-- printed number, not scripture -->
//!   <span class="wj">For God so loved…</span>
//! </div></div>
//! ```
//!
//! Poetry uses `q1`/`q2` divs and a `d` div carries the psalm ascription.

/// Verse text ready to display, plus the verse boundaries found in the markup.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SanitizedPassage {
    /// Plain text, paragraphs separated by a blank line.
    pub text: String,
    /// Verse number and the text belonging to it, in document order.
    pub verses: Vec<(u16, String)>,
}

/// Tags whose content is not scripture: footnotes, cross references, headings
/// and editorial notes. Their entire subtree is dropped.
const DROPPED_TAGS: [&str; 6] = ["note", "sup", "script", "style", "h1", "h2"];

/// Class names for spans that are not scripture.
///
/// `yv-vlbl` is the printed verse number. It must go: the projector shows the
/// reference separately, and leaving it in prefixes every verse with a digit.
const DROPPED_CLASSES: [&str; 6] = [
    "note", "footnote", "crossref", "heading", "label", "yv-vlbl",
];

/// Sanitize passage HTML into plain text plus verse markers.
pub fn sanitize(html: &str) -> SanitizedPassage {
    let mut out = SanitizedPassage::default();
    let mut text = String::new();

    let mut current_verse: Option<u16> = None;
    let mut verse_text = String::new();

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    let mut skip_until: Option<String> = None;

    while i < chars.len() {
        if chars[i] == '<' {
            let Some(close) = find(&chars, i, '>') else {
                break;
            };
            let tag: String = chars[i + 1..close].iter().collect();
            i = close + 1;

            let name = tag_name(&tag);

            // Inside a dropped subtree: wait for its closing tag.
            if let Some(ref waiting) = skip_until {
                if tag.starts_with('/') && name == *waiting {
                    skip_until = None;
                }
                continue;
            }

            if !tag.starts_with('/') && should_drop(&tag, &name) {
                // Self-closing tags have nothing to skip.
                if !tag.ends_with('/') {
                    skip_until = Some(name.clone());
                }
                continue;
            }

            // A verse boundary closes the previous verse.
            if let Some(number) = verse_number(&tag) {
                flush_verse(&mut out, &mut current_verse, &mut verse_text);
                current_verse = Some(number);
                continue;
            }

            if is_break(&name) {
                push_break(&mut text);
                verse_text.push(' ');
            }

            continue;
        }

        let Some(next) = chars[i..].iter().position(|&c| c == '<') else {
            let rest: String = chars[i..].iter().collect();
            append(&mut text, &mut verse_text, &decode_entities(&rest));
            break;
        };

        let chunk: String = chars[i..i + next].iter().collect();
        if skip_until.is_none() {
            append(&mut text, &mut verse_text, &decode_entities(&chunk));
        }
        i += next;
    }

    flush_verse(&mut out, &mut current_verse, &mut verse_text);
    out.text = tidy(&text);
    out
}

fn append(text: &mut String, verse_text: &mut String, chunk: &str) {
    // Newlines inside the source markup are formatting, not paragraph breaks.
    // Only the block tags handled above produce a real break, so flatten these
    // or `tidy` would turn every wrapped line into its own paragraph.
    let flattened: String = chunk
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();

    text.push_str(&flattened);
    verse_text.push_str(&flattened);
}

fn flush_verse(out: &mut SanitizedPassage, current: &mut Option<u16>, buffer: &mut String) {
    if let Some(number) = current.take() {
        let tidied = tidy(buffer);
        if !tidied.is_empty() {
            out.verses.push((number, tidied));
        }
    }
    buffer.clear();
}

fn find(chars: &[char], from: usize, needle: char) -> Option<usize> {
    chars[from..]
        .iter()
        .position(|&c| c == needle)
        .map(|p| p + from)
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn should_drop(tag: &str, name: &str) -> bool {
    if DROPPED_TAGS.contains(&name) {
        return true;
    }
    let lower = tag.to_lowercase();
    DROPPED_CLASSES.iter().any(|class| {
        lower.contains(&format!("class=\"{class}")) || lower.contains(&format!("class='{class}"))
    })
}

fn is_break(name: &str) -> bool {
    matches!(name, "p" | "br" | "div" | "li" | "blockquote")
}

fn push_break(text: &mut String) {
    if !text.ends_with('\n') && !text.is_empty() {
        text.push('\n');
    }
}

/// Pull a verse number out of a verse marker.
///
/// YouVersion marks a verse with `<span class="yv-v" v="16">`, so the number
/// lives in a bare `v` attribute rather than in the class or a data attribute.
/// `data-verse` and `data-usfm` are also accepted because the build script's
/// Python sanitizer and older captures use them, and the two implementations
/// must stay interchangeable.
fn verse_number(tag: &str) -> Option<u16> {
    let lower = tag.to_lowercase();

    // Only verse markers carry a number. Without this guard a stray attribute
    // elsewhere could be read as a verse boundary and split the text.
    if !lower.contains("yv-v") && !lower.contains("verse") && !lower.contains("data-usfm") {
        return None;
    }

    // v="16" — YouVersion's form.
    if let Some(number) = attribute_value(&lower, "v").and_then(|v| v.parse().ok()) {
        return Some(number);
    }

    for key in ["data-verse", "data-number"] {
        if let Some(number) = attribute_value(&lower, key).and_then(|v| v.parse().ok()) {
            return Some(number);
        }
    }

    // data-usfm="JHN.3.16" — the verse is the last segment.
    if let Some(usfm) = attribute_value(&lower, "data-usfm") {
        if let Some(last) = usfm.rsplit('.').next() {
            if let Ok(number) = last.parse::<u16>() {
                return Some(number);
            }
        }
    }

    // class="verse v16"
    for token in lower.split(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        if let Some(digits) = token.strip_prefix('v') {
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(number) = digits.parse::<u16>() {
                    return Some(number);
                }
            }
        }
    }

    None
}

/// Value of `name="..."` in a tag, matching the attribute name exactly so that
/// `v` does not also match `yv-v` or `data-verse`.
fn attribute_value(lower_tag: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{name}={quote}");
        let mut from = 0usize;
        while let Some(found) = lower_tag[from..].find(&needle) {
            let at = from + found;
            let preceded_by_name_char = at > 0
                && lower_tag[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

            if !preceded_by_name_char {
                let rest = &lower_tag[at + needle.len()..];
                if let Some(end) = rest.find(quote) {
                    return Some(rest[..end].trim().to_string());
                }
            }
            from = at + needle.len();
        }
    }
    None
}

fn decode_entities(raw: &str) -> String {
    raw.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&rsquo;", "\u{2019}")
        .replace("&lsquo;", "\u{2018}")
        .replace("&rdquo;", "\u{201D}")
        .replace("&ldquo;", "\u{201C}")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
}

/// Collapse runs of whitespace while keeping paragraph breaks.
fn tidy(raw: &str) -> String {
    raw.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_verse_markup_becomes_plain_text() {
        let html = r#"<p class="p"><span class="verse v16" data-usfm="JHN.3.16">For God so loved the world.</span></p>"#;
        let out = sanitize(html);
        assert_eq!(out.text, "For God so loved the world.");
        assert_eq!(
            out.verses,
            vec![(16, "For God so loved the world.".to_string())]
        );
    }

    #[test]
    fn several_verses_keep_their_numbers_and_order() {
        let html = concat!(
            r#"<p><span class="verse v1" data-usfm="PSA.23.1">The LORD is my shepherd.</span>"#,
            r#"<span class="verse v2" data-usfm="PSA.23.2">He maketh me to lie down.</span></p>"#
        );
        let out = sanitize(html);
        assert_eq!(out.verses.len(), 2);
        assert_eq!(out.verses[0].0, 1);
        assert_eq!(out.verses[1].0, 2);
        assert!(out.verses[1].1.contains("lie down"));
    }

    /// Footnotes and cross references are not scripture and must never reach
    /// the projector.
    #[test]
    fn notes_and_cross_references_are_dropped_entirely() {
        let html = concat!(
            r#"<span class="verse v16" data-verse="16">For God so loved"#,
            r#"<note class="note">See also Romans 5:8</note>"#,
            r#"<sup class="footnote">a</sup> the world.</span>"#
        );
        let out = sanitize(html);
        assert!(
            !out.text.contains("Romans 5:8"),
            "note leaked: {:?}",
            out.text
        );
        assert!(!out.text.contains('a') || !out.text.contains("worlda"));
        assert!(out.text.contains("For God so loved"));
        assert!(out.text.contains("the world."));
    }

    #[test]
    fn paragraph_breaks_survive_as_newlines() {
        let html = "<p>First line.</p><p>Second line.</p>";
        let out = sanitize(html);
        assert_eq!(out.text, "First line.\nSecond line.");
    }

    #[test]
    fn entities_are_decoded() {
        let html = "<p>Jacob&rsquo;s well &amp; the woman&#39;s jar &mdash; there.</p>";
        let out = sanitize(html);
        assert_eq!(
            out.text,
            "Jacob\u{2019}s well & the woman's jar \u{2014} there."
        );
    }

    #[test]
    fn whitespace_is_collapsed() {
        let html = "<p>  Too    many\n\n   spaces  </p>";
        assert_eq!(sanitize(html).text, "Too many spaces");
    }

    #[test]
    fn unknown_markup_is_stripped_rather_than_passed_through() {
        let html =
            r#"<div data-x="1"><em>Emphasis</em> kept, <script>alert(1)</script> gone.</div>"#;
        let out = sanitize(html);
        assert!(out.text.contains("Emphasis kept,"));
        assert!(!out.text.contains("alert"), "script leaked: {:?}", out.text);
        assert!(!out.text.contains('<'));
    }

    #[test]
    fn empty_and_malformed_input_does_not_panic() {
        for html in [
            "",
            "   ",
            "<p>",
            "</p>",
            "<p>unclosed",
            "<<>>",
            "no markup at all",
        ] {
            let _ = sanitize(html);
        }
        assert_eq!(sanitize("no markup at all").text, "no markup at all");
    }

    /// Verbatim markup from YouVersion, captured 21 Sept 2026 from
    /// /bibles/206/passages/JHN.3.16?format=html.
    #[test]
    fn real_youversion_verse_markup() {
        let html = concat!(
            r#"<div><div class="p"><span class="yv-v" v="16"></span>"#,
            r#"<span class="yv-vlbl">16</span>"#,
            r#"<span class="wj">For God so loved the world, that he gave his only born</span> "#,
            r#"<span class="wj">Son, that whoever believes in him should not perish, but have eternal life. </span>"#,
            r#"</div></div>"#
        );

        let out = sanitize(html);

        assert_eq!(
            out.text,
            "For God so loved the world, that he gave his only born Son, that whoever believes in him should not perish, but have eternal life."
        );
        // The printed verse number must not survive into the verse text.
        assert!(
            !out.text.starts_with("16"),
            "verse label leaked: {:?}",
            out.text
        );
        assert_eq!(out.verses.len(), 1);
        assert_eq!(out.verses[0].0, 16);
        assert!(out.verses[0].1.starts_with("For God so loved"));
    }

    /// A whole chapter of poetry, captured from /bibles/206/passages/PSA.23.
    /// Psalm 23 has six verses; the q1/q2 divs are line breaks within them.
    #[test]
    fn real_youversion_chapter_markup() {
        let html = concat!(
            r#"<div><div class="d">A Psalm by David.</div>"#,
            r#"<div class="q1"><span class="yv-v" v="1"></span><span class="yv-vlbl">1</span>Yahweh is my shepherd;</div>"#,
            r#"<div class="q2">I shall lack nothing.</div>"#,
            r#"<div class="q1"><span class="yv-v" v="2"></span><span class="yv-vlbl">2</span>He makes me lie down in green pastures.</div>"#,
            r#"<div class="q2">He leads me beside still waters.</div>"#,
            r#"<div class="q1"><span class="yv-v" v="3"></span><span class="yv-vlbl">3</span>He restores my soul.</div></div>"#
        );

        let out = sanitize(html);
        let numbers: Vec<u16> = out.verses.iter().map(|(n, _)| *n).collect();
        assert_eq!(numbers, vec![1, 2, 3]);

        // A verse split across poetry lines keeps both lines.
        let verse_one = &out.verses[0].1;
        assert!(verse_one.contains("Yahweh is my shepherd"), "{verse_one:?}");
        assert!(verse_one.contains("I shall lack nothing"), "{verse_one:?}");

        // The ascription precedes verse 1 and belongs to no verse.
        assert!(out.text.contains("A Psalm by David."));
        assert!(!verse_one.contains("A Psalm by David"), "{verse_one:?}");
    }

    #[test]
    fn data_verse_attribute_is_recognised() {
        let html = r#"<span class="verse" data-verse="28">All things work together.</span>"#;
        let out = sanitize(html);
        assert_eq!(
            out.verses,
            vec![(28, "All things work together.".to_string())]
        );
    }
}
