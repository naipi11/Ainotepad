use aitext_core::Direction;
use egui::Ui;

use crate::commands::AitextApp;

pub fn draw_find_bar(ui: &mut Ui, app: &mut AitextApp) {
    if !app.find.visible {
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Find");
        let changed = ui.text_edit_singleline(&mut app.find.query.text).changed();
        if ui.button("Next").clicked() {
            app.find_step(Direction::Forward);
        }
        if ui.button("Prev").clicked() {
            app.find_step(Direction::Backward);
        }
        ui.checkbox(&mut app.find.query.match_case, "Case");
        ui.checkbox(&mut app.find.query.whole_word, "Word");
        if app.find.replace_visible {
            ui.label("Replace");
            ui.text_edit_singleline(&mut app.find.replacement);
            if ui.button("Replace").clicked() {
                if let Some(doc) = app.workspace.current_mut() {
                    doc.replace_current(&app.find.query, &app.find.replacement);
                }
                app.refresh_find();
            }
            if ui.button("All").clicked() {
                if let Some(doc) = app.workspace.current_mut() {
                    doc.replace_all(&app.find.query, &app.find.replacement);
                }
                app.refresh_find();
            }
        }
        let n = app.find.matches.len();
        let cur = if n == 0 { 0 } else { app.find.current + 1 };
        ui.label(format!("{cur} of {n}"));
        if changed {
            app.refresh_find();
        }
    });
}
