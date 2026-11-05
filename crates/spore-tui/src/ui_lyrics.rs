use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::TuiApp;

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() || max_width < 2 { return vec![text.to_string()] }
    let mut lines = Vec::new();
    let mut current_line = String::with_capacity(max_width);
    let mut words = text.split_whitespace();

    for word in words.by_ref() {
        if current_line.is_empty() || current_line.len() + 1 + word.len() <= max_width {
            if !current_line.is_empty() { current_line.push(' '); }
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() { lines.push(current_line) }
    lines
}

pub fn render_lyrics(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let empty_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            app.language.no_lyrics(),
            Style::default().fg(Color::DarkGray)
        )),
    ];

    if app.current_lyrics.is_empty() {
        frame.render_widget(
            Paragraph::new(empty_lines).block(Block::default()
                .borders(Borders::LEFT)
                .title_top(app.language.title_lyrics())
            ),
            area
        );
        return;
    }

    let (elap, tot) = app.playback_progress();
    let cur_ms = elap.as_millis() as u64;
    let tot_ms = (tot.as_millis() as u64).max(cur_ms + 4000);
    let max_w = (area.width as usize).saturating_sub(6).max(10);
    let active_idx = app.current_lyrics.iter()
        .rposition(|l| l.time_ms <= cur_ms)
        .unwrap_or(0);

    let lerp_col = |c1: Color, c2: Color, t: f64| -> Color {
        match (c1, c2) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let r = (r1 as f64 + (r2 as f64 - r1 as f64) * t).clamp(0.0, 255.0) as u8;
                let g = (g1 as f64 + (g2 as f64 - g1 as f64) * t).clamp(0.0, 255.0) as u8;
                let b = (b1 as f64 + (b2 as f64 - b1 as f64) * t).clamp(0.0, 255.0) as u8;
                Color::Rgb(r, g, b)
            }
            _ => if t > 0.5 { c2 } else { c1 },
        }
    };

    let mut visual_lines: Vec<Line<'static>> = Vec::new();
    let mut active_visual_idx = 0;
    for (idx, line) in app.current_lyrics.iter().enumerate() {
        if idx == active_idx {
            let next_ms = app.current_lyrics.get(idx + 1).map(|n| n.time_ms).unwrap_or(tot_ms);
            let gap = next_ms.saturating_sub(line.time_ms).max(1200);
            let elapsed = cur_ms.saturating_sub(line.time_ms);
            let sing_dur = ((gap as f64) * 0.88).clamp(800.0, 6000.0);
            let ratio = (elapsed as f64 / sing_dur).clamp(0.0, 1.0);
            let display_text = if app.phonetic_mode == crate::app::PhoneticMode::Phonetic {
                line.phonetic.as_deref().unwrap_or(&line.text)
            } else {
                &line.text
            };
            let total_chars = display_text.chars().count().max(1);
            let progress_char = (total_chars as f64) * ratio;
            let chunks = wrap_text(display_text, max_w);

            let mut global_char_idx = 0usize;
            for (c_idx, chunk) in chunks.into_iter().enumerate() {
                if c_idx == 0 { active_visual_idx = visual_lines.len(); }
                let mut spans = vec![if c_idx == 0 {
                    Span::styled(" ♪ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                } else {
                    Span::raw("   ")
                }];
                for ch in chunk.chars() {
                    let cell = global_char_idx as f64;
                    let (col, is_bold) = if cell + 1.0 <= progress_char {
                        (Color::White, true)
                    } else if cell <= progress_char {
                        let t = progress_char - cell;
                        (lerp_col(Color::Rgb(95, 100, 110), Color::White, t), true)
                    } else {
                        (Color::Rgb(95, 100, 110), false)
                    };
                    let style = if is_bold {
                        Style::default().fg(col).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(col)
                    };
                    spans.push(Span::styled(ch.to_string(), style));
                    global_char_idx += 1;
                }
                global_char_idx += 1;
                visual_lines.push(Line::from(spans));
            }
            if app.phonetic_mode == crate::app::PhoneticMode::Dual {
                if let Some(phonetic) = &line.phonetic {
                    visual_lines.push(Line::from(vec![
                        Span::raw("   "),
                        Span::styled(format!("⤷ {phonetic}"), Style::default().fg(Color::Yellow)),
                    ]));
                }
            }
        } else {
            let text = if app.phonetic_mode == crate::app::PhoneticMode::Phonetic {
                line.phonetic.as_deref().unwrap_or(&line.text)
            } else {
                &line.text
            };
            let chunks = wrap_text(text, max_w);
            for chunk in chunks {
                visual_lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(chunk, Style::default().fg(Color::Rgb(80, 85, 95))),
                ]));
            }
        }
    }

    let height = area.height as usize;
    let start = active_visual_idx.saturating_sub(height / 2);
    let visible: Vec<Line<'static>> = visual_lines.into_iter().skip(start).take(height).collect();
    frame.render_widget(
        Paragraph::new(visible)
            .block(Block::default()
                .borders(Borders::LEFT)
                .title_top(app.language.title_lyrics())),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_long_line() {
        let text = "And it's stickin' out, yeah, like a sore thumb (blowin' up)";
        let wrapped = wrap_text(text, 28);
        assert!(wrapped.len() >= 2);
        assert_eq!(wrapped[0], "And it's stickin' out, yeah,");
    }
}