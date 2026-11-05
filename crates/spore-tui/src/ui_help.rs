use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use crate::app::TuiApp;
use spore_core::Language;

pub fn render_help(frame: &mut Frame<'_>, area: Rect, app: &TuiApp, nav_line: Line<'static>) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let is_es = app.language == Language::Es;
    let shortcuts_content = if is_es {
        vec![
            Line::from(""),
            Line::from(Span::styled("  🧭 Navegación Superior", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [1-5] Secciones   [/] Buscar   [Tab] Cola   [?] Ayuda"),
            Line::from(""),
            Line::from(Span::styled("  📋 Gestión de Cola & Playlists", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [p] Reproducir Lista   [a] Encolar   [n] A continuación"),
            Line::from("  [d] Quitar Pista       [c] Limpiar Todo      [y] Fonética (Dual/Romaji)"),
            Line::from(""),
            Line::from(Span::styled("  🎵 Controles del Reproductor", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [Enter] Play           [Espacio] Pausa   [< / >] Ant/Sig"),
            Line::from("  [s] Aleatorio          [r] Repetir       [- / +] Volumen"),
            Line::from(""),
            Line::from(Span::styled("  ⚙️  Sistema & Configuración", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [L] Cambiar Idioma     [u] Login Spotify [q] Salir"),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("  🧭 Top Navigation", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [1-5] Sections    [/] Search   [Tab] Queue   [?] Help"),
            Line::from(""),
            Line::from(Span::styled("  📋 Queue & Playlists Management", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [p] Play Playlist      [a] Enqueue   [n] Play Next"),
            Line::from("  [d] Remove Track       [c] Clear All         [y] Phonetics (Dual/Romaji)"),
            Line::from(""),
            Line::from(Span::styled("  🎵 Player Controls", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [Enter] Play           [Space] Pause     [< / >] Prev/Next"),
            Line::from("  [s] Shuffle            [r] Repeat        [- / +] Volume"),
            Line::from(""),
            Line::from(Span::styled("  ⚙️  System & Settings", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from("  [L] Toggle Language    [u] Spotify Login [q] Quit"),
        ]
    };

    let shortcuts_block = Block::default().borders(Borders::ALL).title_top(nav_line);

    let mut auth_content = vec![
        Line::from(""),
        Line::from(Span::styled(
            if is_es { "  ⚡ Modo Automático (Con Navegador)" } else { "  ⚡ Automatic Mode (With Browser)" },
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        )),
        Line::from(if is_es { "  [u] Iniciar sesión interactiva con navegador" } else { "  [u] Interactive login via Chromium browser" }),
        Line::from(""),
    ];

    if app.language == Language::Es {
        auth_content.extend_from_slice(&[
            Line::from(vec![Span::styled("Configurar Spotify Client ID:", Style::default().bold().fg(Color::Yellow))]),
            Line::from(vec![Span::styled("1. Ve a developer.spotify.com/dashboard", Style::default())]),
            Line::from(vec![Span::styled("2. Crea una App y añade Redirect URI:", Style::default())]),
            Line::from(vec![Span::styled("   http://127.0.0.1:8888/callback", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("3. En PowerShell ejecuta:", Style::default())]),
            Line::from(vec![Span::styled("   $env:SPOTIFY_CLIENT_ID=\"tu_client_id\"", Style::default().italic().fg(Color::Green))]),
            Line::from(vec![Span::styled("4. Presiona 'u' en Spore para autorizar", Style::default())]),
        ]);
    } else {
        auth_content.extend_from_slice(&[
            Line::from(vec![Span::styled("Setup Spotify Client ID:", Style::default().bold().fg(Color::Yellow))]),
            Line::from(vec![Span::styled("1. Go to developer.spotify.com/dashboard", Style::default())]),
            Line::from(vec![Span::styled("2. Create an App & add Redirect URI:", Style::default())]),
            Line::from(vec![Span::styled("   http://127.0.0.1:8888/callback", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("3. In terminal run:", Style::default())]),
            Line::from(vec![Span::styled("   $env:SPOTIFY_CLIENT_ID=\"your_client_id\"", Style::default().italic().fg(Color::Green))]),
            Line::from(vec![Span::styled("4. Press 'u' in Spore to authorize", Style::default())]),
        ]);
    }

    let right_title = if is_es { "─[🔑 Autenticación & Cuenta]─" } else { "─[🔑 Authentication & Setup]─" };
    let auth_block = Block::default().borders(Borders::ALL).title_top(right_title);

    frame.render_widget(Paragraph::new(shortcuts_content).block(shortcuts_block), cols[0]);
    frame.render_widget(Paragraph::new(auth_content).block(auth_block), cols[1]);
}