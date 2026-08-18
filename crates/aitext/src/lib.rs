#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub mod app;
pub mod commands;
pub mod completion;
pub mod config;
pub mod editor_view;
pub mod find_bar;
pub mod ime;
pub mod painter;
pub mod secrets;
pub mod settings_page;
pub mod status_bar;
pub mod theme;

pub use commands::AitextApp;
