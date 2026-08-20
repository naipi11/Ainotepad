use crate::config::{CustomTheme, ThemeName};
use ainotepad_core::TokenKind;
use egui::{Color32, Context, Stroke, Visuals};

pub struct ThemeColors {
    pub background: Color32,
    pub paper: Color32,
    pub text: Color32,
    pub current_line: Color32,
    pub selection: Color32,
    pub gutter: Color32,
    pub rule: Color32,
    pub chrome: Color32,
    pub chrome_text: Color32,
    pub menu: Color32,
    pub menu_hover: Color32,
    pub accent: Color32,
    pub ghost: Color32,
    pub status: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellColors {
    pub base: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub text: Color32,
    pub muted_text: Color32,
    pub rule: Color32,
    pub focus: Color32,
    pub ghost: Color32,
}

pub fn shell_colors(theme: ThemeName) -> ShellColors {
    if theme == ThemeName::HighContrast {
        return ShellColors {
            base: Color32::BLACK,
            raised: Color32::from_rgb(8, 8, 8),
            hover: Color32::from_rgb(32, 32, 32),
            selected: Color32::from_rgb(48, 48, 48),
            text: Color32::WHITE,
            muted_text: Color32::from_rgb(220, 220, 220),
            rule: Color32::from_rgb(128, 128, 128),
            focus: Color32::from_rgb(255, 208, 0),
            ghost: Color32::from_rgb(120, 255, 160),
        };
    }

    if matches!(theme, ThemeName::White | ThemeName::MacOs) {
        return ShellColors {
            base: Color32::from_rgb(247, 247, 248),
            raised: Color32::WHITE,
            hover: Color32::from_rgb(235, 239, 245),
            selected: Color32::from_rgb(226, 233, 243),
            text: Color32::from_rgb(28, 29, 32),
            muted_text: Color32::from_rgb(101, 107, 117),
            rule: Color32::from_rgb(215, 218, 224),
            focus: Color32::from_rgb(37, 99, 235),
            ghost: Color32::from_rgb(33, 140, 85),
        };
    }

    ShellColors {
        base: Color32::from_rgb(16, 17, 19),
        raised: Color32::from_rgb(23, 25, 29),
        hover: Color32::from_rgb(35, 39, 48),
        selected: Color32::from_rgb(29, 34, 43),
        text: Color32::from_rgb(231, 233, 237),
        muted_text: Color32::from_rgb(155, 161, 170),
        rule: Color32::from_rgb(48, 52, 59),
        focus: Color32::from_rgb(59, 130, 246),
        ghost: Color32::from_rgb(91, 214, 139),
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        ((a.r() as f32) * (1.0 - t) + (b.r() as f32) * t) as u8,
        ((a.g() as f32) * (1.0 - t) + (b.g() as f32) * t) as u8,
        ((a.b() as f32) * (1.0 - t) + (b.b() as f32) * t) as u8,
    )
}

pub fn colors_with_custom(theme: ThemeName, custom: &CustomTheme) -> ThemeColors {
    let mut colors = match theme {
        ThemeName::Custom => {
            let paper = rgb(custom.paper);
            let text = rgb(custom.text);
            let accent = rgb(custom.accent);
            let chrome = rgb(custom.chrome);
            ThemeColors {
                background: mix(chrome, paper, 0.15),
                paper,
                text,
                current_line: mix(paper, accent, 0.18),
                selection: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 70),
                gutter: mix(text, paper, 0.45),
                rule: mix(paper, text, 0.18),
                chrome,
                chrome_text: mix(text, paper, 0.12),
                menu: mix(paper, chrome, 0.2),
                menu_hover: mix(chrome, Color32::BLACK, 0.35),
                accent,
                ghost: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 150),
                status: mix(chrome, Color32::BLACK, 0.2),
            }
        }
        ThemeName::White => ThemeColors {
            background: Color32::from_rgb(255, 255, 255),
            paper: Color32::from_rgb(255, 255, 255),
            text: Color32::from_rgb(24, 24, 27),
            current_line: Color32::from_rgb(245, 248, 252),
            selection: Color32::from_rgba_unmultiplied(0, 120, 212, 60),
            gutter: Color32::from_rgb(118, 118, 123),
            rule: Color32::from_rgb(225, 225, 230),
            chrome: Color32::from_rgb(248, 248, 250),
            chrome_text: Color32::from_rgb(48, 48, 52),
            menu: Color32::from_rgb(255, 255, 255),
            menu_hover: Color32::from_rgb(229, 241, 252),
            accent: Color32::from_rgb(0, 120, 212),
            ghost: Color32::from_rgba_unmultiplied(105, 110, 118, 145),
            status: Color32::from_rgb(245, 245, 247),
        },
        ThemeName::BlackGreen => ThemeColors {
            background: Color32::from_rgb(0, 4, 2),
            paper: Color32::from_rgb(2, 10, 6),
            text: Color32::from_rgb(184, 255, 201),
            current_line: Color32::from_rgb(6, 28, 16),
            selection: Color32::from_rgba_unmultiplied(0, 255, 102, 60),
            gutter: Color32::from_rgb(72, 150, 96),
            rule: Color32::from_rgb(17, 75, 40),
            chrome: Color32::from_rgb(0, 7, 3),
            chrome_text: Color32::from_rgb(151, 238, 177),
            menu: Color32::from_rgb(4, 18, 10),
            menu_hover: Color32::from_rgb(8, 42, 21),
            accent: Color32::from_rgb(0, 255, 102),
            ghost: Color32::from_rgba_unmultiplied(0, 255, 102, 145),
            status: Color32::from_rgb(0, 16, 7),
        },
        ThemeName::VsCode => ThemeColors {
            background: Color32::from_rgb(30, 30, 30),
            paper: Color32::from_rgb(30, 30, 30),
            text: Color32::from_rgb(212, 212, 212),
            current_line: Color32::from_rgb(42, 45, 46),
            selection: Color32::from_rgba_unmultiplied(38, 79, 120, 150),
            gutter: Color32::from_rgb(133, 133, 133),
            rule: Color32::from_rgb(62, 62, 66),
            chrome: Color32::from_rgb(37, 37, 38),
            chrome_text: Color32::from_rgb(204, 204, 204),
            menu: Color32::from_rgb(37, 37, 38),
            menu_hover: Color32::from_rgb(9, 71, 113),
            accent: Color32::from_rgb(0, 122, 204),
            ghost: Color32::from_rgba_unmultiplied(128, 128, 128, 150),
            status: Color32::from_rgb(0, 122, 204),
        },
        ThemeName::MacOs => ThemeColors {
            background: Color32::from_rgb(236, 236, 238),
            paper: Color32::from_rgb(255, 255, 255),
            text: Color32::from_rgb(28, 28, 30),
            current_line: Color32::from_rgb(247, 247, 249),
            selection: Color32::from_rgba_unmultiplied(0, 122, 255, 58),
            gutter: Color32::from_rgb(142, 142, 147),
            rule: Color32::from_rgb(209, 209, 214),
            chrome: Color32::from_rgb(242, 242, 247),
            chrome_text: Color32::from_rgb(58, 58, 60),
            menu: Color32::from_rgb(250, 250, 252),
            menu_hover: Color32::from_rgb(220, 235, 252),
            accent: Color32::from_rgb(0, 122, 255),
            ghost: Color32::from_rgba_unmultiplied(99, 99, 102, 150),
            status: Color32::from_rgb(242, 242, 247),
        },
        ThemeName::Dark => ThemeColors {
            background: Color32::from_rgb(13, 15, 18),
            paper: Color32::from_rgb(18, 21, 25),
            text: Color32::from_rgb(225, 230, 236),
            current_line: Color32::from_rgb(28, 33, 39),
            selection: Color32::from_rgba_unmultiplied(82, 139, 255, 72),
            gutter: Color32::from_rgb(112, 122, 134),
            rule: Color32::from_rgb(46, 53, 62),
            chrome: Color32::from_rgb(14, 17, 21),
            chrome_text: Color32::from_rgb(190, 198, 208),
            menu: Color32::from_rgb(22, 26, 31),
            menu_hover: Color32::from_rgb(34, 41, 49),
            accent: Color32::from_rgb(96, 165, 250),
            ghost: Color32::from_rgba_unmultiplied(130, 145, 162, 150),
            status: Color32::from_rgb(10, 12, 15),
        },
        ThemeName::Lamp => ThemeColors {
            background: Color32::from_rgb(18, 16, 14),
            paper: Color32::from_rgb(28, 24, 20),
            text: Color32::from_rgb(236, 226, 208),
            current_line: Color32::from_rgb(42, 34, 26),
            selection: Color32::from_rgba_unmultiplied(196, 112, 48, 70),
            gutter: Color32::from_rgb(132, 112, 90),
            rule: Color32::from_rgb(58, 46, 36),
            chrome: Color32::from_rgb(22, 20, 18),
            chrome_text: Color32::from_rgb(214, 198, 174),
            menu: Color32::from_rgb(34, 28, 22),
            menu_hover: Color32::from_rgb(12, 10, 8),
            accent: Color32::from_rgb(214, 122, 52),
            ghost: Color32::from_rgba_unmultiplied(214, 122, 52, 150),
            status: Color32::from_rgb(16, 14, 12),
        },
        ThemeName::HighContrast => ThemeColors {
            background: Color32::from_rgb(0, 0, 0),
            paper: Color32::from_rgb(0, 0, 0),
            text: Color32::from_rgb(255, 255, 255),
            current_line: Color32::from_rgb(32, 32, 32),
            selection: Color32::from_rgba_unmultiplied(255, 208, 0, 80),
            gutter: Color32::from_rgb(180, 180, 180),
            rule: Color32::from_rgb(80, 80, 80),
            chrome: Color32::from_rgb(8, 8, 8),
            chrome_text: Color32::from_rgb(255, 255, 255),
            menu: Color32::from_rgb(16, 16, 16),
            menu_hover: Color32::from_rgb(48, 48, 48),
            accent: Color32::from_rgb(255, 208, 0),
            ghost: Color32::from_rgba_unmultiplied(255, 208, 0, 160),
            status: Color32::from_rgb(0, 0, 0),
        },
    };
    let shell = shell_colors(theme);
    let paper_sum = colors.paper.r() as u16 + colors.paper.g() as u16 + colors.paper.b() as u16;
    colors.ghost = if theme == ThemeName::HighContrast {
        shell.ghost
    } else if paper_sum > 500 {
        Color32::from_rgba_unmultiplied(24, 128, 72, 205)
    } else {
        Color32::from_rgba_unmultiplied(shell.ghost.r(), shell.ghost.g(), shell.ghost.b(), 190)
    };
    colors
}

pub fn colors(theme: ThemeName) -> ThemeColors {
    colors_with_custom(theme, &CustomTheme::default())
}

pub fn apply_visuals(ctx: &Context, theme: ThemeName, custom: &CustomTheme) {
    let editor = colors_with_custom(theme, custom);
    let shell = shell_colors(theme);
    let mut visuals = if matches!(theme, ThemeName::White | ThemeName::MacOs) {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.override_text_color = Some(shell.text);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, shell.muted_text);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, shell.text);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, shell.text);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, shell.text);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, shell.text);
    visuals.widgets.inactive.bg_fill = shell.raised;
    visuals.widgets.hovered.bg_fill = shell.hover;
    visuals.widgets.active.bg_fill = shell.selected;
    visuals.widgets.open.bg_fill = shell.selected;
    visuals.widgets.inactive.weak_bg_fill = shell.raised;
    visuals.widgets.hovered.weak_bg_fill = shell.hover;
    visuals.widgets.active.weak_bg_fill = shell.selected;
    visuals.widgets.open.weak_bg_fill = shell.selected;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, shell.rule);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, shell.focus);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, shell.focus);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, shell.focus);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(3);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(3);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(3);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(3);
    visuals.widgets.hovered.expansion = 1.0;
    visuals.widgets.active.expansion = 1.0;
    visuals.panel_fill = shell.base;
    visuals.window_fill = shell.raised;
    visuals.extreme_bg_color = editor.background;
    visuals.faint_bg_color = editor.paper;
    visuals.selection.bg_fill = editor.selection;
    visuals.hyperlink_color = shell.focus;
    visuals.window_stroke = Stroke::new(1.0_f32, shell.rule);
    ctx.set_visuals(visuals);
}

pub fn token_color(theme: ThemeName, kind: TokenKind) -> Color32 {
    token_color_with_custom(theme, &CustomTheme::default(), kind)
}

pub fn token_color_with_custom(theme: ThemeName, custom: &CustomTheme, kind: TokenKind) -> Color32 {
    let c = colors_with_custom(theme, custom);
    match theme {
        ThemeName::HighContrast => match kind {
            TokenKind::Comment => Color32::from_rgb(160, 160, 160),
            TokenKind::String => Color32::from_rgb(120, 220, 120),
            TokenKind::Number => Color32::from_rgb(255, 208, 0),
            TokenKind::Keyword | TokenKind::Control => Color32::from_rgb(120, 180, 255),
            TokenKind::Type => Color32::from_rgb(80, 220, 220),
            TokenKind::Function => Color32::from_rgb(220, 220, 120),
            TokenKind::Punct => Color32::from_rgb(220, 220, 220),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(255, 255, 255),
        },
        ThemeName::VsCode => match kind {
            TokenKind::Comment => Color32::from_rgb(106, 153, 85),
            TokenKind::String => Color32::from_rgb(206, 145, 120),
            TokenKind::Number => Color32::from_rgb(181, 206, 168),
            TokenKind::Keyword => Color32::from_rgb(86, 156, 214),
            TokenKind::Control => Color32::from_rgb(197, 134, 192),
            TokenKind::Type => Color32::from_rgb(78, 201, 176),
            TokenKind::Function => Color32::from_rgb(220, 220, 170),
            TokenKind::Punct => Color32::from_rgb(212, 212, 212),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(212, 212, 212),
        },
        ThemeName::BlackGreen => match kind {
            TokenKind::Comment => Color32::from_rgb(75, 139, 87),
            TokenKind::String => Color32::from_rgb(141, 229, 143),
            TokenKind::Number => Color32::from_rgb(215, 255, 135),
            TokenKind::Keyword => Color32::from_rgb(74, 255, 156),
            TokenKind::Control => Color32::from_rgb(0, 208, 132),
            TokenKind::Type => Color32::from_rgb(114, 241, 184),
            TokenKind::Function => Color32::from_rgb(180, 248, 200),
            TokenKind::Punct => Color32::from_rgb(139, 207, 157),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(184, 255, 201),
        },
        ThemeName::MacOs => match kind {
            TokenKind::Comment => Color32::from_rgb(93, 108, 121),
            TokenKind::String => Color32::from_rgb(196, 26, 22),
            TokenKind::Number => Color32::from_rgb(39, 42, 216),
            TokenKind::Keyword | TokenKind::Control => Color32::from_rgb(173, 61, 164),
            TokenKind::Type => Color32::from_rgb(11, 79, 121),
            TokenKind::Function => Color32::from_rgb(50, 109, 116),
            TokenKind::Punct => Color32::from_rgb(57, 58, 61),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(28, 28, 30),
        },
        ThemeName::Dark => match kind {
            TokenKind::Comment => Color32::from_rgb(127, 140, 152),
            TokenKind::String => Color32::from_rgb(168, 204, 140),
            TokenKind::Number => Color32::from_rgb(224, 168, 107),
            TokenKind::Keyword => Color32::from_rgb(130, 170, 255),
            TokenKind::Control => Color32::from_rgb(199, 146, 234),
            TokenKind::Type => Color32::from_rgb(137, 221, 255),
            TokenKind::Function => Color32::from_rgb(255, 199, 119),
            TokenKind::Punct => Color32::from_rgb(214, 222, 235),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(225, 230, 236),
        },
        ThemeName::White => match kind {
            TokenKind::Comment => Color32::from_rgb(0, 128, 0),
            TokenKind::String => Color32::from_rgb(163, 21, 21),
            TokenKind::Number => Color32::from_rgb(9, 134, 88),
            TokenKind::Keyword => Color32::from_rgb(0, 0, 255),
            TokenKind::Control => Color32::from_rgb(175, 0, 219),
            TokenKind::Type => Color32::from_rgb(38, 127, 153),
            TokenKind::Function => Color32::from_rgb(121, 94, 38),
            TokenKind::Punct => Color32::from_rgb(57, 58, 61),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(30, 30, 30),
        },
        ThemeName::Lamp => match kind {
            TokenKind::Comment => Color32::from_rgb(106, 153, 85),
            TokenKind::String => Color32::from_rgb(206, 145, 120),
            TokenKind::Number => Color32::from_rgb(181, 206, 168),
            TokenKind::Keyword => Color32::from_rgb(86, 156, 214),
            TokenKind::Control => Color32::from_rgb(197, 134, 192),
            TokenKind::Type => Color32::from_rgb(78, 201, 176),
            TokenKind::Function => Color32::from_rgb(220, 220, 170),
            TokenKind::Punct => Color32::from_rgb(212, 212, 212),
            TokenKind::Text | TokenKind::Ident => Color32::from_rgb(212, 212, 212),
        },
        ThemeName::Custom => {
            let dark = (c.paper.r() as u16 + c.paper.g() as u16 + c.paper.b() as u16) < 384;
            if dark {
                match kind {
                    TokenKind::Comment => Color32::from_rgb(106, 153, 85),
                    TokenKind::String => Color32::from_rgb(206, 145, 120),
                    TokenKind::Number => Color32::from_rgb(181, 206, 168),
                    TokenKind::Keyword => Color32::from_rgb(86, 156, 214),
                    TokenKind::Control => Color32::from_rgb(197, 134, 192),
                    TokenKind::Type => Color32::from_rgb(78, 201, 176),
                    TokenKind::Function => Color32::from_rgb(220, 220, 170),
                    TokenKind::Punct => Color32::from_rgb(212, 212, 212),
                    TokenKind::Text | TokenKind::Ident => Color32::from_rgb(212, 212, 212),
                }
            } else {
                match kind {
                    TokenKind::Comment => Color32::from_rgb(0, 128, 0),
                    TokenKind::String => Color32::from_rgb(163, 21, 21),
                    TokenKind::Number => Color32::from_rgb(9, 134, 88),
                    TokenKind::Keyword => Color32::from_rgb(0, 0, 255),
                    TokenKind::Control => Color32::from_rgb(175, 0, 219),
                    TokenKind::Type => Color32::from_rgb(38, 127, 153),
                    TokenKind::Function => Color32::from_rgb(121, 94, 38),
                    TokenKind::Punct => Color32::from_rgb(57, 58, 61),
                    TokenKind::Text | TokenKind::Ident => Color32::from_rgb(30, 30, 30),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast_ratio(a: Color32, b: Color32) -> f32 {
        fn luminance(color: Color32) -> f32 {
            fn channel(value: u8) -> f32 {
                let value = value as f32 / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
        }

        let lighter = luminance(a).max(luminance(b));
        let darker = luminance(a).min(luminance(b));
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn dark_editor_themes_share_the_dark_paper_cut_shell() {
        let dark = shell_colors(ThemeName::Dark);
        for theme in [
            ThemeName::BlackGreen,
            ThemeName::VsCode,
            ThemeName::Dark,
            ThemeName::Lamp,
            ThemeName::Custom,
        ] {
            let shell = shell_colors(theme);
            assert_eq!(shell.base, dark.base);
            assert_eq!(shell.raised, dark.raised);
            assert_eq!(shell.focus, dark.focus);
            assert_eq!(shell.ghost, dark.ghost);
        }
    }

    #[test]
    fn light_editor_themes_use_a_light_shell() {
        for theme in [ThemeName::White, ThemeName::MacOs] {
            let shell = shell_colors(theme);
            assert_eq!(shell.base, Color32::from_rgb(247, 247, 248));
            assert_eq!(shell.raised, Color32::WHITE);
            assert_eq!(shell.text, Color32::from_rgb(28, 29, 32));
            assert_eq!(shell.rule, Color32::from_rgb(215, 218, 224));
        }
        assert_ne!(
            shell_colors(ThemeName::White).base,
            shell_colors(ThemeName::Dark).base
        );
    }

    #[test]
    fn dark_paper_cut_semantics_use_blue_green_and_neutral_rules() {
        let shell = shell_colors(ThemeName::Dark);
        assert_eq!(shell.base, Color32::from_rgb(16, 17, 19));
        assert_eq!(shell.raised, Color32::from_rgb(23, 25, 29));
        assert_eq!(shell.focus, Color32::from_rgb(59, 130, 246));
        assert_eq!(shell.ghost, Color32::from_rgb(91, 214, 139));
        assert_eq!(shell.rule, Color32::from_rgb(48, 52, 59));
    }

    #[test]
    fn high_contrast_overrides_decorative_shell_tokens() {
        let shell = shell_colors(ThemeName::HighContrast);
        assert_eq!(shell.base, Color32::BLACK);
        assert_eq!(shell.text, Color32::WHITE);
        assert_ne!(shell.focus, shell.base);
    }

    #[test]
    fn shell_text_and_theme_adjusted_ghosts_clear_contrast_floors() {
        let shell = shell_colors(ThemeName::White);
        assert!(contrast_ratio(shell.text, shell.base) >= 4.5);
        assert!(contrast_ratio(shell.muted_text, shell.base) >= 4.5);
        assert!(contrast_ratio(shell.focus, shell.base) >= 3.0);

        for theme in [
            ThemeName::White,
            ThemeName::MacOs,
            ThemeName::Dark,
            ThemeName::Lamp,
        ] {
            let colors = colors(theme);
            assert!(contrast_ratio(colors.ghost, colors.paper) >= 4.5);
        }
    }

    #[test]
    fn approved_theme_palettes_are_distinct_and_match_their_visual_intent() {
        let white = colors(ThemeName::White);
        let black_green = colors(ThemeName::BlackGreen);
        let vscode = colors(ThemeName::VsCode);
        let macos = colors(ThemeName::MacOs);
        let dark = colors(ThemeName::Dark);
        let lamp = colors(ThemeName::Lamp);

        assert_eq!(white.paper, Color32::from_rgb(255, 255, 255));
        assert_eq!(white.text, Color32::from_rgb(24, 24, 27));
        assert_eq!(black_green.paper, Color32::from_rgb(2, 10, 6));
        assert_eq!(black_green.accent, Color32::from_rgb(0, 255, 102));
        assert_eq!(vscode.paper, Color32::from_rgb(30, 30, 30));
        assert_eq!(vscode.chrome, Color32::from_rgb(37, 37, 38));
        assert_eq!(vscode.accent, Color32::from_rgb(0, 122, 204));
        assert_eq!(macos.paper, Color32::from_rgb(255, 255, 255));
        assert_eq!(macos.chrome, Color32::from_rgb(242, 242, 247));
        assert_eq!(macos.accent, Color32::from_rgb(0, 122, 255));
        assert_eq!(dark.paper, Color32::from_rgb(18, 21, 25));
        assert_ne!(dark.paper, lamp.paper);
        assert_ne!(white.chrome, macos.chrome);
        assert_ne!(black_green.paper, vscode.paper);
    }

    #[test]
    fn named_presets_have_theme_specific_syntax_colors() {
        assert_eq!(
            token_color(ThemeName::VsCode, TokenKind::Keyword),
            Color32::from_rgb(86, 156, 214)
        );
        assert_eq!(
            token_color(ThemeName::BlackGreen, TokenKind::Keyword),
            Color32::from_rgb(74, 255, 156)
        );
        assert_eq!(
            token_color(ThemeName::MacOs, TokenKind::Keyword),
            Color32::from_rgb(173, 61, 164)
        );
        assert_eq!(
            token_color(ThemeName::Dark, TokenKind::Keyword),
            Color32::from_rgb(130, 170, 255)
        );
        assert_eq!(
            token_color(ThemeName::White, TokenKind::Keyword),
            Color32::from_rgb(0, 0, 255)
        );
    }
}
