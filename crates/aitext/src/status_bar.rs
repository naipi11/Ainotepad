use aitext_core::{Encoding, NewlineStyle};
use egui::Ui;

use crate::commands::AitextApp;

pub fn draw_status_bar(ui: &mut Ui, app: &AitextApp) {
    ui.horizontal(|ui| {
        if let Some(doc) = app.workspace.current() {
            let (line, col) = doc.line_column();
            ui.label(format!("{line}:{col}"));
            ui.separator();
            ui.label(encoding_name(doc.encoding()));
            ui.separator();
            ui.label(match doc.newline_style() {
                NewlineStyle::Lf => "LF",
                NewlineStyle::Crlf => "CRLF",
            });
            ui.separator();
            ui.label(format!("{:?}", doc.language()));
        } else {
            ui.label("1:1");
        }
        ui.separator();
        ui.label(app.completion_label_now());
        if !app.status.is_empty() {
            ui.separator();
            ui.label(&app.status);
        }
    });
}

fn encoding_name(encoding: Encoding) -> &'static str {
    match encoding {
        Encoding::Utf8 => "UTF-8",
        Encoding::Utf8Bom => "UTF-8 BOM",
        Encoding::Utf16Le => "UTF-16 LE",
        Encoding::Utf16Be => "UTF-16 BE",
        Encoding::Gbk => "GBK",
    }
}
