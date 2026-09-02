use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    pub accent: Color,
    pub accent_dark: Color,
    pub ok: Color,
    pub bad: Color,
    pub warning: Color,
}

pub const QENLO_THEME: Theme = Theme {
    bg: Color::Rgb(18, 22, 20),
    surface: Color::Rgb(27, 34, 30),
    surface_raised: Color::Rgb(36, 45, 40),
    border: Color::Rgb(43, 53, 48),
    border_strong: Color::Rgb(68, 83, 75),
    text: Color::Rgb(247, 245, 240),
    text_muted: Color::Rgb(155, 165, 159),
    text_faint: Color::Rgb(106, 117, 112),
    accent: Color::Rgb(239, 139, 121),
    accent_dark: Color::Rgb(181, 60, 47),
    ok: Color::Rgb(112, 225, 161),
    bad: Color::Rgb(255, 154, 145),
    warning: Color::Rgb(245, 166, 35),
};

impl Theme {
    pub fn normal(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg)
    }

    pub fn surface_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.surface)
    }

    pub fn header(&self) -> Style {
        Style::default().fg(self.accent).bg(self.surface).add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn border_active(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    pub fn faint(&self) -> Style {
        Style::default().fg(self.text_faint)
    }

    pub fn accent_bold(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn selected_row(&self) -> Style {
        Style::default().fg(self.text).bg(self.surface_raised).add_modifier(Modifier::BOLD)
    }

    pub fn ok_style(&self) -> Style {
        Style::default().fg(self.ok).add_modifier(Modifier::BOLD)
    }

    pub fn bad_style(&self) -> Style {
        Style::default().fg(self.bad).add_modifier(Modifier::BOLD)
    }
}
