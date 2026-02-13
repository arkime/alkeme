mod app;
mod api;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

/// alkeme - Rust TUI for Arkime
///
/// Keybindings:
///   Tab/Shift+Tab: switch tabs, j/k: navigate, Enter: open detail,
///   Esc: close overlay, t/T: cycle time range, /: expression search,
///   r: refresh, q: quit
#[derive(Parser)]
#[command(name = "alkeme", version)]
struct Cli {
    /// Arkime viewer URL
    #[arg(default_value = "http://localhost:8005")]
    url: String,

    /// Authentication mode
    #[arg(long, value_parser = ["basic", "digest", "form"])]
    auth: Option<String>,

    /// Credentials as user:pass (prompts if omitted with --auth)
    #[arg(long)]
    user: Option<String>,

    /// Default search expression for sessions
    #[arg(long)]
    search: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let auth_mode = match cli.auth.as_deref() {
        Some("basic") => api::AuthMode::Basic,
        Some("digest") => api::AuthMode::Digest,
        Some("form") => api::AuthMode::Form,
        _ => api::AuthMode::None,
    };

    let (username, password) = if let Some(userpass) = &cli.user {
        let (u, p) = userpass.split_once(':').expect("--user format is user:pass");
        (Some(u.to_string()), Some(p.to_string()))
    } else if auth_mode != api::AuthMode::None {
        eprint!("Username: ");
        let mut user = String::new();
        io::stdin().read_line(&mut user)?;
        let user = user.trim().to_string();
        let pass = rpassword::prompt_password("Password: ")?;
        (Some(user), Some(pass))
    } else {
        (None, None)
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(&cli.url, auth_mode, username, password);
    if let Some(search) = cli.search {
        app.expression = search.clone();
        app.expression_edit = search;
    }
    app.client.login().await?;
    app.fetch_user().await;
    app.fetch_fields().await;
    app.fetch_sessions().await;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut packets_handle: Option<tokio::task::JoinHandle<Result<crate::api::PacketsData, anyhow::Error>>> = None;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if app.pending_packets_fetch {
            app.pending_packets_fetch = false;
            let node = std::mem::take(&mut app.packets_node_pending);
            let id = std::mem::take(&mut app.packets_id_pending);
            let raw = app.packets_raw;
            let url = app.client.packets_url(&node, &id, raw);
            let client = app.client.clone_for_fetch();
            packets_handle = Some(tokio::spawn(async move {
                let html = client.fetch_url(&url).await?;
                Ok(crate::api::parse_packets_html(&html))
            }));
            continue;
        }

        // Check if background packets fetch completed
        if let Some(ref mut handle) = packets_handle {
            if handle.is_finished() {
                let handle = packets_handle.take().unwrap();
                match handle.await {
                    Ok(Ok(mut data)) => {
                        data.total = app.packets_total_pending;
                        app.status_msg = format!("{} packets loaded", app.packets_total_pending);
                        app.packets_view = Some(data);
                        app.packets_scroll = 0;
                    }
                    Ok(Err(e)) => {
                        app.status_msg = format!("Error fetching packets: {e}");
                    }
                    Err(e) => {
                        app.status_msg = format!("Error fetching packets: {e}");
                    }
                }
                app.show_loading = false;
                continue;
            }
        }

        // Auto-refresh stats every 30 seconds when on Stats tab
        if app.active_tab == app::Tab::Stats
            && app.input_mode == app::InputMode::Normal
            && app.stats_last_refresh.elapsed() >= Duration::from_secs(30)
        {
            app.fetch_stats().await;
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                if key.code == KeyCode::Char('q') && !app.is_detail_view() && app.input_mode == app::InputMode::Normal {
                    return Ok(());
                }
                app.handle_key(key).await;
            }
    }
}
