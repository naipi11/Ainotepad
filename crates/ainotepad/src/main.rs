#![windows_subsystem = "windows"]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_title("Ainotepad")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Ainotepad",
        options,
        Box::new(|cc| Ok(Box::new(ainotepad::AinotepadApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/ainotepad-icon.png");
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
