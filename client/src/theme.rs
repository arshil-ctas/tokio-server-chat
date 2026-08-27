use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// All the colors chattui needs to paint itself. Every place in `ui.rs` that
/// used to reach for a hardcoded `Color::...` now reaches into a `Theme`
/// instead, so swapping the theme repaints the whole app.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // chrome
    pub bg: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub accent_alt: Color,

    // header pills
    pub brand_fg: Color,
    pub brand_bg: Color,
    pub me_fg: Color,
    pub me_bg: Color,

    // sidebar
    pub channel: Color,
    pub dm: Color,
    pub thread: Color,
    pub unread: Color,
    pub active_fg: Color,
    pub active_bg: Color,

    // messages
    pub self_nick: Color,
    pub self_body: Color,
    pub mention_fg: Color,
    pub mention_bg: Color,
    pub system: Color,
    pub timestamp: Color,
    pub msg_num: Color,

    // nick color palette (hash-picked per user)
    pub nick_palette: Vec<Color>,

    // syntax highlighting
    pub code_bg: Color,
    pub syn_keyword: Color,
    pub syn_string: Color,
    pub syn_number: Color,
    pub syn_comment: Color,
    pub syn_type: Color,
    pub syn_fn: Color,
    pub syn_default: Color,

    // status / gauges
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub presence: Color,
}

impl Theme {
    pub fn by_name(name: &str) -> Theme {
        match name.to_ascii_lowercase().as_str() {
            "dracula" => Theme::dracula(),
            "nord" => Theme::nord(),
            "gruvbox" => Theme::gruvbox(),
            "solarized" | "solarized-dark" => Theme::solarized_dark(),
            "light" | "solarized-light" => Theme::light(),
            _ => Theme::midnight(),
        }
    }

    pub fn names() -> &'static [&'static str] {
        &["midnight", "dracula", "nord", "gruvbox", "solarized", "light"]
    }

    pub fn midnight() -> Theme {
        Theme {
            name: "midnight".into(),
            bg: Color::Rgb(24, 22, 33),
            border: Color::DarkGray,
            border_focus: Color::Cyan,
            text: Color::White,
            text_dim: Color::DarkGray,
            accent: Color::Cyan,
            accent_alt: Color::Magenta,
            brand_fg: Color::Black,
            brand_bg: Color::Cyan,
            me_fg: Color::Black,
            me_bg: Color::Green,
            channel: Color::Green,
            dm: Color::LightBlue,
            thread: Color::Magenta,
            unread: Color::Yellow,
            active_fg: Color::Black,
            active_bg: Color::Cyan,
            self_nick: Color::Cyan,
            self_body: Color::White,
            mention_fg: Color::Black,
            mention_bg: Color::Yellow,
            system: Color::DarkGray,
            timestamp: Color::DarkGray,
            msg_num: Color::DarkGray,
            nick_palette: vec![
                Color::LightRed,
                Color::LightGreen,
                Color::LightYellow,
                Color::LightBlue,
                Color::LightMagenta,
                Color::LightCyan,
                Color::Green,
                Color::Cyan,
            ],
            code_bg: Color::Rgb(40, 38, 50),
            syn_keyword: Color::Magenta,
            syn_string: Color::Green,
            syn_number: Color::LightYellow,
            syn_comment: Color::DarkGray,
            syn_type: Color::Cyan,
            syn_fn: Color::LightBlue,
            syn_default: Color::White,
            ok: Color::Green,
            warn: Color::Yellow,
            err: Color::LightRed,
            presence: Color::Green,
        }
    }

    pub fn dracula() -> Theme {
        Theme {
            name: "dracula".into(),
            bg: Color::Rgb(40, 42, 54),
            border: Color::Rgb(98, 114, 164),
            border_focus: Color::Rgb(189, 147, 249),
            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(98, 114, 164),
            accent: Color::Rgb(189, 147, 249),
            accent_alt: Color::Rgb(255, 121, 198),
            brand_fg: Color::Rgb(40, 42, 54),
            brand_bg: Color::Rgb(189, 147, 249),
            me_fg: Color::Rgb(40, 42, 54),
            me_bg: Color::Rgb(80, 250, 123),
            channel: Color::Rgb(80, 250, 123),
            dm: Color::Rgb(139, 233, 253),
            thread: Color::Rgb(255, 121, 198),
            unread: Color::Rgb(241, 250, 140),
            active_fg: Color::Rgb(40, 42, 54),
            active_bg: Color::Rgb(189, 147, 249),
            self_nick: Color::Rgb(139, 233, 253),
            self_body: Color::Rgb(248, 248, 242),
            mention_fg: Color::Rgb(40, 42, 54),
            mention_bg: Color::Rgb(241, 250, 140),
            system: Color::Rgb(98, 114, 164),
            timestamp: Color::Rgb(98, 114, 164),
            msg_num: Color::Rgb(98, 114, 164),
            nick_palette: vec![
                Color::Rgb(255, 85, 85),
                Color::Rgb(80, 250, 123),
                Color::Rgb(241, 250, 140),
                Color::Rgb(139, 233, 253),
                Color::Rgb(255, 121, 198),
                Color::Rgb(189, 147, 249),
                Color::Rgb(255, 184, 108),
                Color::Rgb(139, 233, 253),
            ],
            code_bg: Color::Rgb(30, 31, 41),
            syn_keyword: Color::Rgb(255, 121, 198),
            syn_string: Color::Rgb(241, 250, 140),
            syn_number: Color::Rgb(189, 147, 249),
            syn_comment: Color::Rgb(98, 114, 164),
            syn_type: Color::Rgb(139, 233, 253),
            syn_fn: Color::Rgb(80, 250, 123),
            syn_default: Color::Rgb(248, 248, 242),
            ok: Color::Rgb(80, 250, 123),
            warn: Color::Rgb(241, 250, 140),
            err: Color::Rgb(255, 85, 85),
            presence: Color::Rgb(80, 250, 123),
        }
    }

    pub fn nord() -> Theme {
        Theme {
            name: "nord".into(),
            bg: Color::Rgb(46, 52, 64),
            border: Color::Rgb(76, 86, 106),
            border_focus: Color::Rgb(136, 192, 208),
            text: Color::Rgb(216, 222, 233),
            text_dim: Color::Rgb(97, 110, 136),
            accent: Color::Rgb(136, 192, 208),
            accent_alt: Color::Rgb(180, 142, 173),
            brand_fg: Color::Rgb(46, 52, 64),
            brand_bg: Color::Rgb(136, 192, 208),
            me_fg: Color::Rgb(46, 52, 64),
            me_bg: Color::Rgb(163, 190, 140),
            channel: Color::Rgb(163, 190, 140),
            dm: Color::Rgb(129, 161, 193),
            thread: Color::Rgb(180, 142, 173),
            unread: Color::Rgb(235, 203, 139),
            active_fg: Color::Rgb(46, 52, 64),
            active_bg: Color::Rgb(136, 192, 208),
            self_nick: Color::Rgb(136, 192, 208),
            self_body: Color::Rgb(216, 222, 233),
            mention_fg: Color::Rgb(46, 52, 64),
            mention_bg: Color::Rgb(235, 203, 139),
            system: Color::Rgb(97, 110, 136),
            timestamp: Color::Rgb(97, 110, 136),
            msg_num: Color::Rgb(97, 110, 136),
            nick_palette: vec![
                Color::Rgb(191, 97, 106),
                Color::Rgb(163, 190, 140),
                Color::Rgb(235, 203, 139),
                Color::Rgb(129, 161, 193),
                Color::Rgb(180, 142, 173),
                Color::Rgb(136, 192, 208),
                Color::Rgb(208, 135, 112),
                Color::Rgb(143, 188, 187),
            ],
            code_bg: Color::Rgb(59, 66, 82),
            syn_keyword: Color::Rgb(180, 142, 173),
            syn_string: Color::Rgb(163, 190, 140),
            syn_number: Color::Rgb(208, 135, 112),
            syn_comment: Color::Rgb(97, 110, 136),
            syn_type: Color::Rgb(143, 188, 187),
            syn_fn: Color::Rgb(136, 192, 208),
            syn_default: Color::Rgb(216, 222, 233),
            ok: Color::Rgb(163, 190, 140),
            warn: Color::Rgb(235, 203, 139),
            err: Color::Rgb(191, 97, 106),
            presence: Color::Rgb(163, 190, 140),
        }
    }

    pub fn gruvbox() -> Theme {
        Theme {
            name: "gruvbox".into(),
            bg: Color::Rgb(40, 40, 40),
            border: Color::Rgb(102, 92, 84),
            border_focus: Color::Rgb(250, 189, 47),
            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(146, 131, 116),
            accent: Color::Rgb(250, 189, 47),
            accent_alt: Color::Rgb(211, 134, 155),
            brand_fg: Color::Rgb(40, 40, 40),
            brand_bg: Color::Rgb(250, 189, 47),
            me_fg: Color::Rgb(40, 40, 40),
            me_bg: Color::Rgb(184, 187, 38),
            channel: Color::Rgb(184, 187, 38),
            dm: Color::Rgb(131, 165, 152),
            thread: Color::Rgb(211, 134, 155),
            unread: Color::Rgb(250, 189, 47),
            active_fg: Color::Rgb(40, 40, 40),
            active_bg: Color::Rgb(250, 189, 47),
            self_nick: Color::Rgb(131, 165, 152),
            self_body: Color::Rgb(235, 219, 178),
            mention_fg: Color::Rgb(40, 40, 40),
            mention_bg: Color::Rgb(250, 189, 47),
            system: Color::Rgb(146, 131, 116),
            timestamp: Color::Rgb(146, 131, 116),
            msg_num: Color::Rgb(146, 131, 116),
            nick_palette: vec![
                Color::Rgb(251, 73, 52),
                Color::Rgb(184, 187, 38),
                Color::Rgb(250, 189, 47),
                Color::Rgb(131, 165, 152),
                Color::Rgb(211, 134, 155),
                Color::Rgb(142, 192, 124),
                Color::Rgb(254, 128, 25),
                Color::Rgb(105, 174, 175),
            ],
            code_bg: Color::Rgb(60, 56, 54),
            syn_keyword: Color::Rgb(251, 73, 52),
            syn_string: Color::Rgb(184, 187, 38),
            syn_number: Color::Rgb(211, 134, 155),
            syn_comment: Color::Rgb(146, 131, 116),
            syn_type: Color::Rgb(250, 189, 47),
            syn_fn: Color::Rgb(142, 192, 124),
            syn_default: Color::Rgb(235, 219, 178),
            ok: Color::Rgb(184, 187, 38),
            warn: Color::Rgb(250, 189, 47),
            err: Color::Rgb(251, 73, 52),
            presence: Color::Rgb(184, 187, 38),
        }
    }

    pub fn solarized_dark() -> Theme {
        Theme {
            name: "solarized".into(),
            bg: Color::Rgb(0, 43, 54),
            border: Color::Rgb(88, 110, 117),
            border_focus: Color::Rgb(38, 139, 210),
            text: Color::Rgb(147, 161, 161),
            text_dim: Color::Rgb(88, 110, 117),
            accent: Color::Rgb(38, 139, 210),
            accent_alt: Color::Rgb(211, 54, 130),
            brand_fg: Color::Rgb(0, 43, 54),
            brand_bg: Color::Rgb(38, 139, 210),
            me_fg: Color::Rgb(0, 43, 54),
            me_bg: Color::Rgb(133, 153, 0),
            channel: Color::Rgb(133, 153, 0),
            dm: Color::Rgb(42, 161, 152),
            thread: Color::Rgb(211, 54, 130),
            unread: Color::Rgb(181, 137, 0),
            active_fg: Color::Rgb(0, 43, 54),
            active_bg: Color::Rgb(38, 139, 210),
            self_nick: Color::Rgb(42, 161, 152),
            self_body: Color::Rgb(147, 161, 161),
            mention_fg: Color::Rgb(0, 43, 54),
            mention_bg: Color::Rgb(181, 137, 0),
            system: Color::Rgb(88, 110, 117),
            timestamp: Color::Rgb(88, 110, 117),
            msg_num: Color::Rgb(88, 110, 117),
            nick_palette: vec![
                Color::Rgb(220, 50, 47),
                Color::Rgb(133, 153, 0),
                Color::Rgb(181, 137, 0),
                Color::Rgb(38, 139, 210),
                Color::Rgb(211, 54, 130),
                Color::Rgb(42, 161, 152),
                Color::Rgb(203, 75, 22),
                Color::Rgb(108, 113, 196),
            ],
            code_bg: Color::Rgb(7, 54, 66),
            syn_keyword: Color::Rgb(133, 153, 0),
            syn_string: Color::Rgb(42, 161, 152),
            syn_number: Color::Rgb(211, 54, 130),
            syn_comment: Color::Rgb(88, 110, 117),
            syn_type: Color::Rgb(181, 137, 0),
            syn_fn: Color::Rgb(38, 139, 210),
            syn_default: Color::Rgb(147, 161, 161),
            ok: Color::Rgb(133, 153, 0),
            warn: Color::Rgb(181, 137, 0),
            err: Color::Rgb(220, 50, 47),
            presence: Color::Rgb(133, 153, 0),
        }
    }

    pub fn light() -> Theme {
        Theme {
            name: "light".into(),
            bg: Color::Rgb(253, 246, 227),
            border: Color::Rgb(147, 161, 161),
            border_focus: Color::Rgb(38, 139, 210),
            text: Color::Rgb(7, 54, 66),
            text_dim: Color::Rgb(147, 161, 161),
            accent: Color::Rgb(38, 139, 210),
            accent_alt: Color::Rgb(211, 54, 130),
            brand_fg: Color::Rgb(253, 246, 227),
            brand_bg: Color::Rgb(38, 139, 210),
            me_fg: Color::Rgb(253, 246, 227),
            me_bg: Color::Rgb(133, 153, 0),
            channel: Color::Rgb(133, 153, 0),
            dm: Color::Rgb(38, 139, 210),
            thread: Color::Rgb(211, 54, 130),
            unread: Color::Rgb(181, 137, 0),
            active_fg: Color::Rgb(253, 246, 227),
            active_bg: Color::Rgb(38, 139, 210),
            self_nick: Color::Rgb(38, 139, 210),
            self_body: Color::Rgb(7, 54, 66),
            mention_fg: Color::Rgb(253, 246, 227),
            mention_bg: Color::Rgb(181, 137, 0),
            system: Color::Rgb(147, 161, 161),
            timestamp: Color::Rgb(147, 161, 161),
            msg_num: Color::Rgb(147, 161, 161),
            nick_palette: vec![
                Color::Rgb(220, 50, 47),
                Color::Rgb(133, 153, 0),
                Color::Rgb(181, 137, 0),
                Color::Rgb(38, 139, 210),
                Color::Rgb(211, 54, 130),
                Color::Rgb(42, 161, 152),
                Color::Rgb(203, 75, 22),
                Color::Rgb(108, 113, 196),
            ],
            code_bg: Color::Rgb(238, 232, 213),
            syn_keyword: Color::Rgb(203, 75, 22),
            syn_string: Color::Rgb(133, 153, 0),
            syn_number: Color::Rgb(211, 54, 130),
            syn_comment: Color::Rgb(147, 161, 161),
            syn_type: Color::Rgb(181, 137, 0),
            syn_fn: Color::Rgb(38, 139, 210),
            syn_default: Color::Rgb(7, 54, 66),
            ok: Color::Rgb(133, 153, 0),
            warn: Color::Rgb(181, 137, 0),
            err: Color::Rgb(220, 50, 47),
            presence: Color::Rgb(133, 153, 0),
        }
    }

    pub fn nick_color(&self, nick: &str) -> Color {
        let h: u64 = nick.bytes().map(|b| b as u64).fold(5381, |acc, b| acc * 33 + b);
        self.nick_palette[(h % self.nick_palette.len() as u64) as usize]
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    theme: Option<String>,
}

fn config_path() -> std::path::PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("chattui").join("config.toml")
}

/// Load the saved theme (if any). CLI/`--theme` flag should still win over this.
pub fn load_saved_theme() -> Option<String> {
    let content = std::fs::read_to_string(config_path()).ok()?;
    let cfg: Config = toml::from_str(&content).ok()?;
    cfg.theme
}

/// Persist the chosen theme so next launch remembers it.
pub fn save_theme(name: &str) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cfg = Config { theme: Some(name.to_string()) };
    std::fs::write(path, toml::to_string_pretty(&cfg)?)?;
    Ok(())
}
