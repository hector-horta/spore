use ratatui::{layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph}, Frame};
use crate::app::TuiApp;
use crate::ui::truncate;

pub fn render_player_panel(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(30)])
        .split(area);

    let art_lines = match &app.current_artwork {
        Some(a) => a.to_lines(),
        None => crate::artwork::TerminalCoverArt::from_jpeg_or_seed(None, "Spore").to_lines(),
    };
    frame.render_widget(
        Paragraph::new(art_lines).block(Block::default().borders(Borders::ALL)),
        bottom_chunks[0],
    );

let header_line = match &app.current_track {
    Some(t) => Line::from(vec![
        Span::styled(format!("{} ", truncate(&t.title, 34)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("— {} • {}", truncate(&t.artist, 22), truncate(&t.album, 22)), Style::default().fg(Color::Green)),
    ]),
    None => Line::from(Span::styled(app.language.no_active_playback(), Style::default().fg(Color::DarkGray))),
};



let (elapsed, total) = app.playback_progress();
let elap_s = elapsed.as_secs();
let tot_s = total.as_secs().max(app.current_track.as_ref().map(|t| t.duration_ms / 1000).unwrap_or(0));
let rem_s = tot_s.saturating_sub(elap_s);
let ratio = if tot_s > 0 { (elapsed.as_millis() as f64 / (tot_s as f64 * 1000.0)).clamp(0.0, 1.0) } else { 0.0 };
let (shuf_str, shuf_col) = if app.queue.is_shuffle() {
    ("🔀  ON [s]", Color::Rgb(200, 235, 245))
} else {
    ("🔀  OFF [s]", Color::DarkGray)
};
let (rep_str, rep_col) = match app.queue.repeat_mode() {
    spore_core::queue::RepeatMode::Off => ("🔁  OFF [r]", Color::DarkGray),
    spore_core::queue::RepeatMode::All => ("🔁  ALL [r]", Color::Green),
    spore_core::queue::RepeatMode::One => ("🔂  ONE [r]", Color::Green),
};

let time_line = Line::from(Span::styled(
    format!("[{:02}:{:02} / {:02}:{:02} (-{:02}:{:02})]", elap_s / 60, elap_s % 60, tot_s / 60, tot_s % 60, rem_s / 60, rem_s % 60),
    Style::default().fg(Color::Rgb(220, 225, 230)).add_modifier(Modifier::BOLD)
));

let get_glyph_spans = |r: u8| -> Vec<Span<'static>> {
    let row_idx = (2 - r) as usize;
    match app.playback_status() {
        spore_core::models::PlaybackStatus::Playing => {
            app.play_icon.rows.get(row_idx).cloned().unwrap_or_else(|| vec![Span::raw("      ")])
        }
        spore_core::models::PlaybackStatus::Paused | spore_core::models::PlaybackStatus::Stopped => {
            app.pause_icon.rows.get(row_idx).cloned().unwrap_or_else(|| vec![Span::raw("      ")])
        }
        spore_core::models::PlaybackStatus::Buffering => {
            vec![Span::styled(if r == 1 { "  ↻   " } else { "      " }, Style::default().fg(Color::Green))]
        }
    }
};

let vol = (app.volume * 100.0).round() as u8;
let vol_lit = ((vol as f64 / 100.0) * 4.0).round() as usize;
let vol_steps = ['▂', '▄', '▆', '█'];
let vol_bar: Vec<Span<'static>> = std::iter::once(Span::raw("  "))
    .chain(vol_steps.iter().enumerate().map(|(i, &s)| {
        let col = if i < vol_lit { Color::Green } else { Color::DarkGray };
        Span::styled(s.to_string(), Style::default().fg(col))
    }))
    .collect();

let switches_line = Line::from(vec![
    Span::styled(shuf_str, Style::default().fg(shuf_col)),
    Span::styled("   │   ", Style::default().fg(Color::DarkGray)),
    Span::styled(rep_str, Style::default().fg(rep_col)),
]);

let spec = app.current_spectrum();
let progress_cell = (ratio * 48.0).clamp(0.0, 48.0);
let dim_col = Color::Rgb(65, 70, 75);
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

let srow = |r: u8, c: Color, v_spans: Vec<Span<'static>>| -> Line<'static> {
    let mut v = get_glyph_spans(r);
    v.push(Span::raw("  "));
    for b in 0..12 {
        let val = spec[b] * 24.0;
        let idx = if val < 0.5 && r == 0 { 1 } else { ((val - (r as f32 * 8.0)).clamp(0.0, 8.0)) as usize };
        let ch = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx];
        let ch_str = ch.to_string();
        for col_idx in 0..4 {
            let cell = (b * 4 + col_idx) as f64;
            let col = if cell + 1.0 <= progress_cell {
                c
            } else if cell <= progress_cell {
                lerp_col(dim_col, c, progress_cell - cell)
            } else {
                dim_col
            };
            v.push(Span::styled(ch_str.clone(), Style::default().fg(col)));
        }
        if b < 11 {
            v.push(Span::raw(" "));
        }
    }
    v.push(Span::raw("  "));
    v.extend(v_spans);
    Line::from(v)
};

let (lbl_next, lbl_queue) = match app.language {
    spore_core::Language::Es => ("A continuación", "en cola"),
    spore_core::Language::En => ("Up next", "in queue"),
};

let footer_line = match (app.queue.peek_next(), app.status_message.is_empty()) {
    (Some(next), true) => Line::from(Span::styled(format!("{lbl_next}: {} — {} ({} {lbl_queue})", truncate(&next.title, 34), truncate(&next.artist, 20), app.queue.len()), Style::default().fg(Color::Rgb(120, 160, 160)))),
    (Some(next), false) => Line::from(vec![
        Span::styled(format!("{lbl_next}: {} — {} ({})  │  ", truncate(&next.title, 34), truncate(&next.artist, 20), app.queue.len()), Style::default().fg(Color::Rgb(120, 160, 160))),
        Span::styled(truncate(&app.status_message, 42), Style::default().fg(Color::Rgb(120, 160, 160)).add_modifier(Modifier::ITALIC)),
    ]),
    (None, false) => Line::from(Span::styled(format!("{}", app.status_message), Style::default().fg(Color::Rgb(120, 160, 160)))),
    (None, true) => Line::from(""),
};

let bottom_controls = Line::from(vec![
    Span::raw("─"),
    Span::styled("[Enter - Play]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    Span::raw(" ── "),
    Span::styled(app.language.btn_pause(), Style::default().fg(Color::DarkGray)),
    Span::raw(" ── "),
    Span::styled(app.language.btn_prev_next(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    Span::raw(" ── "),
    Span::styled("[-/+ - Vol]", Style::default().fg(Color::DarkGray)),
    Span::raw("─"),
]);

    frame.render_widget(
        Paragraph::new(vec![
            header_line,
            Line::from(""),
            srow(2, Color::Rgb(255, 215, 0), vec![Span::styled("   VOL", Style::default().fg(Color::DarkGray))]),
            srow(1, Color::Rgb(0, 240, 180), vec![Span::styled(format!("  {:>4}", format!("{}%", vol)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))]),
            srow(0, Color::Rgb(0, 200, 100), vol_bar),
            time_line,
            switches_line,
            Line::from(""),
            footer_line,
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title_bottom(bottom_controls)),
        bottom_chunks[1],
    );
}