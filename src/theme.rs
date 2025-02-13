use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Theme {
    background: u32,
    foreground: u32,
}

impl Theme {
    pub fn background(&self) -> Color {
        Color::from_u32(self.background)
    }

    pub fn foreground(&self) -> Color {
        Color::from_u32(self.foreground)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: 0x00D75FAF,
            foreground: u32::MAX,
        }
    }
}

fn load_theme() {}
