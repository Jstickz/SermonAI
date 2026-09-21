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

/// Class names YouVersion and most Bible APIs use for non-scripture spans.
const DROPPED_CLASSES: [&str; 5] = ["note", "footnote", "crossref", "heading", "label"];

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
/// Bible APIs mark verses with either a `data-verse`/`data-usfm` attribute or
/// a `verse vN` class, so both are recognised.
fn verse_number(tag: &str) -> Option<u16> {
    let lower = tag.to_lowercase();
    if !lower.contains("verse") {
        return None;
    }

    for key in [
        "data-verse=\"",
        "data-verse='",
        "data-number=\"",
        "data-number='",
    ] {
        if let Some(rest) = lower.split(key).nth(1) {
            if let Some(value) = rest.split(['"', '\'']).next() {
                if let Ok(number) = value.trim().parse::<u16>() {
                    return Some(number);
                }
            }
        }
    }

    // data-usfm="JHN.3.16" — the verse is the last segment.
    for key in ["data-usfm=\"", "data-usfm='"] {
        if let Some(rest) = lower.split(key).nth(1) {
            if let Some(value) = rest.split(['"', '\'']).next() {
                if let Some(last) = value.rsplit('.').next() {
                    if let Ok(number) = last.trim().parse::<u16>() {
                        return Some(number);
                    }
                }
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
