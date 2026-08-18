use aitext_core::Document;
use eframe::egui;

use crate::commands::{AitextApp, Command};
use crate::config::{remember_recent, save_config};
use crate::editor_view::draw_editor;
use crate::find_bar::draw_find_bar;
use crate::status_bar::draw_status_bar;

impl eframe::App for AitextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        static FONTS_INSTALLED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if !FONTS_INSTALLED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            install_fonts(ctx);
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.poll_completion(now_ms);
        if self.completion.inflight.is_some()
            || matches!(
                self.completion.engine.state(),
                aitext_ai::CompletionState::Requesting
            )
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() { self.dispatch(Command::NewTab); ui.close_menu(); }
                    if ui.button("Open...").clicked() { self.open_file(); ui.close_menu(); }
                    if ui.button("Save").clicked() { self.save_file(false); ui.close_menu(); }
                    if ui.button("Save As...").clicked() { self.save_file(true); ui.close_menu(); }
                    if ui.button("Close Tab").clicked() { self.dispatch(Command::CloseTab); ui.close_menu(); }
                    ui.separator();
                    for path in self.config.recent_files.clone() {
                        if ui.button(&path).clicked() {
                            self.open_path(&path);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() { self.dispatch(Command::Undo); }
                    if ui.button("Redo").clicked() { self.dispatch(Command::Redo); }
                    if ui.button("Cut").clicked() { self.dispatch(Command::Cut); }
                    if ui.button("Copy").clicked() { self.dispatch(Command::Copy); }
                    if ui.button("Paste").clicked() { self.dispatch(Command::Paste); }
                    if ui.button("Select All").clicked() { self.dispatch(Command::SelectAll); }
                    if ui.button("Indent").clicked() { self.dispatch(Command::Indent); }
                });
                ui.menu_button("Find", |ui| {
                    if ui.button("Find").clicked() { self.dispatch(Command::Find); }
                    if ui.button("Replace").clicked() { self.dispatch(Command::Replace); }
                });
                if ui.button("Settings").clicked() { self.dispatch(Command::Settings); }
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() { self.about_open = true; }
                    if ui.button("Keyboard shortcuts").clicked() { self.shortcuts_open = true; }
                });
            });
            ui.horizontal(|ui| {
                let ids: Vec<_> = self.workspace.documents().map(|d| (d.id(), d.display_name(), d.is_dirty())).collect();
                for (id, name, dirty) in ids {
                    let title = if dirty { format!("{name}*") } else { name };
                    if ui.selectable_label(self.workspace.current_id() == Some(id), title).clicked() {
                        self.workspace.set_current(id);
                    }
                    if ui.small_button("x").clicked() {
                        self.workspace.close(id);
                    }
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            draw_status_bar(ui, self);
        });
        if self.find.visible {
            egui::TopBottomPanel::top("find").show(ctx, |ui| {
                draw_find_bar(ui, self);
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            draw_editor(ui, self);
        });
        if self.settings_open {
            egui::Window::new("Settings").show(ctx, |ui| {
                crate::settings_page::draw_settings(ui, self);
                if ui.button("Close").clicked() {
                    self.settings_open = false;
                }
            });
        }
        if self.about_open {
            egui::Window::new("About").open(&mut self.about_open).show(ctx, |ui| {
                ui.label("Aitext 0.1.0");
                ui.label("MIT License");
            });
        }
        if self.shortcuts_open {
            egui::Window::new("Keyboard shortcuts").open(&mut self.shortcuts_open).show(ctx, |ui| {
                ui.label("Ctrl+N New  Ctrl+O Open  Ctrl+S Save  Ctrl+Shift+S Save As");
                ui.label("Ctrl+W Close tab  Ctrl+Tab Next tab");
                ui.label("Ctrl+Z Undo  Ctrl+Y Redo  Ctrl+F Find  Ctrl+H Replace");
                ui.label("Tab Indent  Shift+Tab Unindent  Esc Close find");
            });
        }
    }
}

impl AitextApp {
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_path(&path.to_string_lossy());
        }
    }

    fn open_path(&mut self, path: &str) {
        match std::fs::read(path) {
            Ok(bytes) => match Document::open_bytes(&bytes) {
                Ok(mut doc) => {
                    doc.set_path(Some(path.to_string()));
                    self.workspace.add_document(doc);
                    remember_recent(&mut self.config, path);
                    let _ = save_config(&self.config);
                    self.status.clear();
                }
                Err(err) => self.status = format!("open failed: {err:?}"),
            },
            Err(err) => self.status = format!("open failed: {err}"),
        }
    }

    fn save_file(&mut self, save_as: bool) {
        let Some(doc) = self.workspace.current() else { return; };
        let mut path = doc.path().map(ToOwned::to_owned);
        if save_as || path.is_none() {
            path = rfd::FileDialog::new().save_file().map(|p| p.to_string_lossy().into_owned());
        }
        let Some(path) = path else { return; };
        match doc.encode() {
            Ok(bytes) => match std::fs::write(&path, bytes) {
                Ok(()) => {
                    if let Some(doc) = self.workspace.current_mut() {
                        doc.set_path(Some(path.clone()));
                        doc.mark_clean();
                    }
                    remember_recent(&mut self.config, &path);
                    let _ = save_config(&self.config);
                    self.status.clear();
                }
                Err(err) => self.status = format!("save failed: {err}"),
            },
            Err(err) => self.status = format!("encode failed: {err:?}"),
        }
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for candidate in [
        "C:\\Windows\\Fonts\\consola.ttf",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
    ] {
        if let Ok(data) = std::fs::read(candidate) {
            let name = candidate.to_string();
            fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data).into());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, name);
        }
    }
    ctx.set_fonts(fonts);
}
