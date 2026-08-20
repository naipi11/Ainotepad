use ainotepad_core::Direction;
use egui::Ui;

use crate::commands::AinotepadApp;
use crate::i18n::{find_match_count, text, TextKey};

pub fn draw_find_bar(ui: &mut Ui, app: &mut AinotepadApp) {
    if !app.find.visible {
        return;
    }
    let locale = app.locale();
    ui.horizontal(|ui| {
        ui.label(text(locale, TextKey::FindOpen));
        let changed = ui.text_edit_singleline(&mut app.find.query.text).changed();
        if ui.button(text(locale, TextKey::FindNext)).clicked() {
            app.find_step(Direction::Forward);
        }
        if ui.button(text(locale, TextKey::FindPrevious)).clicked() {
            app.find_step(Direction::Backward);
        }
        ui.checkbox(
            &mut app.find.query.match_case,
            text(locale, TextKey::FindMatchCase),
        );
        ui.checkbox(
            &mut app.find.query.whole_word,
            text(locale, TextKey::FindWholeWord),
        );
        if app.find.replace_visible {
            ui.label(text(locale, TextKey::FindReplace));
            ui.text_edit_singleline(&mut app.find.replacement);
            if ui.button(text(locale, TextKey::FindReplaceOne)).clicked() {
                if let Some(doc) = app.workspace.current_mut() {
                    doc.replace_current(&app.find.query, &app.find.replacement);
                }
                app.refresh_find();
            }
            if ui.button(text(locale, TextKey::FindReplaceAll)).clicked() {
                if let Some(doc) = app.workspace.current_mut() {
                    doc.replace_all(&app.find.query, &app.find.replacement);
                }
                app.refresh_find();
            }
        }
        let n = app.find.matches.len();
        let cur = if n == 0 { 0 } else { app.find.current + 1 };
        ui.label(find_match_count(locale, cur, n));
        if changed {
            app.refresh_find();
        }
    });
}
