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

    /// Override app mode (viewer, cont3xt, wise, parliament) — skips /api/appversion
    #[arg(long, value_parser = ["viewer", "cont3xt", "wise", "parliament"])]
    app: Option<String>,

    /// Authentication mode (none, basic, digest*, form, web, okta)
    #[arg(long, value_parser = ["none", "basic", "digest", "form", "web", "okta"], default_value = "digest", hide_default_value = true, hide_possible_values = true)]
    auth: String,

    /// Load Cont3xt results from a JSON file (saved by --cont3xt-save-json or J key) without running a search
    #[arg(long)]
    cont3xt_read_json: Option<String>,

    /// Run a Cont3xt search and save JSON results to a file, then quit
    #[arg(long)]
    cont3xt_save_json: Option<String>,

    /// Default search expression for Cont3xt only (overrides --search for Cont3xt)
    #[arg(long)]
    cont3xt_search: Option<String>,

    /// Comma-separated tags to include with Cont3xt searches
    #[arg(long)]
    cont3xt_tags: Option<String>,

    /// Select a Cont3xt integration view by ID or name
    #[arg(long)]
    cont3xt_view: Option<String>,

    /// Cookie jar file path — encrypted session cookies + username between runs. Prompts for jar password each run. (File created with 0600 permissions)
    #[arg(long)]
    jar: Option<String>,

    /// Cookie jar password. If prefixed with |, runs the rest as a command and uses the first line of output.
    #[arg(long)]
    jar_password: Option<String>,

    /// Authentication password. If prefixed with |, runs the rest as a command and uses the first line of output. Overrides the password portion of --user.
    #[arg(long)]
    password: Option<String>,

    /// Default search expression (sessions query for Viewer, indicator for Cont3xt)
    #[arg(long)]
    search: Option<String>,

    /// Credentials as user:pass (prompts if omitted with --auth)
    #[arg(long)]
    user: Option<String>,

    /// Default search expression for Viewer only (overrides --search for Viewer)
    #[arg(long)]
    viewer_search: Option<String>,

    /// Default time range for Viewer (15m, 30m, 1h, 6h, 24h, 1w, 2w, 1M, All, -1, or {num}h/w/m e.g. 72h, 2w, 3m)
    #[arg(long, allow_hyphen_values = true)]
    viewer_time_range: Option<String>,
}

/// Resolve a value that may be a `|command` pipe. If prefixed with `|`, runs the
/// rest as a shell command and returns the first line of stdout.
fn resolve_pipe_value(val: &str, label: &str) -> Result<String> {
    if let Some(cmd) = val.strip_prefix('|') {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run {label} command: {e}"))?;
        if !output.status.success() {
            eprintln!("{label} command failed with exit code {}", output.status);
            std::process::exit(1);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string())
    } else {
        Ok(val.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let auth_mode = match cli.auth.as_str() {
        "basic" => api::AuthMode::Basic,
        "digest" => api::AuthMode::Digest,
        "form" => api::AuthMode::Form,
        "web" => api::AuthMode::Web,
        "okta" => api::AuthMode::Okta,
        _ => api::AuthMode::None,
    };

    let defers_prompts = auth_mode == api::AuthMode::Web || auth_mode == api::AuthMode::Okta;
    let has_jar = cli.jar.is_some();

    // Resolve --password (supports |command)
    let cli_password = if let Some(ref p) = cli.password {
        Some(resolve_pipe_value(p, "password")?)
    } else {
        None
    };

    let (username, password) = if let Some(userpass) = &cli.user {
        if let Some((u, p)) = userpass.split_once(':') {
            (Some(u.to_string()), Some(cli_password.unwrap_or_else(|| p.to_string())))
        } else {
            // Username only (no colon) — use --password if provided, else prompt/defer
            if let Some(p) = cli_password {
                (Some(userpass.clone()), Some(p))
            } else if defers_prompts || has_jar {
                (Some(userpass.clone()), None)
            } else {
                let pass = rpassword::prompt_password(format!("Password for {userpass}: "))?;
                (Some(userpass.clone()), Some(pass))
            }
        }
    } else if defers_prompts {
        // Web/Okta auth will prompt using form labels after fetching the page
        (None, cli_password)
    } else if auth_mode != api::AuthMode::None {
        if has_jar {
            // Defer prompting — jar might have valid session
            (None, cli_password)
        } else if let Some(p) = cli_password {
            eprint!("Username: ");
            let mut user = String::new();
            io::stdin().read_line(&mut user)?;
            let user = user.trim().to_string();
            (Some(user), Some(p))
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
        if let Some(ref jp) = cli.jar_password {
            Some(resolve_pipe_value(jp, "jar-password")?)
        } else {
            let pass = rpassword::prompt_password("Cookie jar password: ")?;
            Some(pass)
        }
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

    let mut cluster_name: Option<String> = None;
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

    if app_mode == app::AppMode::Viewer {
        if let Ok(health) = client.get_eshealth().await {
            if let Some(name) = health.get("cluster_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                cluster_name = Some(name.to_string());
            }
        }
    }

    let mut app = App::new(&cli.url, auth_mode, username, password, app_mode);
    if let Some(name) = cluster_name {
        app.title_name = name;
    }
    app.http_log = client.http_log(); // sync log before replacing client
    app.client = client; // reuse the already-logged-in client
    app.fetch_user().await;
    // Resolve search expressions: app-specific flags override --search
    let viewer_search = cli.viewer_search.or_else(|| cli.search.clone());
    let cont3xt_search = cli.cont3xt_search.or(cli.search);

    match app_mode {
        app::AppMode::Viewer => {
            if let Some(search) = &viewer_search {
                app.expression = search.clone();
                app.expression_edit = search.clone();
            }
        }
        app::AppMode::Cont3xt => {
            if let Some(search) = &cont3xt_search {
                app.expression = search.clone();
                app.expression_edit = search.clone();
            }
        }
        app::AppMode::Parliament => {
            // Seed saved expressions so they're used when switching to Viewer/Cont3xt
            if let Some(search) = &viewer_search {
                app.parliament.saved_viewer_expression = search.clone();
            }
            if let Some(search) = &cont3xt_search {
                app.parliament.saved_c3_expression = search.clone();
            }
        }
        _ => {}
    }

    if let Some(ref tr_arg) = cli.viewer_time_range {
        match app::TimeRange::parse(tr_arg) {
            Ok(tr) => {
                let idx = app::TimeRange::insert_sorted(&mut app.time_ranges, tr);
                app.time_range = app.time_ranges[idx].clone();
            }
            Err(e) => {
                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }

    if let Some(tags) = cli.cont3xt_tags {
        app.cont3xt.tags = tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }

    if let Some(ref path) = cli.cont3xt_save_json {
        if app_mode != app::AppMode::Cont3xt {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            eprintln!("Error: --cont3xt-save-json requires Cont3xt mode (use --app cont3xt or connect to a Cont3xt server)");
            std::process::exit(1);
        }
        if app.expression.is_empty() {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            eprintln!("Error: --cont3xt-save-json requires a search expression (use --search or --cont3xt-search)");
            std::process::exit(1);
        }
        app.cont3xt.save_json_path = Some(path.clone());
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
            if let Some(ref view_arg) = cli.cont3xt_view {
                // Match by ID first, then by name
                let found = app.cont3xt.views.iter().find(|v| v.id == *view_arg)
                    .or_else(|| app.cont3xt.views.iter().find(|v| v.name == *view_arg));
                if let Some(view) = found {
                    let integrations = view.integrations.clone();
                    let name = view.name.clone();
                    app.cont3xt.active_view_id = Some(view.id.clone());
                    app.cont3xt.active_view_name = Some(name.clone());
                    app.c3_apply_view(&integrations);
                    app.status_msg = format!("Loaded view: {name}");
                } else {
                    app.status_msg = format!("View not found: {view_arg}");
                }
            }
            app.c3_fetch_overviews().await;
            app.c3_fetch_link_groups().await;
            if let Some(ref path) = cli.cont3xt_read_json {
                if let Err(e) = app.c3_load_json(path) {
                    disable_raw_mode()?;
                    execute!(io::stdout(), LeaveAlternateScreen)?;
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            } else if !app.expression.is_empty() {
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
            app.cont3xt.indicator_parents.entry(
                (item.indicator.clone(), item.itype.clone()),
            ).or_default().push((parent_query, parent_itype));
        } else {
            app.cont3xt.results.push(item.clone());
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
            app::AppMode::Viewer => format!("Alkeme - Viewer - {}", app.title_name),
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
            if app.viewer.pending_packets_fetch {
                app.viewer.pending_packets_fetch = false;
                let node = std::mem::take(&mut app.viewer.packets_node_pending);
                let id = std::mem::take(&mut app.viewer.packets_id_pending);
                let raw = app.viewer.packets_raw;
                let url = app.client.vr_packets_url(&node, &id, raw);
                let client = app.client.clone_for_fetch();
                packets_handle = Some(tokio::spawn(async move {
                    let html = client.fetch_url(&url).await?;
                    Ok(crate::api::parse_packets_html(&html))
                }));
                needs_redraw = true;
                continue;
            }

            if app.viewer.pending_summary_fetch {
                app.viewer.pending_summary_fetch = false;
                let field = app.viewer.summary_field.clone();
                let url = app.client.vr_summary_url(&app.expression, app.time_range.date_value(), &app.viewer.active_view);
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
                            let field = app.viewer.summary_field.clone();
                            app.viewer.summary_data = items;
                            app.vr_sort_summary_data();
                            app.viewer.summary_selected = 0;
                            app.viewer.summary_table_state.select(Some(0));
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
                            data.total = app.viewer.packets_total_pending;
                            app.status_msg = format!("{} packets loaded", app.viewer.packets_total_pending);
                            app.viewer.packets_view = Some(data);
                            app.viewer.packets_scroll = 0;
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
                && app.viewer.stats_last_refresh.elapsed() >= Duration::from_secs(30)
            {
                app.vr_fetch_stats().await;
                needs_redraw = true;
            }
        }

        // Cont3xt background tasks
        if app.app_mode == app::AppMode::Cont3xt {
            if app.cont3xt.pending_search {
                app.cont3xt.pending_search = false;
                app.cont3xt.searching = true;
                let query = app.expression.clone();
                let url = app.client.cont3xt_search_url();
                let client = app.client.clone_for_fetch();
                let mut body = serde_json::json!({"query": query});
                if app.cont3xt.no_cache {
                    app.cont3xt.no_cache = false;
                    body["skipCache"] = serde_json::json!(1);
                }
                if let Some(ref view_id) = app.cont3xt.active_view_id {
                    body["viewId"] = serde_json::json!(view_id);
                }
                if !app.cont3xt.disabled_integrations.is_empty() {
                    let enabled: Vec<String> = app.c3_enabled_integration_names();
                    body["doIntegrations"] = serde_json::json!(enabled);
                }
                if !app.cont3xt.tags.is_empty() {
                    body["tags"] = serde_json::json!(app.cont3xt.tags);
                }
                let json_body = body.to_string();
                let shared = c3_streaming_results.clone();
                let stotal = c3_streaming_total.clone();
                let ssent = c3_streaming_sent.clone();
                let disabled = app.cont3xt.disabled_integrations.clone();
                // Clear shared results for new search
                if let Ok(mut vec) = shared.lock() { vec.clear(); }
                stotal.store(0, std::sync::atomic::Ordering::Relaxed);
                ssent.store(0, std::sync::atomic::Ordering::Relaxed);
                app.cont3xt.results.clear();
                app.cont3xt.indicator_parents.clear();
                app.cont3xt.init_indicators.clear();
                app.cont3xt.search_total = 0;
                app.cont3xt.search_sent = 0;
                app.cont3xt.selected = 0;
                app.cont3xt.detail_scroll = 0;
                c3_stream_consumed = 0;
                c3_search_handle = Some(tokio::spawn(async move {
                    client.fetch_post_json_streaming(&url, &json_body, shared, disabled, stotal, ssent).await
                }));
                needs_redraw = true;
                continue;
            }

            // Poll for streaming results and copy into app
            if app.cont3xt.searching {
                // Update sent/total from streaming atomics
                let live_total = c3_streaming_total.load(std::sync::atomic::Ordering::Relaxed);
                let live_sent = c3_streaming_sent.load(std::sync::atomic::Ordering::Relaxed);
                if live_total > 0 {
                    app.cont3xt.search_total = live_total;
                }
                app.cont3xt.search_sent = live_sent;
                if let Ok(vec) = c3_streaming_results.lock() {
                    if vec.len() > c3_stream_consumed {
                        c3_stream_consumed = drain_c3_results(app, &vec, c3_stream_consumed);
                        app.popup_bg_cache = None; // background changed
                        app.status_msg = format!(
                            "Searching... {} results so far",
                            app.cont3xt.results.len()
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
                            let count = app.cont3xt.results.len();
                            app.cont3xt.search_total = total;
                            app.cont3xt.search_itype = itype;
                            app.cont3xt.init_indicators = init_indicators;
                            app.cont3xt.focus = app::Cont3xtFocus::Results;
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
                    app.cont3xt.searching = false;
                    app.popup_bg_cache = None; // background changed

                    // Headless save-json mode: write results and quit
                    if let Some(path) = app.cont3xt.save_json_path.clone() {
                        app.c3_save_json(&path);
                        return Ok(());
                    }
                    needs_redraw = true;
                    continue;
                }
            }
        }

        // Parliament auto-refresh every 30 seconds
        if app.app_mode == app::AppMode::Parliament
            && app.input_mode == app::InputMode::Normal
            && app.parliament.last_refresh.elapsed() >= Duration::from_secs(30)
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
            && app.wise.last_refresh.elapsed() >= Duration::from_secs(30)
        {
            app.ws_fetch_stats().await;
            needs_redraw = true;
        }

        // Users tab: auto-fetch on first visit
        if app.active_tab == app::Tab::Users && app.us_needs_fetch {
            app.us_needs_fetch = false;
            app.us_fetch_users().await;
            needs_redraw = true;
        }

        // Trigger periodic redraws for animated pages (under construction owl)
        if app.needs_animation() {
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
                    if app.parliament.saved_client.is_some() {
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
                            if app.parliament.saved_client.is_some() {
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
