use ainotepad_core::Document;
use eframe::egui;

use crate::commands::{AinotepadApp, Command};
use crate::config::{remember_recent, save_config};
use crate::editor_view::draw_editor;
use crate::find_bar::draw_find_bar;
use crate::i18n::{localized_document_name, text, TextKey, UiMessage};
use crate::status_bar::{draw_document_toolbar, draw_status_bar};

fn should_close_settings_for_outside_click(
    settings_was_open: bool,
    pointer_pressed: bool,
    pointer_pos: Option<egui::Pos2>,
    settings_rect: egui::Rect,
) -> bool {
    settings_was_open
        && pointer_pressed
        && pointer_pos
            .map(|pos| !settings_rect.expand(10.0).contains(pos))
            .unwrap_or(false)
}

fn settings_window_size(viewport: egui::Vec2) -> egui::Vec2 {
    egui::vec2(
        (viewport.x - 48.0).clamp(360.0, 860.0),
        (viewport.y - 96.0).clamp(300.0, 680.0),
    )
}

fn settings_window_position(viewport: egui::Vec2, window: egui::Vec2) -> egui::Pos2 {
    egui::pos2(
        ((viewport.x - window.x) * 0.5).max(12.0),
        ((viewport.y - window.y) * 0.5).max(12.0),
    )
}

fn close_icon_button(
    ui: &mut egui::Ui,
    accessible_label: &'static str,
    idle_color: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), accessible_label)
    });
    let visuals = ui.style().interact(&response);
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(3), visuals.weak_bg_fill);
    }
    let icon = rect.shrink(5.0).expand(visuals.expansion);
    let stroke = if response.hovered() || response.has_focus() {
        visuals.fg_stroke
    } else {
        egui::Stroke::new(1.2_f32, idle_color)
    };
    ui.painter()
        .line_segment([icon.left_top(), icon.right_bottom()], stroke);
    ui.painter()
        .line_segment([icon.right_top(), icon.left_bottom()], stroke);
    response.on_hover_text(accessible_label)
}

fn active_tab_rule(rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - 2.0),
        rect.right_bottom(),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseDecision {
    Save,
    Discard,
    Cancel,
}

impl eframe::App for AinotepadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let settings_was_open = self.settings_open;
        let locale = self.locale();
        if ctx.input(|input| input.viewport().close_requested())
            && (self.should_prompt_before_close() || self.close_prompt_open)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_prompt_open = true;
        }
        crate::theme::apply_visuals(ctx, self.config.theme, &self.config.custom_theme);
        let shell = crate::theme::shell_colors(self.config.theme);
        let editor = crate::theme::colors_with_custom(self.config.theme, &self.config.custom_theme);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if ctx.input(|i| {
            i.key_pressed(egui::Key::Enter) && !(i.modifiers.ctrl || i.modifiers.command)
        }) && !self.completion.composing
        {
            let settings_typing = self.settings_open;
            let find_typing = self.find.visible;
            if !settings_typing && !find_typing {
                let readonly = self
                    .workspace
                    .current()
                    .map(|d| d.is_readonly())
                    .unwrap_or(true);
                if !readonly {
                    self.handle_text_input(
                        "
",
                    );
                }
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            }
        }
        self.poll_background_workers(now_ms);
        if self.background_work_needs_repaint() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        egui::TopBottomPanel::top("menu")
            .exact_height(36.0)
            .frame(
                egui::Frame::NONE
                    .fill(shell.base)
                    .stroke(egui::Stroke::new(1.0_f32, shell.rule))
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
                    ui.menu_button(text(locale, TextKey::MenuFile), |ui| {
                        if ui.button(text(locale, TextKey::FileNew)).clicked() {
                            self.dispatch(Command::NewTab);
                            ui.close_menu();
                        }
                        if ui.button(text(locale, TextKey::FileOpen)).clicked() {
                            self.open_file();
                            ui.close_menu();
                        }
                        if ui.button(text(locale, TextKey::FileSave)).clicked() {
                            self.save_file(false);
                            ui.close_menu();
                        }
                        if ui.button(text(locale, TextKey::FileSaveAs)).clicked() {
                            self.save_file(true);
                            ui.close_menu();
                        }
                        if ui.button(text(locale, TextKey::FileCloseTab)).clicked() {
                            self.dispatch(Command::CloseTab);
                            ui.close_menu();
                        }
                        ui.separator();
                        if !self.config.recent_files.is_empty() {
                            ui.weak(text(locale, TextKey::FileRecent));
                        }
                        for path in self.config.recent_files.clone() {
                            if ui.button(&path).clicked() {
                                self.open_path(&path);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button(text(locale, TextKey::FileExit)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(text(locale, TextKey::MenuEdit), |ui| {
                        if ui.button(text(locale, TextKey::EditUndo)).clicked() {
                            self.dispatch(Command::Undo);
                        }
                        if ui.button(text(locale, TextKey::EditRedo)).clicked() {
                            self.dispatch(Command::Redo);
                        }
                        if ui.button(text(locale, TextKey::EditCut)).clicked() {
                            self.cut_selection_to_system(ui.ctx());
                        }
                        if ui.button(text(locale, TextKey::EditCopy)).clicked() {
                            self.copy_selection_to_system(ui.ctx());
                        }
                        if ui.button(text(locale, TextKey::EditPaste)).clicked() {
                            self.paste_from_system();
                        }
                        if ui.button(text(locale, TextKey::EditDelete)).clicked() {
                            self.dispatch(Command::Delete);
                        }
                        if ui.button(text(locale, TextKey::EditSelectAll)).clicked() {
                            self.dispatch(Command::SelectAll);
                        }
                        if ui.button(text(locale, TextKey::EditIndent)).clicked() {
                            self.dispatch(Command::Indent);
                        }
                    });
                    ui.menu_button(text(locale, TextKey::MenuFind), |ui| {
                        if ui.button(text(locale, TextKey::FindOpen)).clicked() {
                            self.dispatch(Command::Find);
                        }
                        if ui.button(text(locale, TextKey::FindReplace)).clicked() {
                            self.dispatch(Command::Replace);
                        }
                    });
                    if ui.button(text(locale, TextKey::MenuSettings)).clicked() {
                        self.dispatch(Command::Settings);
                    }
                    ui.menu_button(text(locale, TextKey::MenuHelp), |ui| {
                        if ui.button(text(locale, TextKey::HelpAbout)).clicked() {
                            self.about_open = true;
                        }
                        if ui.button(text(locale, TextKey::HelpShortcuts)).clicked() {
                            self.shortcuts_open = true;
                        }
                    });
                });
            });
        egui::TopBottomPanel::top("tabs")
            .exact_height(34.0)
            .frame(
                egui::Frame::NONE
                    .fill(shell.base)
                    .stroke(egui::Stroke::new(1.0_f32, shell.rule))
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.horizontal(|ui| {
                    let ids: Vec<_> = self
                        .workspace
                        .documents()
                        .map(|d| (d.id(), d.display_name(), d.is_dirty(), d.path().is_some()))
                        .collect();
                    let current = self.workspace.current_id();
                    for (id, name, dirty, has_path) in ids {
                        let name = localized_document_name(locale, &name, has_path);
                        let title = if dirty { format!("{name}*") } else { name };
                        let selected = current == Some(id);
                        let fill = if selected {
                            shell.selected
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let text_color = if selected {
                            shell.text
                        } else {
                            shell.muted_text
                        };
                        let tab = egui::Button::new(egui::RichText::new(title).color(text_color))
                            .fill(fill)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(3));
                        let tab_response = ui.add(tab);
                        if selected {
                            ui.painter().rect_filled(
                                active_tab_rule(tab_response.rect),
                                egui::CornerRadius::ZERO,
                                shell.focus,
                            );
                        }
                        if tab_response.clicked() {
                            self.workspace.set_current(id);
                        }
                        if close_icon_button(
                            ui,
                            text(locale, TextKey::CommonClose),
                            shell.muted_text,
                        )
                        .clicked()
                        {
                            self.workspace.close(id);
                        }
                    }
                });
            });
        egui::TopBottomPanel::top("document-toolbar")
            .exact_height(30.0)
            .frame(
                egui::Frame::NONE
                    .fill(shell.base)
                    .stroke(egui::Stroke::new(1.0_f32, shell.rule))
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ctx, |ui| {
                draw_document_toolbar(ui, self);
            });
        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .frame(
                egui::Frame::NONE
                    .fill(shell.base)
                    .stroke(egui::Stroke::new(1.0_f32, shell.rule))
                    .inner_margin(egui::Margin::symmetric(10, 4)),
            )
            .show(ctx, |ui| {
                draw_status_bar(ui, self);
            });
        if self.find.visible {
            egui::TopBottomPanel::top("find")
                .frame(
                    egui::Frame::NONE
                        .fill(shell.raised)
                        .stroke(egui::Stroke::new(1.0_f32, shell.rule))
                        .inner_margin(egui::Margin::symmetric(8, 5)),
                )
                .show(ctx, |ui| {
                    draw_find_bar(ui, self);
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(editor.background)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                draw_editor(ui, self);
            });
        if self.settings_open {
            let mut close = false;
            let viewport_size = ctx.screen_rect().size();
            let settings_size = settings_window_size(viewport_size);
            let settings_position = settings_window_position(viewport_size, settings_size);
            let mut window_open = true;
            let response = egui::Window::new(text(locale, TextKey::SettingsTitle))
                .open(&mut window_open)
                .collapsible(false)
                .resizable(true)
                .title_bar(true)
                .default_size(settings_size)
                .default_pos(settings_position)
                .max_width((viewport_size.x - 24.0).max(360.0))
                .max_height(settings_size.y)
                .show(ctx, |ui| {
                    crate::settings_page::draw_settings(ui, self, &mut close);
                });
            if close || !window_open {
                self.settings_open = false;
            } else if let Some(inner) = response {
                let clicked_away = ctx.input(|i| {
                    should_close_settings_for_outside_click(
                        settings_was_open,
                        i.pointer.any_pressed(),
                        i.pointer.interact_pos(),
                        inner.response.rect,
                    )
                });
                if clicked_away {
                    self.settings_open = false;
                }
            }
        }
        if self.close_prompt_open {
            let mut window_open = true;
            let mut decision = None;
            egui::Window::new(text(locale, TextKey::CloseUnsavedTitle))
                .open(&mut window_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(text(locale, TextKey::CloseUnsavedDetail));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(text(locale, TextKey::CloseSaveAll)).clicked() {
                            decision = Some(CloseDecision::Save);
                        }
                        if ui.button(text(locale, TextKey::CloseDiscard)).clicked() {
                            decision = Some(CloseDecision::Discard);
                        }
                        if ui.button(text(locale, TextKey::CloseCancel)).clicked() {
                            decision = Some(CloseDecision::Cancel);
                        }
                    });
                });
            if !window_open && decision.is_none() {
                decision = Some(CloseDecision::Cancel);
            }
            match decision {
                Some(CloseDecision::Save) => {
                    if self.save_all_documents() && !self.should_prompt_before_close() {
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Some(CloseDecision::Discard) => {
                    self.discard_all_documents();
                    self.close_prompt_open = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(CloseDecision::Cancel) => {
                    self.close_prompt_open = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
                None => {}
            }
        }
        if self.about_open {
            egui::Window::new(text(locale, TextKey::HelpAbout))
                .open(&mut self.about_open)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "{}: Ainotepad 0.1.0",
                        text(locale, TextKey::AboutVersion)
                    ));
                    ui.label(format!("{}: MIT", text(locale, TextKey::AboutLicense)));
                });
        }
        if self.shortcuts_open {
            egui::Window::new(text(locale, TextKey::ShortcutsTitle))
                .open(&mut self.shortcuts_open)
                .show(ctx, |ui| {
                    ui.label(text(locale, TextKey::ShortcutsFile));
                    ui.label(text(locale, TextKey::ShortcutsTabs));
                    ui.label(text(locale, TextKey::ShortcutsEdit));
                    ui.label(text(locale, TextKey::ShortcutsEditor));
                });
        }
    }
}

impl AinotepadApp {
    pub(crate) fn poll_background_workers(&mut self, now_ms: u64) {
        self.poll_completion(now_ms);
        self.poll_profile_workers();
    }

    pub(crate) fn background_work_needs_repaint(&self) -> bool {
        self.completion.engine.has_pending()
            || self.completion.inflight.is_some()
            || matches!(
                self.completion.engine.state(),
                ainotepad_ai::CompletionState::Requesting
            )
            || !self.profile_worker_inboxes.is_empty()
    }

    pub fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_path(&path.to_string_lossy());
        }
    }

    pub fn open_path(&mut self, path: &str) {
        match std::fs::read(path) {
            Ok(bytes) => match Document::open_bytes(&bytes) {
                Ok(mut doc) => {
                    doc.set_path(Some(path.to_string()));
                    self.workspace.add_document(doc);
                    remember_recent(&mut self.config, path);
                    let _ = save_config(&self.config);
                    self.status = None;
                }
                Err(err) => self.status = Some(UiMessage::OpenFailed(format!("{err:?}"))),
            },
            Err(err) => self.status = Some(UiMessage::OpenFailed(err.to_string())),
        }
    }

    pub fn save_file(&mut self, save_as: bool) {
        let Some(doc) = self.workspace.current() else {
            return;
        };
        let mut path = doc.path().map(ToOwned::to_owned);
        if save_as || path.is_none() {
            path = rfd::FileDialog::new()
                .save_file()
                .map(|p| p.to_string_lossy().into_owned());
        }
        let Some(path) = path else {
            return;
        };
        match doc.encode() {
            Ok(bytes) => match std::fs::write(&path, bytes) {
                Ok(()) => {
                    if let Some(doc) = self.workspace.current_mut() {
                        doc.set_path(Some(path.clone()));
                        doc.mark_clean();
                    }
                    remember_recent(&mut self.config, &path);
                    let _ = save_config(&self.config);
                    self.status = None;
                }
                Err(err) => self.status = Some(UiMessage::SaveFailed(err.to_string())),
            },
            Err(err) => self.status = Some(UiMessage::EncodeFailed(format!("{err:?}"))),
        }
    }
}

pub fn available_fonts() -> Vec<(String, String)> {
    [
        ("YaHei", "C:/Windows/Fonts/msyh.ttc"),
        ("SimHei", "C:/Windows/Fonts/simhei.ttf"),
        ("Consolas", "C:/Windows/Fonts/consola.ttf"),
        ("Cascadia Mono", "C:/Windows/Fonts/CascadiaMono.ttf"),
        ("Segoe UI", "C:/Windows/Fonts/segoeui.ttf"),
    ]
    .into_iter()
    .map(|(name, path)| (name.to_string(), path.to_string()))
    .collect()
}

pub fn font_is_available(name: &str) -> bool {
    available_fonts()
        .iter()
        .any(|(n, path)| n == name && std::path::Path::new(path).exists())
}

pub fn fallback_font_family() -> String {
    available_fonts()
        .into_iter()
        .find(|(_, path)| std::path::Path::new(path).exists())
        .map(|(name, _)| name)
        .unwrap_or_else(|| "proportional".into())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut fallback = Vec::new();
    for (name, path) in available_fonts() {
        if let Ok(data) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(data).into());
            fonts.families.insert(
                egui::FontFamily::Name(name.clone().into()),
                vec![name.clone(), "YaHei".into(), "Consolas".into()],
            );
            fallback.push(name.clone());
        }
    }
    if !fallback.is_empty() {
        fonts
            .families
            .insert(egui::FontFamily::Monospace, fallback.clone());
        fonts
            .families
            .insert(egui::FontFamily::Proportional, fallback);
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiProfile;
    use crate::settings_page::{ProfileWorkerOperation, ProfileWorkerPayload, ProfileWorkerResult};
    use ainotepad_ai::ProviderKind;

    #[test]
    fn active_tab_uses_a_two_pixel_bottom_rule() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 30.0));
        let rule = active_tab_rule(rect);
        assert_eq!(rule.height(), 2.0);
        assert_eq!(rule.bottom(), rect.bottom());
    }

    #[test]
    fn opening_settings_does_not_close_on_the_same_outside_click() {
        let settings_rect =
            egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(240.0, 180.0));

        assert!(!should_close_settings_for_outside_click(
            false,
            true,
            Some(egui::pos2(20.0, 20.0)),
            settings_rect,
        ));
    }

    #[test]
    fn settings_window_size_stays_inside_a_720p_viewport() {
        let size = settings_window_size(egui::vec2(1100.0, 697.0));

        assert_eq!(size, egui::vec2(860.0, 601.0));
        assert!(size.x < 1100.0);
        assert!(size.y < 697.0);
    }

    #[test]
    fn settings_window_starts_centered_inside_the_viewport() {
        let viewport = egui::vec2(1100.0, 697.0);
        let size = settings_window_size(viewport);

        assert_eq!(
            settings_window_position(viewport, size),
            egui::pos2(120.0, 48.0)
        );
    }

    #[test]
    fn dirty_documents_require_close_prompt() {
        let mut app = AinotepadApp::new_for_test();
        app.workspace.new_untitled();
        app.workspace.current_mut().unwrap().insert("未保存内容");
        assert!(app.should_prompt_before_close());
    }

    #[test]
    fn clean_documents_can_close_without_prompt() {
        let mut app = AinotepadApp::new_for_test();
        app.workspace.new_untitled();
        assert!(!app.should_prompt_before_close());
    }

    #[test]
    fn polling_background_workers_applies_a_current_profile_model_result() {
        let mut app = AinotepadApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));
        let profile_id = app.config.active_profile().unwrap().id.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        app.profile_worker_inboxes.push(receiver);
        sender
            .send(ProfileWorkerResult {
                profile_id,
                profile_revision: app.profile_revision,
                operation: ProfileWorkerOperation::FetchModels,
                result: ProfileWorkerPayload::Models(Ok(vec!["relay-model".into()])),
            })
            .unwrap();

        app.poll_background_workers(0);

        let profile = app.config.active_profile().unwrap();
        assert_eq!(profile.selected_model, "relay-model");
        assert_eq!(profile.known_models, vec!["relay-model"]);
    }

    #[test]
    fn background_work_needs_repaint_while_a_profile_worker_is_running() {
        let mut app = AinotepadApp::new_for_test();
        let (_sender, receiver) = std::sync::mpsc::channel::<ProfileWorkerResult>();
        app.profile_worker_inboxes.push(receiver);

        assert!(app.background_work_needs_repaint());
    }

    #[test]
    fn background_work_needs_repaint_while_completion_is_debouncing() {
        let mut app = AinotepadApp::new_for_test();
        app.workspace.new_untitled();
        let mut profile = ApiProfile::new("DeepSeek", ProviderKind::DeepSeek);
        profile.base_url = "https://api.deepseek.com/v1".into();
        profile.remember_model("deepseek-v4-flash");
        app.config.add_profile(profile);
        app.api_key = Some("sk-test".into());
        app.handle_text_input("print(");

        assert!(app.background_work_needs_repaint());
    }
}
