//! Claude Code 风格配色：暖橙强调、深底、少装饰。

use ratatui::style::{Color, Modifier, Style};

/// 近似 Claude Code 深色主题
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub border: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // 深炭灰底
            bg: Color::Rgb(38, 38, 36),
            // 近白正文
            fg: Color::Rgb(245, 245, 240),
            // 次级说明
            muted: Color::Rgb(138, 138, 128),
            // Claude 暖橙
            accent: Color::Rgb(217, 119, 87),
            accent_dim: Color::Rgb(180, 100, 72),
            // 细边框
            border: Color::Rgb(64, 64, 58),
            success: Color::Rgb(122, 162, 122),
            error: Color::Rgb(199, 90, 90),
            warning: Color::Rgb(200, 160, 90),
        }
    }
}

impl Theme {
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn dim(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn key(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }
}
