mod app;
mod api;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;
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
    #[arg(long, value_parser = ["basic", "digest", "form", "web", "okta"])]
    auth: Option<String>,

    /// Credentials as user:pass (prompts if omitted with --auth)
    #[arg(long)]
    user: Option<String>,

    /// Default search expression (sessions query for Viewer, indicator for Cont3xt)
    #[arg(long)]
    search: Option<String>,

    /// Comma-separated tags to include with Cont3xt searches
    #[arg(long)]
    cont3xt_tags: Option<String>,

    /// Override app mode (viewer, cont3xt, wise, parliament) — skips /api/appversion
    #[arg(long, value_parser = ["viewer", "cont3xt", "wise", "parliament"])]
    app: Option<String>,

    /// Cookie jar file path — encrypted session cookies + username between runs. Prompts for jar password each run. (File created with 0600 permissions)
    #[arg(long)]
    jar: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let auth_mode = match cli.auth.as_deref() {
        Some("basic") => api::AuthMode::Basic,
        Some("digest") => api::AuthMode::Digest,
        Some("form") => api::AuthMode::Form,
        Some("web") => api::AuthMode::Web,
        Some("okta") => api::AuthMode::Okta,
        _ => api::AuthMode::None,
    };

    let defers_prompts = auth_mode == api::AuthMode::Web || auth_mode == api::AuthMode::Okta;
    let has_jar = cli.jar.is_some();

    let (username, password) = if let Some(userpass) = &cli.user {
        if let Some((u, p)) = userpass.split_once(':') {
            (Some(u.to_string()), Some(p.to_string()))
        } else {
            // Username only (no colon) — prompt for password (unless auth defers or jar will try first)
            if defers_prompts || has_jar {
                (Some(userpass.clone()), None)
            } else {
                let pass = rpassword::prompt_password(format!("Password for {userpass}: "))?;
                (Some(userpass.clone()), Some(pass))
            }
        }
    } else if defers_prompts {
        // Web/Okta auth will prompt using form labels after fetching the page
        (None, None)
    } else if auth_mode != api::AuthMode::None {
        if has_jar {
            // Defer prompting — jar might have valid session
            (None, None)
        } else {
            eprint!("Username: ");
            let mut user = String::new();
            io::stdin().read_line(&mut user)?;
            let user = user.trim().to_string();
            let pass = rpassword::prompt_password("Password: ")?;
            (Some(user), Some(pass))
        }
    } else {
        (None, None)
    };

    // Load cookie jar from file if --jar specified
    let jar_password = if cli.jar.is_some() {
        let pass = rpassword::prompt_password("Cookie jar password: ")?;
        Some(pass)
    } else {
        None
    };

    let (cookie_store, jar_username) = if let Some(ref jar_path) = cli.jar {
        match api::load_cookie_store(jar_path, jar_password.as_deref()) {
            Ok((store, saved_user)) => (Some(Arc::new(reqwest_cookie_store::CookieStoreMutex::new(store))), saved_user),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    } else {
        (None, None)
    };

    // Use saved username from jar if no --user was provided
    let username = username.or(jar_username);

    // For web/okta auth, login before entering raw mode (needs interactive stdin for prompts)
    let mut client = api::ArkimeClient::new(&cli.url, auth_mode, username.clone(), password.clone(), cookie_store);

    // If we have a cookie jar, try to reuse existing session before prompting for login
    let mut jar_session_valid = false;
    if cli.jar.is_some() {
        if let Ok(true) = client.check_session().await {
            jar_session_valid = true;
        }
    }

    if !jar_session_valid && defers_prompts {
        client.login().await?;
        client.fetch_cookie().await.ok();
    }

    // If jar was tried and failed for non-deferred auth, prompt for missing credentials (before raw mode)
    if !jar_session_valid && !defers_prompts && has_jar && auth_mode != api::AuthMode::None {
        if client.username.is_none() {
            eprint!("Username: ");
            let mut user = String::new();
            io::stdin().read_line(&mut user)?;
            let user = user.trim().to_string();
            let pass = rpassword::prompt_password("Password: ")?;
            client.set_credentials(Some(user), Some(pass));
        } else if client.password.is_none() {
            let user = client.username.clone().unwrap();
            let pass = rpassword::prompt_password(format!("Password for {user}: "))?;
            client.set_credentials(Some(user), Some(pass));
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Fetch app version to determine mode
    if !jar_session_valid && !defers_prompts {
        if let Err(e) = client.login().await {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
        client.fetch_cookie().await.ok();
    }

    let app_mode = if let Some(ref app_name) = cli.app {
        match app_name.as_str() {
            "cont3xt" => app::AppMode::Cont3xt,
            "wise" => app::AppMode::Wise,
            "parliament" => app::AppMode::Parliament,
            _ => app::AppMode::Viewer,
        }
    } else {
        match client.get_appversion().await {
            Ok(info) => {
                let app_name = info.get("app").and_then(|v| v.as_str()).unwrap_or("");
                match app_name {
                    "cont3xt" => app::AppMode::Cont3xt,
                    "wise" | "wiseService" => app::AppMode::Wise,
                    "parliament" => app::AppMode::Parliament,
                    _ => app::AppMode::Viewer,
                }
            }
            Err(_) => {
                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;
                eprintln!("\n  ⚠️  Alkeme requires Arkime 6 or later.");
                eprintln!("     The /api/appversion endpoint was not found at {}", cli.url);
                eprintln!("     Please upgrade your Arkime installation.\n");
                std::process::exit(1);
            }
        }
    };

    let mut app = App::new(&cli.url, auth_mode, username, password, app_mode);
    app.http_log = client.http_log(); // sync log before replacing client
    app.client = client; // reuse the already-logged-in client
    app.fetch_user().await;
    if let Some(search) = cli.search {
        app.expression = search.clone();
        app.expression_edit = search;
    }
    if let Some(tags) = cli.cont3xt_tags {
        app.c3_tags = tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }

    // Mode-specific initialization
    match app_mode {
        app::AppMode::Viewer => {
            app.vr_fetch_fields().await;
            app.vr_fetch_sessions().await;
        }
        app::AppMode::Cont3xt => {
            app.c3_fetch_integrations().await;
            app.c3_fetch_views().await;
            app.c3_fetch_overviews().await;
            app.c3_fetch_link_groups().await;
            if !app.expression.is_empty() {
                app.c3_request_search();
            }
        }
        app::AppMode::Parliament => {
            app.pl_fetch_data().await;
            app.pl_fetch_issues().await;
        }
        app::AppMode::Wise => {
            app.ws_fetch_stats().await;
            app.ws_fetch_sources_types().await;
        }
    }

    let result = run_app(&mut terminal, &mut app).await;

    // Save cookies to jar file on exit
    if let Some(ref jar_path) = cli.jar {
        app.client.save_cookies(jar_path, jar_password.as_deref());
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

/// Drain newly arrived streaming C3 results into the app, returning new consumed count
fn drain_c3_results(app: &mut App, vec: &[crate::api::Cont3xtResult], consumed: usize) -> usize {
    for item in &vec[consumed..] {
        if item.name.is_empty() {
            let parent_query = item.data.get("_link_parent_query")
                .and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_itype = item.data.get("_link_parent_itype")
                .and_then(|v| v.as_str()).unwrap_or("").to_string();
            app.c3_indicator_parents.entry(
                (item.indicator.clone(), item.itype.clone()),
            ).or_default().push((parent_query, parent_itype));
        } else {
            app.c3_results.push(item.clone());
        }
    }
    vec.len()
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut packets_handle: Option<tokio::task::JoinHandle<Result<crate::api::PacketsData, anyhow::Error>>> = None;
    let mut summary_handle: Option<tokio::task::JoinHandle<Result<Vec<crate::api::SummaryItem>, anyhow::Error>>> = None;
    let mut c3_search_handle: Option<tokio::task::JoinHandle<Result<(u64, String, Vec<(String, String)>), anyhow::Error>>> = None;
    let c3_streaming_results: std::sync::Arc<std::sync::Mutex<Vec<crate::api::Cont3xtResult>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let c3_streaming_total: std::sync::Arc<std::sync::atomic::AtomicU64> =
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let c3_streaming_sent: std::sync::Arc<std::sync::atomic::AtomicU64> =
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut c3_stream_consumed: usize = 0;

    let mut needs_redraw = true;
    let mut last_title = String::new();

    loop {
        if app.force_clear {
            app.force_clear = false;
            terminal.clear()?;
            needs_redraw = true;
        }
        // Update terminal title when mode changes
        let title = match app.app_mode {
            app::AppMode::Viewer => format!("Alkeme - Viewer - {}", app.client.base_url()),
            app::AppMode::Cont3xt => "Alkeme - Cont3xt".into(),
            app::AppMode::Wise => "Alkeme - WISE".into(),
            app::AppMode::Parliament => "Alkeme - Parliament".into(),
        };
        if title != last_title {
            execute!(io::stdout(), SetTitle(&title))?;
            last_title = title;
        }
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, app))?;
            needs_redraw = false;
        }

        // Viewer-specific background tasks
        if app.app_mode == app::AppMode::Viewer {
            if app.vr_pending_packets_fetch {
                app.vr_pending_packets_fetch = false;
                let node = std::mem::take(&mut app.vr_packets_node_pending);
                let id = std::mem::take(&mut app.vr_packets_id_pending);
                let raw = app.vr_packets_raw;
                let url = app.client.vr_packets_url(&node, &id, raw);
                let client = app.client.clone_for_fetch();
                packets_handle = Some(tokio::spawn(async move {
                    let html = client.fetch_url(&url).await?;
                    Ok(crate::api::parse_packets_html(&html))
                }));
                needs_redraw = true;
                continue;
            }

            if app.vr_pending_summary_fetch {
                app.vr_pending_summary_fetch = false;
                let field = app.vr_summary_field.clone();
                let url = app.client.vr_summary_url(&app.expression, app.time_range.date_value(), &app.vr_active_view);
                let client = app.client.clone_for_fetch();
                summary_handle = Some(tokio::spawn(async move {
                    let body = client.fetch_post(&url, &[("fields", field.as_str())]).await?;
                    let arr: Vec<serde_json::Value> = serde_json::from_str(&body)?;
                    if arr.len() >= 2 {
                        if let Some(data) = arr[1].get("data") {
                            let items: Vec<crate::api::SummaryItem> = serde_json::from_value(data.clone())?;
                            return Ok(items);
                        }
                    }
                    Ok(Vec::new())
                }));
                needs_redraw = true;
                continue;
            }

            // Check if background summary fetch completed
            if let Some(ref mut handle) = summary_handle {
                if handle.is_finished() {
                    let handle = summary_handle.take().unwrap();
                    match handle.await {
                        Ok(Ok(items)) => {
                            let count = items.len();
                            let field = app.vr_summary_field.clone();
                            app.vr_summary_data = items;
                            app.vr_sort_summary_data();
                            app.vr_summary_selected = 0;
                            app.vr_summary_table_state.select(Some(0));
                            app.status_msg = format!("Summary: {} items for {}", count, field);
                        }
                        Ok(Err(e)) => {
                            app.status_msg = format!("Error: {e}");
                        }
                        Err(e) => {
                            app.status_msg = format!("Error: {e}");
                        }
                    }
                    app.show_loading = false;
                    needs_redraw = true;
                    continue;
                }
            }

            // Check if background packets fetch completed
            if let Some(ref mut handle) = packets_handle {
                if handle.is_finished() {
                    let handle = packets_handle.take().unwrap();
                    match handle.await {
                        Ok(Ok(mut data)) => {
                            data.total = app.vr_packets_total_pending;
                            app.status_msg = format!("{} packets loaded", app.vr_packets_total_pending);
                            app.vr_packets_view = Some(data);
                            app.vr_packets_scroll = 0;
                        }
                        Ok(Err(e)) => {
                            app.status_msg = format!("Error fetching packets: {e}");
                        }
                        Err(e) => {
                            app.status_msg = format!("Error fetching packets: {e}");
                        }
                    }
                    app.show_loading = false;
                    needs_redraw = true;
                    continue;
                }
            }

            // Auto-refresh stats every 30 seconds when on Stats tab
            if app.active_tab == app::Tab::Stats
                && app.input_mode == app::InputMode::Normal
                && app.vr_stats_last_refresh.elapsed() >= Duration::from_secs(30)
            {
                app.vr_fetch_stats().await;
                needs_redraw = true;
            }
        }

        // Cont3xt background tasks
        if app.app_mode == app::AppMode::Cont3xt {
            if app.c3_pending_search {
                app.c3_pending_search = false;
                app.c3_searching = true;
                let query = app.expression.clone();
                let url = app.client.cont3xt_search_url();
                let client = app.client.clone_for_fetch();
                let mut body = serde_json::json!({"query": query});
                if app.c3_no_cache {
                    app.c3_no_cache = false;
                    body["skipCache"] = serde_json::json!(1);
                }
                if let Some(ref view_id) = app.c3_active_view_id {
                    body["viewId"] = serde_json::json!(view_id);
                }
                if !app.c3_disabled_integrations.is_empty() {
                    let enabled: Vec<String> = app.c3_enabled_integration_names();
                    body["doIntegrations"] = serde_json::json!(enabled);
                }
                if !app.c3_tags.is_empty() {
                    body["tags"] = serde_json::json!(app.c3_tags);
                }
                let json_body = body.to_string();
                let shared = c3_streaming_results.clone();
                let stotal = c3_streaming_total.clone();
                let ssent = c3_streaming_sent.clone();
                let disabled = app.c3_disabled_integrations.clone();
                // Clear shared results for new search
                if let Ok(mut vec) = shared.lock() { vec.clear(); }
                stotal.store(0, std::sync::atomic::Ordering::Relaxed);
                ssent.store(0, std::sync::atomic::Ordering::Relaxed);
                app.c3_results.clear();
                app.c3_indicator_parents.clear();
                app.c3_init_indicators.clear();
                app.c3_search_total = 0;
                app.c3_search_sent = 0;
                app.c3_selected = 0;
                app.c3_detail_scroll = 0;
                c3_stream_consumed = 0;
                c3_search_handle = Some(tokio::spawn(async move {
                    client.fetch_post_json_streaming(&url, &json_body, shared, disabled, stotal, ssent).await
                }));
                needs_redraw = true;
                continue;
            }

            // Poll for streaming results and copy into app
            if app.c3_searching {
                // Update sent/total from streaming atomics
                let live_total = c3_streaming_total.load(std::sync::atomic::Ordering::Relaxed);
                let live_sent = c3_streaming_sent.load(std::sync::atomic::Ordering::Relaxed);
                if live_total > 0 {
                    app.c3_search_total = live_total;
                }
                app.c3_search_sent = live_sent;
                if let Ok(vec) = c3_streaming_results.lock() {
                    if vec.len() > c3_stream_consumed {
                        c3_stream_consumed = drain_c3_results(app, &vec, c3_stream_consumed);
                        app.popup_bg_cache = None; // background changed
                        app.status_msg = format!(
                            "Searching... {} results so far",
                            app.c3_results.len()
                        );
                        needs_redraw = true;
                    }
                }
            }

            if let Some(ref mut handle) = c3_search_handle {
                if handle.is_finished() {
                    let handle = c3_search_handle.take().unwrap();
                    // Final drain of any remaining results
                    if let Ok(vec) = c3_streaming_results.lock() {
                        c3_stream_consumed = drain_c3_results(app, &vec, c3_stream_consumed);
                    }
                    match handle.await {
                        Ok(Ok((total, itype, init_indicators))) => {
                            let count = app.c3_results.len();
                            app.c3_search_total = total;
                            app.c3_search_itype = itype;
                            app.c3_init_indicators = init_indicators;
                            app.c3_focus = app::Cont3xtFocus::Results;
                            app.status_msg = format!(
                                "Search complete: {} integrations returned data",
                                count
                            );
                        }
                        Ok(Err(e)) => {
                            app.status_msg = format!("Search error: {e}");
                        }
                        Err(e) => {
                            app.status_msg = format!("Search error: {e}");
                        }
                    }
                    app.c3_searching = false;
                    app.popup_bg_cache = None; // background changed
                    needs_redraw = true;
                    continue;
                }
            }
        }

        // Parliament auto-refresh every 30 seconds
        if app.app_mode == app::AppMode::Parliament
            && app.input_mode == app::InputMode::Normal
            && app.pl_last_refresh.elapsed() >= Duration::from_secs(30)
        {
            app.pl_fetch_data().await;
            if app.active_tab == app::Tab::Issues {
                app.pl_fetch_issues().await;
            }
            needs_redraw = true;
        }

        // WISE auto-refresh every 30 seconds
        if app.app_mode == app::AppMode::Wise
            && app.active_tab == app::Tab::WsStats
            && app.input_mode == app::InputMode::Normal
            && app.ws_last_refresh.elapsed() >= Duration::from_secs(30)
        {
            app.ws_fetch_stats().await;
            needs_redraw = true;
        }

        // Drain all pending key events before next draw
        while event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                if key.code == KeyCode::Char('q') && !app.is_detail_view() && app.input_mode == app::InputMode::Normal
                    && !app.q_closes_popup() {
                    if app.pl_saved_client.is_some() {
                        app.pl_return_to_parliament().await;
                        needs_redraw = true;
                        break;
                    } else {
                        return Ok(());
                    }
                }
                app.handle_key(key).await;
                needs_redraw = true;
                // Process remaining queued keys without waiting
                while event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                            return Ok(());
                        }
                        if key.code == KeyCode::Char('q') && !app.is_detail_view() && app.input_mode == app::InputMode::Normal
                            && !app.q_closes_popup() {
                            if app.pl_saved_client.is_some() {
                                app.pl_return_to_parliament().await;
                                needs_redraw = true;
                                break;
                            } else {
                                return Ok(());
                            }
                        }
                        app.handle_key(key).await;
                    } else {
                        break;
                    }
                }
                break;
            }
        }
    }
}
