use image::DynamicImage;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

#[derive(Default, Clone)]
pub struct PlaylistIcon {
    colors: Vec<[u8; 3]>,
}

impl PlaylistIcon {
    pub fn new(img: DynamicImage) -> Self {
        let img_rgb = img.to_rgb8();
        let pixels = img_rgb.as_raw();

        let r = color_thief::get_palette(pixels, color_thief::ColorFormat::Rgb, 10, 4)
            .unwrap()
            .iter()
            .map(|c| [c.r, c.g, c.b])
            .collect::<Vec<[u8; 3]>>();

        Self { colors: r }
    }

    fn lerp(a: &[u8; 3], b: &[u8; 3], n: f32) -> [u8; 3] {
        let r = (b[0] as f32 - a[0] as f32) * n + b[0] as f32;
        let g = (b[1] as f32 - a[1] as f32) * n + b[1] as f32;
        let b = (b[2] as f32 - a[2] as f32) * n + b[2] as f32;
        [r as u8, g as u8, b as u8]
    }
}

impl Widget for PlaylistIcon {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut i = 0;

        let start = self.colors.first().unwrap_or(&[255u8, 255u8, 255u8]);
        let end = &[0u8, 0u8, 0u8];

        let mut c = *start;

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let n = (((area.bottom() - y).pow(2) + (area.right() - x).pow(2)) as f32).sqrt();
                c = PlaylistIcon::lerp(start, end, n);
                buf[(x, y)]
                    .set_char(' ')
                    .set_bg(Color::Rgb(c[0], c[1], c[2]))
                    .set_fg(Color::Rgb(c[0], c[1], c[2]));
            }
            i = match self.colors.len() {
                0 => 0,
                _ => {
                    if i >= self.colors.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
            };
        }
    }
}
