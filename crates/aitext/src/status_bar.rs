use aitext_core::{Encoding, NewlineStyle};
use egui::Ui;

use crate::commands::AitextApp;
use crate::config::StatusItem;
use crate::i18n::TextKey;
use crate::theme::shell_colors;

pub fn draw_status_bar(ui: &mut Ui, app: &AitextApp) {
    let shell = shell_colors(app.config.theme);
    ui.horizontal(|ui| {
        let mut first = true;
        for item in &app.config.status_items {
            if let Some(text) = status_text(app, *item) {
                if !first {
                    ui.separator();
                }
                let color = if *item == StatusItem::Completion
                    && matches!(
                        app.completion.engine.state(),
                        aitext_ai::CompletionState::Suggested
                    ) {
                    shell.ghost
                } else {
                    shell.muted_text
                };
                ui.label(egui::RichText::new(text).color(color));
                first = false;
            }
        }
    });
}
fn status_text(app: &AitextApp, item: StatusItem) -> Option<String> {
    match item {
        StatusItem::Cursor => {
            if let Some(doc) = app.workspace.current() {
                let (line, col) = doc.line_column();
                Some(format!("{line}:{col}"))
            } else {
                Some("1:1".into())
            }
        }
        StatusItem::Encoding => app
            .workspace
            .current()
            .map(|doc| encoding_name(doc.encoding())),
        StatusItem::Newline => app
            .workspace
            .current()
            .map(|doc| match doc.newline_style() {
                NewlineStyle::Lf => "LF".into(),
                NewlineStyle::Crlf => "CRLF".into(),
            }),
        StatusItem::Language => app
            .workspace
            .current()
            .map(|doc| format!("{:?}", doc.language())),
        StatusItem::Model => app.config.active_profile().map_or_else(
            || Some(app.tr(TextKey::ProfileUnset).into()),
            |profile| {
                if profile.selected_model.trim().is_empty() {
                    Some(format!(
                        "{} · {}",
                        profile.name,
                        app.tr(TextKey::ModelUnset)
                    ))
                } else {
                    Some(format!("{} · {}", profile.name, profile.selected_model))
                }
            },
        ),
        StatusItem::Completion => Some(app.completion_label_now().to_string()),
        StatusItem::Message => app.status_text(),
        StatusItem::Custom => {
            let text = app.config.status_custom.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
    }
}

fn encoding_name(encoding: Encoding) -> String {
    match encoding {
        Encoding::Utf8 => "UTF-8".into(),
        Encoding::Utf8Bom => "UTF-8 BOM".into(),
        Encoding::Utf16Le => "UTF-16 LE".into(),
        Encoding::Utf16Be => "UTF-16 BE".into(),
        Encoding::Gbk => "GBK".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::status_text;
    use crate::commands::AitextApp;
    use crate::config::{ApiProfile, StatusItem};
    use crate::i18n::UiLanguage;
    use aitext_ai::ProviderKind;

    #[test]
    fn status_bar_identifies_active_profile_and_model() {
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Grok", ProviderKind::Xai);
        profile.remember_model("grok-test");
        app.config.add_profile(profile);

        assert_eq!(
            status_text(&app, StatusItem::Model).as_deref(),
            Some("Grok · grok-test")
        );
    }

    #[test]
    fn status_bar_model_shows_safe_fallback_without_active_profile() {
        let app = AitextApp::new_for_test();

        assert_eq!(
            status_text(&app, StatusItem::Model).as_deref(),
            Some("profile unset")
        );
    }

    #[test]
    fn status_bar_model_marks_profile_without_selected_model() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Claude", ProviderKind::Anthropic));

        assert_eq!(
            status_text(&app, StatusItem::Model).as_deref(),
            Some("Claude · model unset")
        );
    }

    #[test]
    fn status_bar_model_fallback_follows_the_ui_language() {
        let mut app = AitextApp::new_for_test();
        app.set_ui_language(UiLanguage::ZhCn);

        assert_eq!(
            status_text(&app, StatusItem::Model).as_deref(),
            Some("未设置配置")
        );
    }
}
