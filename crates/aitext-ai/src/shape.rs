pub const MAX_PROSE_CHARS: usize = 16;
pub const MAX_STATEMENT_CHARS: usize = 80;
pub const MAX_BLOCK_CHARS: usize = 400;
pub const MAX_BLOCK_LINES: usize = 8;

#[cfg(test)]
mod language_mode_tests {
    use super::*;

    #[test]
    fn cpp_language_keeps_a_statement_that_has_no_heuristic_prefix_marker() {
        let shaped =
            shape_suggestion_for_language("lo\" << std::endl;", "std::cout << \"Hel", "cpp");

        assert_eq!(shaped.as_deref(), Some("lo\" << std::endl;"));
    }

    #[test]
    fn context_shaping_removes_model_echo_of_existing_suffix() {
        let shaped = shape_suggestion_for_context(" world is bright", "Hello", " world", "plain");

        assert_eq!(shaped.as_deref(), Some(" is bright"));
    }

    #[test]
    fn markdown_shaping_uses_prose_budget_even_when_prefix_has_parentheses() {
        let shaped = shape_suggestion_for_language(
            "This is a sentence that continues naturally.",
            "A note (with a parenthesis)",
            "markdown",
        )
        .expect("prose completion should remain visible");

        assert!(shaped.chars().count() <= MAX_PROSE_CHARS);
    }
}

use crate::snapshot::CompletionMode;

pub fn shape_suggestion(raw: &str, prefix: &str) -> Option<String> {
    let mode = if looks_like_code(prefix) {
        CompletionMode::Code
    } else {
        CompletionMode::PlainText
    };
    shape_suggestion_with_mode(raw, prefix, mode)
}

pub fn shape_suggestion_for_language(raw: &str, prefix: &str, language: &str) -> Option<String> {
    shape_suggestion_with_mode(raw, prefix, CompletionMode::from_language(language))
}

pub fn shape_suggestion_for_context(
    raw: &str,
    prefix: &str,
    suffix: &str,
    language: &str,
) -> Option<String> {
    let text = shape_suggestion_for_language(raw, prefix, language)?;
    let text = strip_suffix_echo(&text, suffix).trim_end().to_string();
    (!text.is_empty()).then_some(text)
}

fn strip_suffix_echo(text: &str, suffix: &str) -> String {
    if text.is_empty() || suffix.is_empty() {
        return text.to_string();
    }
    let overlap = text
        .chars()
        .zip(suffix.chars())
        .take_while(|(text, suffix)| text == suffix)
        .count();
    if overlap == 0 {
        return text.to_string();
    }
    let overlap_text: String = text.chars().take(overlap).collect();
    let suffix_len = suffix.chars().count();
    if overlap_text.chars().any(|ch| !ch.is_whitespace()) || overlap == suffix_len {
        text.chars().skip(overlap).collect()
    } else {
        text.to_string()
    }
}

fn shape_suggestion_with_mode(raw: &str, prefix: &str, mode: CompletionMode) -> Option<String> {
    let mut text = strip_fences(raw);
    text = strip_echoed_prefix(&text, prefix);
    text = strip_overlap(&text, prefix);
    text = strip_prompt_artifacts(&text);
    if looks_like_meta_completion(&text) {
        return None;
    }
    if prefix_is_cjk(prefix) && has_unexpected_latin(&text, prefix) {
        text = take_before_latin(&text);
    }
    if looks_like_token_spam(&text) || looks_like_invented_identity(prefix, &text) {
        return None;
    }
    text = clip_repeats(prefix, &text);
    text = clip_to_mode(prefix, &text, mode);
    text = collapse_question_restatement(prefix, &text);
    if mode == CompletionMode::Code {
        text = complete_unclosed_delimiters(prefix, &text);
    }
    let text = text.trim_end().to_string();
    if text.is_empty() || looks_like_meta_completion(&text) || looks_like_blog_opener(&text) {
        None
    } else {
        Some(text)
    }
}

pub fn repair_unclosed_code_completion(prefix: &str, text: &str) -> String {
    let trimmed = text.trim_start();
    if trimmed.starts_with('(') {
        let mut chars = trimmed.chars();
        let _ = chars.next();
        if let Some(quote @ ('\'' | '"')) = chars.next() {
            if !trimmed.ends_with(')') {
                let mut repaired = text.to_string();
                if !trimmed.ends_with(quote) {
                    repaired.push(quote);
                }
                repaired.push(')');
                return repaired;
            }
        }
    }
    let current_line = prefix.lines().last().unwrap_or(prefix).trim_end();
    let text = text.trim_end_matches(['\r', '\n']);
    if current_line.ends_with('(')
        && matches!(text.chars().next(), Some('\'' | '"'))
        && !text.ends_with(')')
    {
        let quote = text.chars().next().unwrap();
        return format!("{text}{quote})");
    }
    complete_unclosed_delimiters(prefix, text)
}

fn complete_unclosed_delimiters(prefix: &str, text: &str) -> String {
    let text = text.trim_end_matches(['\r', '\n']);
    let (first_line, rest) = text.split_once('\n').unwrap_or((text, ""));
    if first_line.trim().is_empty() && !rest.is_empty() {
        return text.to_string();
    }
    let repaired = complete_unclosed_single_line(prefix, first_line);
    if repaired == first_line || rest.is_empty() {
        return if rest.is_empty() {
            repaired
        } else {
            text.to_string()
        };
    }
    format!("{repaired}\n{rest}")
}

fn complete_unclosed_single_line(prefix: &str, text: &str) -> String {
    if !looks_like_code(prefix) {
        return text.to_string();
    }

    let combined = format!("{prefix}{text}");
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;

    for ch in combined.chars() {
        if comment {
            continue;
        }
        if let Some(open_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == open_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '#' => comment = true,
            '\'' | '"' => quote = Some(ch),
            '(' | '[' | '{' => stack.push(ch),
            ')' | ']' | '}' if stack.last().copied() == Some(matching_open(ch)) => {
                stack.pop();
            }
            _ => {}
        }
    }

    let mut suffix = String::new();
    if let Some(open_quote) = quote {
        suffix.push(open_quote);
    }
    while let Some(open) = stack.pop() {
        suffix.push(matching_close(open));
    }
    if suffix.is_empty() {
        text.to_string()
    } else {
        format!("{text}{suffix}")
    }
}

fn matching_open(close: char) -> char {
    match close {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => close,
    }
}

fn matching_close(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => open,
    }
}

fn strip_fences(raw: &str) -> String {
    let mut text = raw.to_string();
    let trimmed = text.trim_start();
    if trimmed.starts_with("```") {
        text = trimmed.trim_start_matches('`').to_string();
        if let Some(idx) = text.find('\n') {
            text = text[idx + 1..].to_string();
        }
        if let Some(idx) = text.rfind("```") {
            text = text[..idx].to_string();
        }
    }
    text
}

fn strip_echoed_prefix(text: &str, prefix: &str) -> String {
    if !prefix.is_empty() && text.starts_with(prefix) {
        text[prefix.len()..].to_string()
    } else {
        text.to_string()
    }
}

fn strip_overlap(text: &str, prefix: &str) -> String {
    let prefix_chars: Vec<char> = prefix.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    if prefix_chars.len() < 2 {
        return text.to_string();
    }
    let max = prefix_chars.len().min(text_chars.len());
    if max < 2 {
        return text.to_string();
    }
    for n in (2..=max).rev() {
        if prefix_chars[prefix_chars.len() - n..] == text_chars[..n] {
            return text_chars[n..].iter().collect();
        }
    }
    text.to_string()
}

fn strip_prompt_artifacts(text: &str) -> String {
    text.replace("TEXT_BEFORE", "")
        .replace("TEXT_AFTER", "")
        .replace("<<<", "")
        .replace(">>>", "")
}

fn looks_like_meta_completion(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("complete this")
        || lower.contains("complete the text")
        || lower.contains("we need to")
        || lower.contains("the user says")
        || lower.contains("file=untitled")
        || lower.contains("lang=plain")
        || lower.contains("text_before")
        || lower.contains("text_after")
        || lower.contains("before=[")
        || lower.contains("after=[")
}

fn looks_like_invented_identity(prefix: &str, text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let prefix_l = prefix.to_ascii_lowercase();
    if prefix_l.contains("alex") {
        return false;
    }
    lower.contains("alex") || text.contains("我来自") || text.contains("我叫")
}

fn looks_like_blog_opener(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("让我们")
        || trimmed.starts_with("下面")
        || trimmed.starts_with("首先")
        || trimmed.starts_with("本文将")
        || trimmed.starts_with("今天，")
        || trimmed.contains("一起来了解")
        || trimmed.to_ascii_lowercase().contains("in this article")
}

fn looks_like_token_spam(text: &str) -> bool {
    let mut prev_cjk = false;
    let mut spaced = 0usize;
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        let ch = chars[i];
        if ch == ' ' && prev_cjk && chars.get(i + 1).copied().map(is_cjk).unwrap_or(false) {
            spaced += 1;
        }
        prev_cjk = is_cjk(ch);
    }
    spaced >= 2
}

fn has_unexpected_latin(text: &str, prefix: &str) -> bool {
    if prefix.chars().any(|ch| ch.is_ascii_alphabetic()) || looks_like_code(prefix) {
        return false;
    }
    text.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn take_before_latin(text: &str) -> String {
    text.chars()
        .take_while(|ch| !ch.is_ascii_alphabetic())
        .collect()
}

fn looks_like_code(prefix: &str) -> bool {
    let last = prefix.lines().last().unwrap_or(prefix);
    last.contains('{')
        || last.contains('}')
        || last.contains(';')
        || last.contains('(')
        || last.contains("def ")
        || last.contains("fn ")
        || last.contains("class ")
        || last.contains("printf")
        || prefix.contains("fn ")
        || prefix.contains("def ")
}

fn prefix_is_cjk(prefix: &str) -> bool {
    let last = prefix.lines().last().unwrap_or(prefix);
    last.chars().any(is_cjk)
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn clip_repeats(prefix: &str, text: &str) -> String {
    let last_line = prefix.lines().last().unwrap_or("");
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() {
        return String::new();
    }
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let completed_first = format!("{last_line}{}", lines[0]);
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();
        if i > 0 {
            if !last_line.trim().is_empty()
                && !completed_first.trim().is_empty()
                && (*line == completed_first || stripped == completed_first.trim())
            {
                break;
            }
            if !stripped.is_empty() && stripped.chars().count() >= 8 && seen.contains(stripped) {
                break;
            }
        }
        if !stripped.is_empty() {
            seen.insert(stripped.to_string());
        }
        out.push((*line).to_string());
    }
    out.join("\n")
}

fn clip_to_mode(prefix: &str, text: &str, mode: CompletionMode) -> String {
    if mode == CompletionMode::Code {
        let last = prefix.lines().last().unwrap_or("");
        let multiline = last.trim().is_empty()
            || last.trim_end().ends_with(':')
            || last.trim_end().ends_with('{')
            || last.trim_end().ends_with(',');
        if multiline {
            return cap_chars_and_lines(text, MAX_BLOCK_CHARS, MAX_BLOCK_LINES);
        }
        return cap_chars_and_lines(text, MAX_STATEMENT_CHARS, 2);
    }
    let first = text.lines().next().unwrap_or(text);
    let first = clip_at_sentence(first);
    cap_chars_and_lines(&first, MAX_PROSE_CHARS, 1)
}

fn clip_at_sentence(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        if matches!(ch, '。' | '！' | '？' | '.' | '!' | '?') && out.chars().count() >= 1 {
            break;
        }
    }
    out
}

fn collapse_question_restatement(prefix: &str, text: &str) -> String {
    if !is_question_like(prefix) {
        return text.to_string();
    }
    let first = text.lines().next().unwrap_or(text);
    if !is_question_like(first) {
        return text.to_string();
    }
    let prefix_words = content_chars(prefix);
    let text_words = content_chars(first);
    if prefix_words.is_empty() || text_words.is_empty() {
        return text.to_string();
    }
    let overlap = prefix_words
        .iter()
        .filter(|ch| text_words.contains(ch))
        .count();
    if overlap * 2 >= prefix_words.len() {
        if !prefix.ends_with('？') && !prefix.ends_with('?') {
            return "？".to_string();
        }
        return String::new();
    }
    text.to_string()
}

fn is_question_like(text: &str) -> bool {
    let t = text.trim();
    t.ends_with('？')
        || t.ends_with('?')
        || t.contains("怎么")
        || t.contains("怎样")
        || t.contains("什么")
        || t.contains("吗")
        || t.contains("呢")
        || t.contains("如何")
}

fn content_chars(text: &str) -> Vec<char> {
    text.chars()
        .filter(|ch| is_cjk(*ch) || ch.is_ascii_alphanumeric())
        .filter(|ch| !matches!(*ch, '的' | '了' | '吗' | '呢' | '啊' | '呀'))
        .collect()
}

fn cap_chars_and_lines(text: &str, max_chars: usize, max_lines: usize) -> String {
    let mut lines = 0usize;
    let mut out = String::new();
    for ch in text.chars() {
        if out.chars().count() >= max_chars {
            break;
        }
        if ch == '\n' {
            lines += 1;
            if lines >= max_lines {
                break;
            }
        }
        out.push(ch);
    }
    out.trim_end().to_string()
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
    fn prose_stays_on_one_short_line() {
        let long = "line\n".repeat(8);
        let shaped = shape_suggestion(&long, "hello").unwrap();
        assert_eq!(shaped.lines().count(), 1);
        assert!(shaped.chars().count() <= MAX_PROSE_CHARS);
    }

    #[test]
    fn keeps_continuation_when_model_repeats_line() {
        let shaped = shape_suggestion("1+2=3", "1+2").unwrap();
        assert_eq!(shaped, "=3");
    }

    #[test]
    fn keeps_leading_space() {
        let shaped = shape_suggestion(" world\");", "printf(\"hello").unwrap();
        assert_eq!(shaped, " world\");");
    }

    #[test]
    fn rejects_english_meta_reasoning() {
        let raw = "We need to complete the text. The user says: Complete this.";
        assert!(shape_suggestion(raw, "\u{4f60}\u{597d}").is_none());
    }

    #[test]
    fn clips_weather_essay_to_question_mark() {
        let raw = "\u{4eca}\u{5929}\u{7684}\u{5929}\u{6c14}\u{5982}\u{4f55}\u{ff1f}\u{8ba9}\u{6211}\u{4eec}\u{4e00}\u{8d77}\u{6765}\u{4e86}\u{89e3}\u{4e00}\u{4e0b}\u{ff01}\n\nxx";
        let shaped = shape_suggestion(
            raw,
            "\u{4eca}\u{5929}\u{5929}\u{6c14}\u{600e}\u{4e48}\u{6837}",
        )
        .unwrap();
        assert_eq!(shaped, "\u{ff1f}");
    }

    #[test]
    fn rejects_spaced_cjk_spam_and_latin_identity() {
        let raw = "ex \u{6211}\u{6765}\u{81ea} alex";
        let shaped = shape_suggestion(raw, "\u{4f60}\u{4eec}\u{597d}");
        assert!(shaped.is_none() || !shaped.unwrap().to_ascii_lowercase().contains("alex"));
    }

    #[test]
    fn strips_scaffold_labels() {
        assert_eq!(
            shape_suggestion("printf>>>TEXT_AFTER<<<", "printf").as_deref(),
            None
        );
    }

    #[test]
    fn clips_repeated_printf_block() {
        let raw = " world\")\nprintf(\"hello world\")\nprintf(\"hello world\")";
        let shaped = shape_suggestion(raw, "printf(\"hello").unwrap();
        assert_eq!(shaped, " world\")");
    }

    #[test]
    fn strips_overlapping_hello() {
        let shaped = shape_suggestion("hello a", "printf(\"hello").unwrap();
        assert_ne!(shaped, "hello a");
        assert!(!shaped.starts_with("hello"));
    }

    #[test]
    fn completes_an_unclosed_python_string_call() {
        assert_eq!(
            shape_suggestion("print(\"Hello, World!", "print(\"Hello, World!").as_deref(),
            Some("\")")
        );
        assert_eq!(
            shape_suggestion("\"This is a test", "print(").as_deref(),
            Some("\"This is a test\")")
        );
        assert_eq!(
            shape_suggestion("\"Hello, World!\n", "print(").as_deref(),
            Some("\"Hello, World!\")")
        );
        assert_eq!(
            shape_suggestion("\"Hello, World!\nprint(\"next", "print(").as_deref(),
            Some("\"Hello, World!\")\nprint(\"next")
        );
        assert_eq!(
            repair_unclosed_code_completion("# previous\nprint(", "\"Hello, World!"),
            "\"Hello, World!\")"
        );
        assert_eq!(
            repair_unclosed_code_completion("print", "(\"Hello, World!"),
            "(\"Hello, World!\")"
        );
    }
}
