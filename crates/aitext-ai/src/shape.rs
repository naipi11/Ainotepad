pub const MAX_SUGGESTION_CHARS: usize = 120;
pub const MAX_SUGGESTION_LINES: usize = 4;

pub fn shape_suggestion(raw: &str, prefix: &str) -> Option<String> {
    let mut text = raw.trim().to_string();
    if text.starts_with("```") {
        text = text.trim_start_matches('`').to_string();
        if let Some(idx) = text.find('\n') {
            text = text[idx + 1..].to_string();
        }
        if let Some(idx) = text.rfind("```") {
            text = text[..idx].to_string();
        }
        text = text.trim().to_string();
    }
    if !prefix.is_empty() && text.starts_with(prefix) {
        text = text[prefix.len()..].to_string();
    }
    if looks_like_meta_completion(&text) {
        return extract_from_meta(&text, prefix);
    }
    if prefix_is_cjk(prefix) && is_mostly_english(&text) {
        return extract_from_meta(&text, prefix);
    }
    let mut lines = 0usize;
    let mut out = String::new();
    for ch in text.chars() {
        if out.chars().count() >= MAX_SUGGESTION_CHARS {
            break;
        }
        if ch == '\n' {
            lines += 1;
            if lines >= MAX_SUGGESTION_LINES {
                break;
            }
        }
        out.push(ch);
    }
    let out = out.trim_end().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn looks_like_meta_completion(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("complete this")
        || lower.contains("complete the text")
        || lower.contains("we need to")
        || lower.contains("the user says")
        || lower.contains("file=untitled")
        || lower.contains("lang=plain")
}

fn is_mostly_english(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 8 {
        return false;
    }
    let ascii = letters.iter().filter(|c| c.is_ascii_alphabetic()).count();
    ascii * 2 >= letters.len()
}

fn extract_from_meta(text: &str, prefix: &str) -> Option<String> {
    if let Some(quoted) = last_quoted(text) {
        if !looks_like_meta_completion(&quoted) {
            if let Some(shaped) = shape_plain(&quoted, prefix) {
                return Some(shaped);
            }
        }
    }
    if prefix_is_cjk(prefix) {
        if let Some(run) = last_cjk_run(text) {
            return shape_plain(&run, prefix);
        }
        return None;
    }
    None
}

fn shape_plain(text: &str, prefix: &str) -> Option<String> {
    let mut text = text.trim().to_string();
    if !prefix.is_empty() && text.starts_with(prefix) {
        text = text[prefix.len()..].to_string();
    }
    let text = text.trim().to_string();
    if text.is_empty() || looks_like_meta_completion(&text) {
        None
    } else {
        Some(text.chars().take(MAX_SUGGESTION_CHARS).collect())
    }
}

fn prefix_is_cjk(prefix: &str) -> bool {
    prefix.chars().any(|ch| ('一'..='鿿').contains(&ch))
}

fn last_cjk_run(text: &str) -> Option<String> {
    let mut current = String::new();
    let mut last = None;
    for ch in text.chars() {
        if ('一'..='鿿').contains(&ch) {
            current.push(ch);
        } else if !current.is_empty() {
            last = Some(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        last = Some(current);
    }
    last.filter(|s| s.chars().count() >= 1)
}

fn last_quoted(text: &str) -> Option<String> {
    let mut last = None;
    let mut current = String::new();
    let mut in_quote = false;
    for ch in text.chars() {
        if ch == '"' || ch == '“' || ch == '”' || ch == '「' || ch == '」' {
            if in_quote {
                if !current.trim().is_empty() {
                    last = Some(current.trim().to_string());
                }
                current.clear();
                in_quote = false;
            } else {
                in_quote = true;
                current.clear();
            }
        } else if in_quote {
            current.push(ch);
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_repeated_prefix_and_fences() {
        let shaped = shape_suggestion("```\nhello world\n```", "hel").unwrap();
        assert_eq!(shaped, "lo world");
    }

    #[test]
    fn caps_lines_and_chars() {
        let long = "a\n".repeat(10);
        let shaped = shape_suggestion(&long, "").unwrap();
        assert_eq!(shaped.lines().count(), 4);
        let chars = "x".repeat(200);
        assert_eq!(shape_suggestion(&chars, "").unwrap().chars().count(), 120);
    }

    #[test]
    fn keeps_continuation_when_model_repeats_line() {
        let shaped = shape_suggestion("1+2=3", "1+2").unwrap();
        assert_eq!(shaped, "=3");
    }

    #[test]
    fn rejects_english_meta_reasoning() {
        let raw = "We need to complete the text. The user says: Complete this.";
        assert!(shape_suggestion(raw, "你好").is_none());
    }

    #[test]
    fn extracts_cjk_from_reasoning() {
        let raw = "The user wrote 你好. A natural continuation is “世界”.";
        assert_eq!(shape_suggestion(raw, "你好").as_deref(), Some("世界"));
    }
}
