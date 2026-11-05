use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::{ActiveView, TuiApp};
use crate::ui_player::render_player_panel;

pub fn render(frame: &mut Frame<'_>, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(12)])
        .split(frame.area());
    let nav_tab = |label: &'static str, active: bool| -> Span<'static> {
        let style = if active { Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Green) };
        Span::styled(label, style)
    };
    let l = app.language;
    let nav_line = Line::from(vec![
        Span::raw("─"),
        nav_tab(l.nav_home(), app.active_view == ActiveView::Home),
        Span::raw(" ── "),
        nav_tab(l.nav_new(), app.active_view == ActiveView::New),
        Span::raw(" ── "),
        nav_tab(l.nav_radio(), app.active_view == ActiveView::Radio),
        Span::raw(" ── "),
        nav_tab(l.nav_library(), app.active_view == ActiveView::Library),
        Span::raw(" ── "),
        nav_tab(l.nav_playlists(), app.active_view == ActiveView::Playlists),
        Span::raw(" ── "),
        nav_tab(l.nav_queue(), app.active_view == ActiveView::Queue),
        Span::raw(" ── "),
        nav_tab(l.nav_lang(), false),
        Span::raw("───"),
    ]);

    let search_text = if app.is_editing {
        format!(" 🔍 {}█", app.search_input)
    } else if app.search_input.is_empty() {
        l.search_placeholder().to_string()
    } else {
        format!(" 🔍 {}", app.search_input)
    };

    let mut top_actions = Vec::new();
    if !app.is_logged_in() {
        top_actions.push(Span::styled(l.nav_login(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        top_actions.push(Span::raw("──"));
    }
    top_actions.push(nav_tab(l.nav_help(), app.active_view == ActiveView::Help));
    top_actions.push(Span::raw(" "));
    let help_line = Line::from(top_actions).alignment(Alignment::Right);

    let top_block = Block::default()
        .borders(Borders::ALL)
        .title_top(" Spore ")
        .title_top(help_line);

    frame.render_widget(
        Paragraph::new(search_text)
            .style(if app.is_editing { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::White) })
            .block(top_block),
        chunks[0],
    );

    match app.active_view {
        ActiveView::Help => crate::ui_help::render_help(frame, chunks[1], app, nav_line),
        _ => render_central_with_lyrics(frame, chunks[1], app, nav_line),
    }
    render_player_panel(frame, chunks[2], app);
}

fn render_central_with_lyrics(frame: &mut Frame<'_>, area: Rect, app: &TuiApp, nav_line: Line<'static>) {
    let (lbl_a, lbl_n, lbl_p, lbl_c) = match app.language {
        spore_core::Language::Es => (" Encolar", " A Continuación", " Reproducir Lista", " Limpiar"),
        spore_core::Language::En => (" Enqueue", " Play Next", " Play Playlist", " Clear"),
    };
    let lbl_y = match app.phonetic_mode {
        crate::app::PhoneticMode::Dual => app.language.btn_phonetic_dual(),
        crate::app::PhoneticMode::Phonetic => app.language.btn_phonetic_only(),
        crate::app::PhoneticMode::Original => app.language.btn_phonetic_off(),
    };
    let bottom_bar = match app.active_view {
        ActiveView::Playlists => Line::from(vec![
            Span::raw("─"),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Abrir", Style::default().fg(Color::DarkGray)),
            Span::raw(" ── "),
            Span::styled("p", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_p, Style::default().fg(Color::DarkGray)),
            Span::raw("─"),
        ]),
        ActiveView::Queue => Line::from(vec![
            Span::raw("─"),
            Span::styled("d", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Quitar", Style::default().fg(Color::DarkGray)),
            Span::raw(" ── "),
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_y, Style::default().fg(Color::Yellow)),
            Span::raw(" ── "),
            Span::styled("c", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_c, Style::default().fg(Color::DarkGray)),
            Span::raw("─"),
        ]),
        _ => Line::from(vec![
            Span::raw("─"),
            Span::styled("a", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_a, Style::default().fg(Color::DarkGray)),
            Span::raw(" ── "),
            Span::styled("n", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_n, Style::default().fg(Color::DarkGray)),
            Span::raw(" ── "),
            Span::styled("p", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_p, Style::default().fg(Color::DarkGray)),
            Span::raw(" ── "),
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_y, Style::default().fg(Color::Yellow)),
            Span::raw(" ── "),
            Span::styled("c", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(lbl_c, Style::default().fg(Color::DarkGray)),
            Span::raw("─"),
        ]),
    };
    let outer = Block::default().borders(Borders::ALL).title_top(nav_line).title_bottom(bottom_bar);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(62), Constraint::Percentage(38)]).split(inner);
    match app.active_view {
        ActiveView::Queue => render_queue(frame, cols[0], app),
        ActiveView::Playlists => render_playlists(frame, cols[0], app),
        _ => render_tracks(frame, cols[0], app),
    }
    crate::ui_lyrics::render_lyrics(frame, cols[1], app);
}

fn render_playlists(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let items: Vec<ListItem<'_>> = if app.playlists.is_empty() {
        vec![ListItem::new(Line::from(Span::styled("  No hay playlists cargadas. Presiona '5' para sincronizar o 'u' para iniciar sesión.", Style::default().fg(Color::DarkGray))))]
    } else {
        app.playlists.iter().map(|p| ListItem::new(Line::from(format!("📁 {:<36} │ {}", truncate(&p.name, 34), truncate(p.description.as_deref().unwrap_or(""), 36))))).collect()
    };
    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD));
    let mut state = app.playlist_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}



fn render_queue(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let upcoming = app.queue.upcoming();
    let avail = (area.width as usize).saturating_sub(18);
    let (wt, wa) = if avail < 30 { (28, 16) } else {
        let wt = (avail * 60) / 100;
        let wa = avail.saturating_sub(wt);
        (wt, wa)
    };
    let items: Vec<ListItem<'_>> = if upcoming.is_empty() {
        vec![ListItem::new(Line::from(Span::styled("  La cola está vacía. Selecciona pistas y presiona 'a' o 'n'.", Style::default().fg(Color::DarkGray))))]
    } else {
        upcoming.iter().enumerate().map(|(i, t)| ListItem::new(Line::from(format!("{:>2}. {:<wt$} │ {:<wa$} │ {}", i + 1, truncate(&t.title, wt.saturating_sub(1)), truncate(&t.artist, wa.saturating_sub(1)), t.formatted_duration())))).collect()
    };
    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD));
    let mut state = app.queue_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tracks(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let avail = (area.width as usize).saturating_sub(18);
    let (wt, wa, wal) = if avail < 40 {
        (26, 16, 14)
    } else {
        let wt = (avail * 46) / 100;
        let wa = (avail * 24) / 100;
        let wal = avail.saturating_sub(wt + wa);
        (wt, wa, wal)
    };
    let items: Vec<ListItem<'_>> = app.tracks.iter().map(|t| ListItem::new(Line::from(format!("{:<wt$} │ {:<wa$} │ {:<wal$} │ {}", truncate(&t.title, wt.saturating_sub(1)), truncate(&t.artist, wa.saturating_sub(1)), truncate(&t.album, wal.saturating_sub(1)), t.formatted_duration())))).collect();
    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD));
    let mut state = app.list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}



pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars { s.to_string() } else { let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect(); out.push('…'); out }
}