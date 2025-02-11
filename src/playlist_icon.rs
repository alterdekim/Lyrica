use std::collections::HashSet;

use color_eyre::owo_colors::OwoColorize;
use image::{DynamicImage, GenericImageView};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::Widget,
};

#[derive(Default, Clone)]
pub struct PlaylistIcon {
    colors: [[u8; 3]; 8],
}

impl PlaylistIcon {
    pub fn new(img: DynamicImage) -> Self {
        let pixels = img
            .resize_exact(8, 8, image::imageops::FilterType::Nearest)
            .to_rgb8()
            .pixels()
            .map(|p| p.0)
            .collect::<HashSet<[u8; 3]>>()
            .iter()
            .copied()
            .collect::<Vec<[u8; 3]>>();

        Self {
            colors: pixels[..8].try_into().unwrap(),
        }
    }
}

impl Widget for PlaylistIcon {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut i = 0;

        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                let color = self.colors[i];
                buf.set_string(
                    x,
                    y,
                    "█",
                    Style::default().fg(Color::Rgb(color[0], color[1], color[2])),
                );
                i = if i >= self.colors.len() - 1 { 0 } else { i + 1 };
            }
        }
    }
}
