use ainotepad_core::Motion;
use egui::{Event, Key, Ui};

use crate::commands::{AinotepadApp, Command};
use crate::i18n::{text, TextKey};
use crate::painter::paint_editor;

pub fn draw_editor(ui: &mut Ui, app: &mut AinotepadApp) {
    if !app.settings_open && !app.find.visible {
        handle_keys(ui, app);
        handle_text(ui, app);
    }
    let no_file_open = app.tr(TextKey::NoFileOpen);
    let ghost = app.ghost_text().map(ToOwned::to_owned);
    let preedit = if app.ime.composing && !app.ime.preedit.is_empty() {
        Some(app.ime.preedit.clone())
    } else {
        None
    };
    let mut caret_changed = false;
    let mut editor_response = None;
    if let Some(doc) = app.workspace.current_mut() {
        let output = paint_editor(ui, doc, &app.config, ghost.as_deref(), preedit.as_deref());
        caret_changed = output.caret_changed;
        editor_response = Some(output.response);
    } else {
        ui.centered_and_justified(|ui| ui.label(no_file_open));
    }
    if let Some(response) = editor_response {
        response.context_menu(|ui| draw_editor_context_menu(ui, app));
    }
    if caret_changed {
        app.note_caret_changed();
    }
}

fn draw_editor_context_menu(ui: &mut Ui, app: &mut AinotepadApp) {
    let (has_selection, readonly, can_undo, can_redo) = app
        .workspace
        .current()
        .map(|doc| {
            (
                !doc.selection().is_empty(),
                doc.is_readonly(),
                doc.can_undo(),
                doc.can_redo(),
            )
        })
        .unwrap_or((false, true, false, false));
    let locale = app.locale();

    if ui
        .add_enabled(can_undo, egui::Button::new(text(locale, TextKey::EditUndo)))
        .clicked()
    {
        app.dispatch(Command::Undo);
        ui.close_menu();
    }
    if ui
        .add_enabled(can_redo, egui::Button::new(text(locale, TextKey::EditRedo)))
        .clicked()
    {
        app.dispatch(Command::Redo);
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            has_selection && !readonly,
            egui::Button::new(text(locale, TextKey::EditCut)),
        )
        .clicked()
    {
        app.cut_selection_to_system(ui.ctx());
        ui.close_menu();
    }
    if ui
        .add_enabled(
            has_selection,
            egui::Button::new(text(locale, TextKey::EditCopy)),
        )
        .clicked()
    {
        app.copy_selection_to_system(ui.ctx());
        ui.close_menu();
    }
    if ui.button(text(locale, TextKey::EditPaste)).clicked() {
        app.paste_from_system();
        ui.close_menu();
    }
    if ui
        .add_enabled(
            has_selection && !readonly,
            egui::Button::new(text(locale, TextKey::EditDelete)),
        )
        .clicked()
    {
        app.dispatch(Command::Delete);
        ui.close_menu();
    }
    ui.separator();
    if ui.button(text(locale, TextKey::EditSelectAll)).clicked() {
        app.dispatch(Command::SelectAll);
        ui.close_menu();
    }
}

fn handle_text(ui: &Ui, app: &mut AinotepadApp) {
    let events = ui.input(|i| i.events.clone());
    for event in events {
        if let Event::Ime(_) = &event {
            if let Event::Ime(ime) = event.clone() {
                app.apply_ime(ime);
                continue;
            }
        }
        match event {
            Event::Copy => app.copy_selection_to_system(ui.ctx()),
            Event::Cut => app.cut_selection_to_system(ui.ctx()),
            Event::Paste(text) => app.paste_text(&text),
            Event::Text(text) => {
                if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    continue;
                }
                if app.ime.composing {
                    continue;
                }
                let readonly = app
                    .workspace
                    .current()
                    .map(|d| d.is_readonly())
                    .unwrap_or(true);
                if !readonly {
                    app.handle_text_input(&text);
                }
            }
            _ => {}
        }
    }
}

fn handle_keys(ui: &Ui, app: &mut AinotepadApp) {
    let mut commands = Vec::new();
    ui.input(|input| {
        if input.modifiers.ctrl || input.modifiers.command {
            if input.key_pressed(Key::N) {
                commands.push(Command::NewTab);
            }
            if input.key_pressed(Key::O) {
                commands.push(Command::Open);
            }
            if input.key_pressed(Key::S) {
                commands.push(if input.modifiers.shift {
                    Command::SaveAs
                } else {
                    Command::Save
                });
            }
            if input.key_pressed(Key::W) {
                commands.push(Command::CloseTab);
            }
            if input.key_pressed(Key::Tab) {
                commands.push(if input.modifiers.shift {
                    Command::PrevTab
                } else {
                    Command::NextTab
                });
            }
            if input.key_pressed(Key::Z) {
                commands.push(Command::Undo);
            }
            if input.key_pressed(Key::Y) {
                commands.push(Command::Redo);
            }
            if input.key_pressed(Key::F) {
                commands.push(Command::Find);
            }
            if input.key_pressed(Key::H) {
                commands.push(Command::Replace);
            }
            if input.key_pressed(Key::A) {
                commands.push(Command::SelectAll);
            }
            if input.key_pressed(Key::Comma) {
                commands.push(Command::Settings);
            }
        } else if !app.ime.composing {
            if input.key_pressed(Key::Escape) {
                if app.find.visible {
                    // handled below
                }
                commands.push(Command::RejectGhost);
            }
            if input.key_pressed(Key::Tab) {
                if input.modifiers.shift {
                    commands.push(Command::Unindent);
                } else if app.completion.engine.suggestion().is_some() {
                    commands.push(Command::AcceptGhost);
                } else {
                    commands.push(Command::Indent);
                }
            }
        }
    });
    for command in commands {
        if command == Command::RejectGhost
            && app.find.visible
            && app.workspace.current().and_then(|_| Some(())).is_some()
        {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                app.find.visible = false;
            }
        }
        app.dispatch(command);
    }
    if !app.ime.composing {
        let extend = ui.input(|input| input.modifiers.shift);
        if ui.input(|input| input.key_pressed(Key::ArrowLeft)) {
            app.move_caret(Motion::Left, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowRight)) {
            app.move_caret(Motion::Right, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
            app.move_caret(Motion::Up, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
            app.move_caret(Motion::Down, extend);
        }
        if ui.input(|input| input.key_pressed(Key::Home)) {
            app.move_caret(Motion::Home, extend);
        }
        if ui.input(|input| input.key_pressed(Key::End)) {
            app.move_caret(Motion::End, extend);
        }
        if ui.input(|input| input.key_pressed(Key::PageUp)) {
            app.move_caret(Motion::PageUp, extend);
        }
        if ui.input(|input| input.key_pressed(Key::PageDown)) {
            app.move_caret(Motion::PageDown, extend);
        }
        if ui.input(|input| input.key_pressed(Key::Backspace)) {
            app.delete_backward();
        }
        if ui.input(|input| input.key_pressed(Key::Delete)) {
            app.delete_forward();
        }
    }
    let enter = ui.input(|input| {
        input.key_pressed(Key::Enter) && !(input.modifiers.ctrl || input.modifiers.command)
    });
    if enter && !app.ime.composing {
        let readonly = app
            .workspace
            .current()
            .map(|d| d.is_readonly())
            .unwrap_or(true);
        if !readonly {
            app.handle_text_input(
                "
",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_text_input_does_not_edit_the_document() {
        let mut app = AinotepadApp::new_for_test();
        app.workspace.new_untitled();
        app.settings_open = true;
        app.config.font_family = "__aitext_test_missing_font__".into();
        let context = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        input.events.push(Event::Text("DEEPSEEK".into()));

        let _ = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        assert_eq!(app.workspace.current().unwrap().text(), "");
    }

    #[test]
    fn copy_event_writes_selected_text_to_the_system_clipboard_output() {
        let mut app = AinotepadApp::new_for_test();
        app.config.font_family = "__aitext_test_missing_font__".into();
        app.workspace.new_untitled();
        let document = app.workspace.current_mut().unwrap();
        document.insert("hello");
        document.set_selection(ainotepad_core::Selection {
            anchor: 0,
            caret: 5,
        });

        let context = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        input.events.push(Event::Copy);

        let output = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        assert!(output.platform_output.commands.iter().any(|command| {
            matches!(command, egui::OutputCommand::CopyText(text) if text == "hello")
        }));
    }

    #[test]
    fn paste_event_inserts_text_from_the_system_clipboard() {
        let mut app = AinotepadApp::new_for_test();
        app.config.font_family = "__aitext_test_missing_font__".into();
        app.workspace.new_untitled();
        app.workspace.current_mut().unwrap().insert("hello");

        let context = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        input.events.push(Event::Paste(" world".into()));

        let _ = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        assert_eq!(app.workspace.current().unwrap().text(), "hello world");
    }

    #[test]
    fn right_click_opens_the_editor_context_menu() {
        let mut app = AinotepadApp::new_for_test();
        app.config.font_family = "__aitext_test_missing_font__".into();
        app.workspace.new_untitled();
        let context = egui::Context::default();
        let position = egui::pos2(80.0, 20.0);

        let mut initial = egui::RawInput::default();
        initial.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        let _ = context.run(initial, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        let mut press = egui::RawInput::default();
        press.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        press.events.push(Event::PointerMoved(position));
        press.events.push(Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Secondary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = context.run(press, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        let mut release = egui::RawInput::default();
        release.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        release.events.push(Event::PointerMoved(position));
        release.events.push(Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = context.run(release, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        let mut visible = egui::RawInput::default();
        visible.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 220.0),
        ));
        visible.events.push(Event::PointerMoved(position));
        let output = context.run(visible, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| draw_editor(ui, &mut app));
        });

        assert!(output.shapes.iter().any(|clipped| {
            matches!(
                &clipped.shape,
                egui::Shape::Text(shape) if shape.galley.job.text == "Copy"
            )
        }));
    }
}
