#![windows_subsystem = "windows"]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("Aitext"),
        ..Default::default()
    };
    eframe::run_native(
        "Aitext",
        options,
        Box::new(|cc| Ok(Box::new(aitext::AitextApp::new(cc)))),
    )
}
