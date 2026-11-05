use image::imageops::FilterType;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub const ART_WIDTH: usize = 20;
pub const ART_HEIGHT_PIXELS: usize = 20;

#[derive(Debug, Clone)]
pub struct TerminalCoverArt {
    pub grid: Vec<Vec<(u8, u8, u8)>>,
    #[allow(dead_code)]
    pub is_real_image: bool,
}

impl TerminalCoverArt {
    pub fn from_jpeg_or_seed(bytes: Option<&[u8]>, seed_text: &str) -> Self {
        if let Some(raw) = bytes {
            if let Ok(img) = image::load_from_memory(raw) {
                let resized = img
                    .resize_exact(ART_WIDTH as u32, ART_HEIGHT_PIXELS as u32, FilterType::Lanczos3)
                    .to_rgb8();
                let mut grid = vec![vec![(0u8, 0u8, 0u8); ART_WIDTH]; ART_HEIGHT_PIXELS];
                for y in 0..ART_HEIGHT_PIXELS {
                    for x in 0..ART_WIDTH {
                        let p = resized.get_pixel(x as u32, y as u32);
                        grid[y][x] = (p[0], p[1], p[2]);
                    }
                }
                return Self {
                    grid,
                    is_real_image: true,
                };
            }
        }
        Self::procedural_fallback(seed_text)
    }

    fn procedural_fallback(seed_text: &str) -> Self {
        let mut grid = vec![vec![(24u8, 28u8, 44u8); ART_WIDTH]; ART_HEIGHT_PIXELS];
        let hash = fnv1a_hash(seed_text.as_bytes());

        let r_base = ((hash & 0xFF) as u8).wrapping_mul(3);
        let g_base = (((hash >> 8) & 0xFF) as u8).wrapping_mul(5);
        let b_base = (((hash >> 16) & 0xFF) as u8).wrapping_mul(7);

        let r_accent = ((hash >> 24) & 0xFF) as u8;
        let g_accent = ((hash >> 32) & 0xFF) as u8;
        let b_accent = ((hash >> 40) & 0xFF) as u8;

        for y in 0..ART_HEIGHT_PIXELS {
            for x in 0..ART_WIDTH {
                let dx = (x as isize - (ART_WIDTH as isize / 2)).unsigned_abs();
                let dy = (y as isize - (ART_HEIGHT_PIXELS as isize / 2)).unsigned_abs();
                let dist = dx + dy;

                if dist <= 3 {
                    grid[y][x] = (r_accent, g_accent, b_accent);
                } else if dist <= 8 {
                    let groove = (dist * 24) as u8;
                    grid[y][x] = (
                        r_base.wrapping_add(groove),
                        g_base.wrapping_add(groove / 2),
                        b_base.wrapping_add(groove),
                    );
                } else {
                    let gx = (x * 11) as u8;
                    let gy = (y * 11) as u8;
                    grid[y][x] = (
                        r_base.wrapping_add(gx),
                        g_accent.wrapping_add(gy),
                        b_base.wrapping_add(gx ^ gy),
                    );
                }
            }
        }

        Self {
            grid,
            is_real_image: false,
        }
    }

    pub fn to_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::with_capacity(ART_HEIGHT_PIXELS / 2);
        for y in (0..ART_HEIGHT_PIXELS).step_by(2) {
            let mut spans = Vec::with_capacity(ART_WIDTH);
            for x in 0..ART_WIDTH {
                let top = self.grid[y][x];
                let bottom = self.grid[y + 1][x];
                spans.push(Span::styled(
                    "▀",
                    Style::default()
                        .fg(Color::Rgb(top.0, top.1, top.2))
                        .bg(Color::Rgb(bottom.0, bottom.1, bottom.2)),
                ));
            }
            lines.push(Line::from(spans));
        }
        lines
    }
}

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Clone)]
pub struct TerminalIcon {
    pub rows: Vec<Vec<Span<'static>>>,
}

impl TerminalIcon {
    pub fn load_or_fallback(paths: &[&str], width: usize, height_pixels: usize, fallback_lines: [&'static str; 3], fallback_col: Color) -> Self {
        for path in paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    return Self::from_image(img, width, height_pixels);
                }
            }
        }
        let rows = fallback_lines.iter().map(|&s| vec![Span::styled(s, Style::default().fg(fallback_col).add_modifier(ratatui::style::Modifier::BOLD))]).collect();
        Self { rows }
    }

    fn from_image(img: image::DynamicImage, width: usize, height_pixels: usize) -> Self {
        let orig = img.to_rgba8();
        let (w, h) = (orig.width(), orig.height());
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0, 0);
        for y in (0..h).step_by(2) {
            for x in (0..w).step_by(2) {
                let p = orig.get_pixel(x, y);
                if p[3] >= 32 && (p[0] > 25 || p[1] > 25 || p[2] > 25) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        let cropped = if min_x < max_x && min_y < max_y {
            img.crop_imm(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
        } else {
            img
        };
        let rgba = cropped.resize_exact(width as u32, height_pixels as u32, FilterType::Lanczos3).to_rgba8();

    let mut rows = Vec::with_capacity(height_pixels / 2);
    for y in (0..height_pixels).step_by(2) {
        let mut spans = Vec::with_capacity(width);
        for x in 0..width {
            let top = rgba.get_pixel(x as u32, y as u32);
            let bot = rgba.get_pixel(x as u32, (y + 1) as u32);

            let top_vis = top[3] >= 64 && (top[0] > 25 || top[1] > 25 || top[2] > 25);
            let bot_vis = bot[3] >= 64 && (bot[0] > 25 || bot[1] > 25 || bot[2] > 25);

            match (top_vis, bot_vis) {
                (true, true) => spans.push(Span::styled("▀", Style::default().fg(Color::Rgb(top[0], top[1], top[2])).bg(Color::Rgb(bot[0], bot[1], bot[2])))),
                (true, false) => spans.push(Span::styled("▀", Style::default().fg(Color::Rgb(top[0], top[1], top[2])))),
                (false, true) => spans.push(Span::styled("▄", Style::default().fg(Color::Rgb(bot[0], bot[1], bot[2])))),
                (false, false) => spans.push(Span::raw(" ")),
            }
        }
        rows.push(spans);
    }

    Self { rows }
}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_load_dimensions() {
        let icon = TerminalIcon::load_or_fallback(
            &["../../assets/icons/transparent-play.png", "assets/icons/transparent-play.png"],
            6, 6,
            [" █◣   ", " ██◣  ", " █◤   "],
            Color::Green,
        );
        assert_eq!(icon.rows.len(), 3);
        for row in &icon.rows {
            let width: usize = row.iter().map(|s| s.content.chars().count()).sum();
            assert_eq!(width, 6);
        }
    }
}