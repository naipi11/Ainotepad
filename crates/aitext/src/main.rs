#![windows_subsystem = "windows"]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("Aitext")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Aitext",
        options,
        Box::new(|cc| Ok(Box::new(aitext::AitextApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/aitext-icon.png");
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let img = img.to_rgba8();
            egui::IconData {
                width: img.width(),
                height: img.height(),
                rgba: img.into_raw(),
            }
        }
        Err(_) => egui::IconData {
            rgba: vec![214, 122, 52, 255],
            width: 1,
            height: 1,
        },
    }
}
