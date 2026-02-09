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
    #[arg(long, value_parser = ["basic", "digest"])]
    auth: Option<String>,

    /// Credentials as user:pass (prompts if omitted with --auth)
    #[arg(long)]
    user: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let auth_mode = match cli.auth.as_deref() {
        Some("basic") => api::AuthMode::Basic,
        Some("digest") => api::AuthMode::Digest,
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
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Auto-refresh stats every 30 seconds when on Stats tab
        if app.active_tab == app::Tab::Stats
            && app.input_mode == app::InputMode::Normal
            && app.stats_last_refresh.elapsed() >= Duration::from_secs(30)
        {
            app.fetch_stats().await;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
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
}
