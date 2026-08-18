use crate::config::ThemeName;
use aitext_core::TokenKind;
use egui::Color32;

pub struct ThemeColors {
    pub background: Color32,
    pub text: Color32,
    pub current_line: Color32,
    pub selection: Color32,
    pub gutter: Color32,
    pub status: Color32,
}

pub fn colors(theme: ThemeName) -> ThemeColors {
    match theme {
        ThemeName::Dark => ThemeColors {
            background: Color32::from_rgb(18, 18, 18),
            text: Color32::from_rgb(230, 230, 230),
            current_line: Color32::from_rgb(32, 32, 32),
            selection: Color32::from_rgba_unmultiplied(60, 90, 160, 120),
            gutter: Color32::from_rgb(120, 120, 120),
            status: Color32::from_rgb(40, 40, 40),
        },
        ThemeName::Light => ThemeColors {
            background: Color32::from_rgb(250, 250, 250),
            text: Color32::from_rgb(20, 20, 20),
            current_line: Color32::from_rgb(235, 235, 235),
            selection: Color32::from_rgba_unmultiplied(170, 200, 255, 140),
            gutter: Color32::from_rgb(90, 90, 90),
            status: Color32::from_rgb(230, 230, 230),
        },
    }
}

pub fn token_color(theme: ThemeName, kind: TokenKind) -> Color32 {
    let dark = matches!(theme, ThemeName::Dark);
    match kind {
        TokenKind::Text | TokenKind::Ident => {
            if dark { Color32::from_rgb(220, 220, 220) } else { Color32::from_rgb(30, 30, 30) }
        }
        TokenKind::Comment => Color32::from_rgb(106, 153, 85),
        TokenKind::String => Color32::from_rgb(206, 145, 120),
        TokenKind::Number => Color32::from_rgb(181, 206, 168),
        TokenKind::Keyword => Color32::from_rgb(86, 156, 214),
        TokenKind::Punct => {
            if dark { Color32::from_rgb(200, 200, 200) } else { Color32::from_rgb(60, 60, 60) }
        }
    }
}
