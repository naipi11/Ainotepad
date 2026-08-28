use aitext_core::{Encoding, LanguageId, NewlineStyle};
use egui::Ui;

use crate::commands::AitextApp;
use crate::config::StatusItem;
use crate::i18n::{text, Locale, TextKey};
use crate::theme::shell_colors;

pub fn draw_status_bar(ui: &mut Ui, app: &mut AitextApp) {
    let shell = shell_colors(app.config.theme);
    let items = app.config.status_items.clone();
    ui.horizontal(|ui| {
        let mut first = true;
        for item in items {
            if let Some(status) = status_text(app, item) {
                if !first {
                    ui.separator();
                }
                let color = if item == StatusItem::Completion
                    && matches!(
                        app.completion.engine.state(),
                        aitext_ai::CompletionState::Suggested
                    ) {
                    shell.ghost
                } else {
                    shell.muted_text
                };
                ui.label(egui::RichText::new(status).color(color));
                first = false;
            }
        }
    });
}

pub fn draw_document_toolbar(ui: &mut Ui, app: &mut AitextApp) {
    let shell = shell_colors(app.config.theme);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(
            egui::RichText::new(text(app.locale(), TextKey::DocumentTypeLabel))
                .color(shell.muted_text),
        );
        draw_language_selector(ui, app);
    });
}

pub fn language_label(locale: Locale, language: LanguageId) -> &'static str {
    let key = match language {
        LanguageId::Markdown => TextKey::LanguageMarkdown,
        LanguageId::PlainText => TextKey::LanguagePlainText,
        LanguageId::C => TextKey::LanguageC,
        LanguageId::Cpp => TextKey::LanguageCpp,
        LanguageId::CSharp => TextKey::LanguageCSharp,
        LanguageId::Python => TextKey::LanguagePython,
        LanguageId::Rust => TextKey::LanguageRust,
        LanguageId::JavaScript => TextKey::LanguageJavaScript,
        LanguageId::TypeScript => TextKey::LanguageTypeScript,
        LanguageId::Html => TextKey::LanguageHtml,
        LanguageId::Css => TextKey::LanguageCss,
        LanguageId::Json => TextKey::LanguageJson,
        LanguageId::Toml => TextKey::LanguageToml,
        LanguageId::PowerShell => TextKey::LanguagePowerShell,
        LanguageId::Batch => TextKey::LanguageBatch,
        LanguageId::Ini => TextKey::LanguageIni,
    };
    text(locale, key)
}

pub fn draw_language_selector(ui: &mut Ui, app: &mut AitextApp) {
    let Some(current) = app.workspace.current().map(|doc| doc.language()) else {
        ui.label(language_label(app.locale(), LanguageId::Markdown));
        return;
    };
    let locale = app.locale();
    let mut selected = current;
    egui::ComboBox::from_id_salt("document-language")
        .width(160.0)
        .selected_text(language_label(locale, current))
        .show_ui(ui, |ui| {
            ui.weak(text(locale, TextKey::DocumentTypeText));
            for language in [LanguageId::Markdown, LanguageId::PlainText] {
                ui.selectable_value(&mut selected, language, language_label(locale, language));
            }
            ui.separator();
            ui.weak(text(locale, TextKey::DocumentTypeProgramming));
            for language in LanguageId::ALL.iter().copied().filter(|language| {
                !matches!(language, LanguageId::Markdown | LanguageId::PlainText)
            }) {
                ui.selectable_value(&mut selected, language, language_label(locale, language));
            }
        });
    if selected != current {
        app.set_document_language(selected);
    }
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
            .map(|doc| language_label(app.locale(), doc.language()).to_string()),
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
    use super::{draw_document_toolbar, language_label, status_text};
    use crate::commands::AitextApp;
    use crate::config::{ApiProfile, StatusItem};
    use crate::i18n::{Locale, UiLanguage};
    use aitext_ai::ProviderKind;
    use aitext_core::LanguageId;

    #[test]
    fn language_labels_are_localized() {
        assert_eq!(language_label(Locale::En, LanguageId::Markdown), "Markdown");
        assert_eq!(
            language_label(Locale::ZhCn, LanguageId::PlainText),
            "纯文本"
        );
        assert_eq!(language_label(Locale::En, LanguageId::CSharp), "C#");
    }

    #[test]
    fn document_toolbar_exposes_markdown_as_the_default_file_type() {
        let mut app = AitextApp::new_for_test();
        app.workspace.new_untitled();
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(500.0, 80.0),
            )),
            ..Default::default()
        };
        let output = context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                draw_document_toolbar(ui, &mut app);
            });
        });
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) => Some(shape.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"File type"));
        assert!(labels.contains(&"Markdown"));
    }

    #[test]
    fn language_options_cover_requested_mainstream_formats() {
        for language in [
            LanguageId::PlainText,
            LanguageId::Markdown,
            LanguageId::C,
            LanguageId::Cpp,
            LanguageId::Python,
            LanguageId::CSharp,
            LanguageId::Html,
            LanguageId::Css,
        ] {
            assert!(LanguageId::ALL.contains(&language));
        }
    }

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
