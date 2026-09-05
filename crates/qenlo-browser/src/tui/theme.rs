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
    bg: Color::Rgb(17, 22, 19),
    surface: Color::Rgb(24, 31, 27),
    surface_raised: Color::Rgb(34, 43, 38),
    border: Color::Rgb(55, 67, 60),
    border_strong: Color::Rgb(87, 103, 94),
    text: Color::Rgb(239, 238, 232),
    text_muted: Color::Rgb(174, 181, 175),
    text_faint: Color::Rgb(122, 133, 126),
    accent: Color::Rgb(233, 137, 111),
    accent_dark: Color::Rgb(181, 76, 54),
    ok: Color::Rgb(128, 213, 160),
    bad: Color::Rgb(239, 125, 119),
    warning: Color::Rgb(224, 181, 92),
};

impl Theme {
    pub fn normal(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg)
    }

    pub fn surface_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.surface)
    }

    pub fn header(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.surface)
            .add_modifier(Modifier::BOLD)
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
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row(&self) -> Style {
        Style::default()
            .fg(self.text)
            .bg(self.surface_raised)
            .add_modifier(Modifier::BOLD)
    }

    pub fn ok_style(&self) -> Style {
        Style::default().fg(self.ok).add_modifier(Modifier::BOLD)
    }

    pub fn bad_style(&self) -> Style {
        Style::default().fg(self.bad).add_modifier(Modifier::BOLD)
    }
}
