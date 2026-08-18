use aitext_core::Motion;
use egui::{Event, Key, Ui};

use crate::commands::{AitextApp, Command};
use crate::painter::paint_editor;

pub fn draw_editor(ui: &mut Ui, app: &mut AitextApp) {
    handle_keys(ui, app);
    handle_text(ui, app);
    let ghost = app.ghost_text().map(ToOwned::to_owned);
    let preedit = if app.ime.composing && !app.ime.preedit.is_empty() {
        Some(app.ime.preedit.clone())
    } else {
        None
    };
    if let Some(doc) = app.workspace.current_mut() {
        paint_editor(
            ui,
            doc,
            &app.config,
            ghost.as_deref(),
            preedit.as_deref(),
        );
    } else {
        ui.centered_and_justified(|ui| ui.label("No file open"));
    }
}

fn handle_text(ui: &Ui, app: &mut AitextApp) {
    let events = ui.input(|i| i.events.clone());
    for event in events {
        if let Event::Ime(_) = &event {
            if let Event::Ime(ime) = event.clone() {
                app.apply_ime(ime);
                continue;
            }
        }
        if let Event::Text(text) = event {
            if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                continue;
            }
            if app.ime.composing {
                continue;
            }
            let readonly = app.workspace.current().map(|d| d.is_readonly()).unwrap_or(true);
            if !readonly {
                app.handle_text_input(&text);
            }
        }
    }
}

fn handle_keys(ui: &Ui, app: &mut AitextApp) {
    let mut commands = Vec::new();
    ui.input(|input| {
        if input.modifiers.ctrl || input.modifiers.command {
            if input.key_pressed(Key::N) { commands.push(Command::NewTab); }
            if input.key_pressed(Key::O) { commands.push(Command::Open); }
            if input.key_pressed(Key::S) {
                commands.push(if input.modifiers.shift { Command::SaveAs } else { Command::Save });
            }
            if input.key_pressed(Key::W) { commands.push(Command::CloseTab); }
            if input.key_pressed(Key::Tab) {
                commands.push(if input.modifiers.shift { Command::PrevTab } else { Command::NextTab });
            }
            if input.key_pressed(Key::Z) { commands.push(Command::Undo); }
            if input.key_pressed(Key::Y) { commands.push(Command::Redo); }
            if input.key_pressed(Key::F) { commands.push(Command::Find); }
            if input.key_pressed(Key::H) { commands.push(Command::Replace); }
            if input.key_pressed(Key::A) { commands.push(Command::SelectAll); }
            if input.key_pressed(Key::Comma) { commands.push(Command::Settings); }
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
        if command == Command::RejectGhost && app.find.visible && app.workspace.current().and_then(|_| Some(())).is_some() {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                app.find.visible = false;
            }
        }
        app.dispatch(command);
    }
    if let Some(doc) = app.workspace.current_mut() {
        ui.input(|input| {
            if app.ime.composing {
                return;
            }
            let extend = input.modifiers.shift;
            if input.key_pressed(Key::ArrowLeft) { doc.move_caret(Motion::Left, extend); }
            if input.key_pressed(Key::ArrowRight) { doc.move_caret(Motion::Right, extend); }
            if input.key_pressed(Key::ArrowUp) { doc.move_caret(Motion::Up, extend); }
            if input.key_pressed(Key::ArrowDown) { doc.move_caret(Motion::Down, extend); }
            if input.key_pressed(Key::Home) { doc.move_caret(Motion::Home, extend); }
            if input.key_pressed(Key::End) { doc.move_caret(Motion::End, extend); }
            if input.key_pressed(Key::PageUp) { doc.move_caret(Motion::PageUp, extend); }
            if input.key_pressed(Key::PageDown) { doc.move_caret(Motion::PageDown, extend); }
            if input.key_pressed(Key::Backspace) { doc.delete_backward(); }
            if input.key_pressed(Key::Delete) { doc.delete_forward(); }
        });
    }
    let enter = ui.input(|input| {
        input.key_pressed(Key::Enter) && !(input.modifiers.ctrl || input.modifiers.command)
    });
    if enter && !app.ime.composing {
        let readonly = app.workspace.current().map(|d| d.is_readonly()).unwrap_or(true);
        if !readonly {
            app.handle_text_input("
");
        }
    }
}
