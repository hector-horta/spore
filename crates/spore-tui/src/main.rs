mod app;
mod artwork;
mod navigation;
mod playback;
mod ui;
mod ui_help;
mod ui_lyrics;
mod ui_player;

use std::io;
use std::time::Duration;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--check" || a == "--version") {
        return Ok(());
    }

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create Ratatui terminal")?;

    let mut app = app::TuiApp::new().context("Failed to initialize TuiApp")?;
    let res = run_event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Spore encountered an error: {err:?}");
    }
    Ok(())
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::TuiApp,
) -> Result<()> {
    while !app.should_quit {
        app.poll_bg_events().await;
        app.check_auto_advance().await;
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }

                if app.is_editing {
                    match key_event.code {
                        KeyCode::Enter => {
                            app.is_editing = false;
                            app.execute_search().await;
                        }
                        KeyCode::Esc => app.is_editing = false,
                        KeyCode::Backspace => { let _ = app.search_input.pop(); },
                        KeyCode::Char(c) => app.search_input.push(c),
                        _ => {}
                    }
                } else {
                    match key_event.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Char('/') => app.is_editing = true,

                        KeyCode::Char('a') => app.enqueue_selected_back(),
                        KeyCode::Char('n') => app.enqueue_selected_next(),
                        KeyCode::Char('d') => app.remove_selected_from_queue(),
                        KeyCode::Char('c') => app.clear_current_list(),
                        KeyCode::Char('y') | KeyCode::Char('Y') => app.toggle_phonetic_mode(),
                        KeyCode::Char('s') => { app.toggle_shuffle(); },
                        KeyCode::Char('r') => { app.toggle_repeat(); },
                        KeyCode::Char('1') => app.open_home(),
                        KeyCode::Char('2') => {
                            app.status_message = "Cargando 2 — Novedades en Spotify...".to_string();
                            terminal.draw(|f| ui::render(f, app))?;
                            app.open_section_query(app::ActiveView::New, "top hits 2026", "Novedades").await;
                        }
                        KeyCode::Char('3') => {
                            app.status_message = "Cargando 3 — Estaciones y Mezclas de Radio...".to_string();
                            terminal.draw(|f| ui::render(f, app))?;
                            app.open_section_query(app::ActiveView::Radio, "electronic chill radio mix", "Estaciones de Radio").await;
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => app.toggle_language(),
                        KeyCode::Char('4') => {
                            app.status_message = "Sincronizando 4 — Biblioteca personal de Spotify...".to_string();
                            terminal.draw(|f| ui::render(f, app))?;
                            app.sync_library().await;
                        }
KeyCode::Char('5') => {
    app.status_message = "Sincronizando 5 — Tus Playlists de Spotify...".to_string();
    terminal.draw(|f| ui::render(f, app))?;
    app.sync_playlists().await;
},

KeyCode::Char('p') | KeyCode::Char('P') => {
    app.status_message = "Reproduciendo toda la playlist / lista actual...".to_string();
    terminal.draw(|f| ui::render(f, app))?;
    app.play_all().await;
}
                        KeyCode::Char('?') => {
                            app.active_view = app::ActiveView::Help;
                            app.status_message = "Ayuda de atajos de teclado (Presiona 1-5 o Tab para volver).".to_string();
                        }
                        KeyCode::Char('>') | KeyCode::Char('.') => app.play_next().await,
                        KeyCode::Char('<') | KeyCode::Char(',') => app.play_previous().await,
                        KeyCode::Tab => app.toggle_view(),
                        KeyCode::Char('u') => {
                            app.status_message = "Sincronizando sesión de Spotify...".to_string();
                            terminal.draw(|f| ui::render(f, app))?;
                            app.sync_user_auth().await;
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.next_track(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_track(),
                        KeyCode::Enter => {
                            terminal.draw(|f| ui::render(f, app))?;
                            app.play_selected().await;
                        }
                        KeyCode::Char(' ') => app.toggle_pause().await,
                        KeyCode::Char('+') | KeyCode::Char('=') => app.adjust_volume(0.05).await,
                        KeyCode::Char('-') => app.adjust_volume(-0.05).await,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
