use ratatui::style::Color;

pub struct Theme {
    pub title_bg: Color,
    pub title_fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub spec_fg: Color,
    pub ac_fg: Color,
    pub level_header_fg: Color,
    pub border_focused: Color,
    pub border_inactive: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub env_fg: Color,
}

pub static THEME: Theme = Theme {
    title_bg: Color::Rgb(60, 60, 120),
    title_fg: Color::Rgb(220, 220, 255),
    selected_bg: Color::Rgb(80, 80, 160),
    selected_fg: Color::Rgb(255, 255, 255),
    spec_fg: Color::Rgb(150, 200, 255),
    ac_fg: Color::Rgb(180, 180, 180),
    level_header_fg: Color::Rgb(255, 200, 100),
    border_focused: Color::Rgb(255, 200, 100),
    border_inactive: Color::Rgb(80, 80, 80),
    status_bg: Color::Rgb(30, 30, 50),
    status_fg: Color::Rgb(180, 180, 220),
    env_fg: Color::Rgb(200, 200, 100),
};
