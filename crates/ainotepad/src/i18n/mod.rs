use serde::{Deserialize, Serialize};

mod catalog;
mod message;
pub use catalog::{find_match_count, known_models_count, localized_document_name, text, TextKey};
pub use message::{FailureReason, UiMessage};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    ZhCn,
    En,
    #[default]
    #[serde(other)]
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    ZhCn,
    En,
}

pub fn resolve_locale(language: UiLanguage, system_tag: Option<&str>) -> Locale {
    match language {
        UiLanguage::ZhCn => Locale::ZhCn,
        UiLanguage::En => Locale::En,
        UiLanguage::System => {
            let is_chinese = system_tag.map(str::trim).is_some_and(|tag| {
                tag.get(..2)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("zh"))
            });
            if is_chinese {
                Locale::ZhCn
            } else {
                Locale::En
            }
        }
    }
}

#[cfg(windows)]
pub fn windows_user_locale_tag() -> Option<String> {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    let mut buffer = [0_u16; 85];
    let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if written <= 1 {
        return None;
    }
    String::from_utf16(&buffer[..written as usize - 1]).ok()
}

#[cfg(not(windows))]
pub fn windows_user_locale_tag() -> Option<String> {
    None
}

pub fn completion_state_key(state: ainotepad_ai::CompletionState) -> TextKey {
    match state {
        ainotepad_ai::CompletionState::Empty => TextKey::CompletionEmpty,
        ainotepad_ai::CompletionState::Requesting => TextKey::CompletionRequesting,
        ainotepad_ai::CompletionState::Suggested => TextKey::CompletionSuggested,
        ainotepad_ai::CompletionState::NotConfigured => TextKey::CompletionNotConfigured,
        ainotepad_ai::CompletionState::Timeout => TextKey::CompletionTimeout,
        ainotepad_ai::CompletionState::AuthFailed => TextKey::CompletionAuthFailed,
        ainotepad_ai::CompletionState::NoSuggestion => TextKey::CompletionNoSuggestion,
        ainotepad_ai::CompletionState::RequestFailed => TextKey::CompletionRequestFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainotepad_ai::CompletionState;

    #[test]
    fn explicit_language_ignores_system_tag() {
        assert_eq!(
            resolve_locale(UiLanguage::ZhCn, Some("en-US")),
            Locale::ZhCn
        );
        assert_eq!(resolve_locale(UiLanguage::En, Some("zh-CN")), Locale::En);
    }

    #[test]
    fn system_language_maps_chinese_tags_to_simplified_chinese() {
        for tag in ["zh-CN", "zh-Hans", "zh-TW", "ZH-hk"] {
            assert_eq!(resolve_locale(UiLanguage::System, Some(tag)), Locale::ZhCn);
        }
    }

    #[test]
    fn system_language_falls_back_to_english() {
        assert_eq!(
            resolve_locale(UiLanguage::System, Some("en-US")),
            Locale::En
        );
        assert_eq!(
            resolve_locale(UiLanguage::System, Some("ja-JP")),
            Locale::En
        );
        assert_eq!(resolve_locale(UiLanguage::System, None), Locale::En);
    }

    #[test]
    fn completion_states_map_to_typed_keys() {
        assert_eq!(
            completion_state_key(CompletionState::Empty),
            TextKey::CompletionEmpty
        );
        assert_eq!(
            completion_state_key(CompletionState::Requesting),
            TextKey::CompletionRequesting
        );
        assert_eq!(
            completion_state_key(CompletionState::Suggested),
            TextKey::CompletionSuggested
        );
        assert_eq!(
            completion_state_key(CompletionState::NotConfigured),
            TextKey::CompletionNotConfigured
        );
        assert_eq!(
            completion_state_key(CompletionState::Timeout),
            TextKey::CompletionTimeout
        );
        assert_eq!(
            completion_state_key(CompletionState::AuthFailed),
            TextKey::CompletionAuthFailed
        );
        assert_eq!(
            completion_state_key(CompletionState::NoSuggestion),
            TextKey::CompletionNoSuggestion
        );
        assert_eq!(
            completion_state_key(CompletionState::RequestFailed),
            TextKey::CompletionRequestFailed
        );
    }
}
