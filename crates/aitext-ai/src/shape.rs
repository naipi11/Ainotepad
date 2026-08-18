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
}
