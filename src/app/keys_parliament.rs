use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

/// Temporarily exit TUI, prompt for credentials, re-enter TUI, and retry login.
/// Returns Ok(()) if login succeeds, Err if user cancels or login fails again.
async fn prompt_credentials_and_login(client: &mut crate::api::ArkimeClient, url: &str) -> anyhow::Result<()> {
    use std::io::Write;
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    println!("Authentication required for {}", url);
    let username = if let Some(existing) = client.username() {
        println!("Using username: {}", existing);
        existing.to_string()
    } else {
        eprint!("Username: ");
        std::io::stderr().flush()?;
        let mut user = String::new();
        std::io::stdin().read_line(&mut user)?;
        user.trim().to_string()
    };
    let password = rpassword::prompt_password("Password: ")?;

    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    client.set_credentials(Some(username), Some(password));
    client.login().await
}

impl App {
    pub async fn handle_parliament_key(&mut self, key: KeyEvent) {
        match self.active_tab {
            Tab::Dashboard => self.handle_dashboard_key(key).await,
            Tab::Issues => self.handle_issues_key(key).await,
            Tab::Settings => self.handle_parliament_settings_key(key).await,
            _ => {
                match key.code {
                    KeyCode::Tab => self.next_tab(),
                    KeyCode::BackTab => self.prev_tab(),
                    KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
                    _ => {}
                }
            }
        }
    }

    async fn handle_dashboard_key(&mut self, key: KeyEvent) {
        if self.parliament.show_detail {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.parliament.show_detail = false;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.parliament.detail_scroll = self.parliament.detail_scroll.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.parliament.detail_scroll += 10;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.parliament.detail_scroll = self.parliament.detail_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.parliament.detail_scroll += 1;
                }
                KeyCode::Home | KeyCode::Left => self.parliament.detail_scroll = 0,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('r') => {
                self.pl_fetch_data().await;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.pl_nav_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.pl_nav_next();
            }
            KeyCode::Enter => {
                if let Some(cluster) = self.pl_selected_cluster_ref() {
                    if !cluster.url.is_empty() && cluster.cluster_type != "disabled" {
                        // Switch to viewer mode with this cluster's URL
                        let url = self.pl_resolve_url(&cluster.url.clone());
                        self.pl_switch_to_viewer(&url).await;
                    } else {
                        // Show detail overlay for this cluster
                        self.parliament.show_detail = true;
                        self.parliament.detail_scroll = 0;
                    }
                }
            }
            KeyCode::Char('i') => {
                // Show detail overlay
                self.parliament.show_detail = true;
                self.parliament.detail_scroll = 0;
            }
            KeyCode::Char('c') => {
                if !self.parliament.cont3xt_url.is_empty() {
                    let url = self.parliament.cont3xt_url.clone();
                    self.pl_switch_to_cont3xt(&url).await;
                } else {
                    self.status_msg = "No Cont3xt URL configured in Parliament settings".into();
                }
            }
            KeyCode::Char('w') => {
                if !self.parliament.wise_url.is_empty() {
                    let url = self.parliament.wise_url.clone();
                    self.pl_switch_to_wise(&url).await;
                } else {
                    self.status_msg = "No WISE URL configured in Parliament settings".into();
                }
            }
            _ => {}
        }
    }

    async fn handle_issues_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Expression {
            return; // handled by expression handler
        }

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('r') => {
                self.pl_fetch_issues().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.parliament.issues_filter_edit = self.parliament.issues_filter.clone();
                self.input_mode = InputMode::Expression;
                self.expression_cursor = self.parliament.issues_filter_edit.len();
            }
            KeyCode::Char('s') => {
                self.parliament.issues_sort = self.parliament.issues_sort.next();
                self.pl_sort_issues();
            }
            KeyCode::Char('S') => {
                self.parliament.issues_sort_desc = !self.parliament.issues_sort_desc;
                self.pl_sort_issues();
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() {
                    self.parliament.issues_selected = self.parliament.issues_selected.saturating_sub(self.visible_rows);
                    self.parliament.issues_table_state.select(Some(self.parliament.issues_selected));
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let filtered = self.pl_filtered_issues();
                let max = filtered.len().saturating_sub(1);
                self.parliament.issues_selected = (self.parliament.issues_selected + self.visible_rows).min(max);
                self.parliament.issues_table_state.select(Some(self.parliament.issues_selected));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() && self.parliament.issues_selected > 0 {
                    self.parliament.issues_selected -= 1;
                    self.parliament.issues_table_state.select(Some(self.parliament.issues_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() && self.parliament.issues_selected < filtered.len() - 1 {
                    self.parliament.issues_selected += 1;
                    self.parliament.issues_table_state.select(Some(self.parliament.issues_selected));
                }
            }
            KeyCode::Home => {
                self.parliament.issues_selected = 0;
                self.parliament.issues_table_state.select(Some(0));
            }
            KeyCode::End => {
                let filtered = self.pl_filtered_issues();
                self.parliament.issues_selected = filtered.len().saturating_sub(1);
                self.parliament.issues_table_state.select(Some(self.parliament.issues_selected));
            }
            _ => {}
        }
    }

    fn pl_nav_next(&mut self) {
        if self.parliament.cluster_list.is_empty() {
            return;
        }
        let cur = self.pl_dashboard_nav_index();
        if cur + 1 < self.parliament.cluster_list.len() {
            let (gi, ci) = self.parliament.cluster_list[cur + 1];
            self.parliament.selected_group = gi;
            self.parliament.selected_cluster = ci;
        }
    }

    fn pl_nav_prev(&mut self) {
        if self.parliament.cluster_list.is_empty() {
            return;
        }
        let cur = self.pl_dashboard_nav_index();
        if cur > 0 {
            let (gi, ci) = self.parliament.cluster_list[cur - 1];
            self.parliament.selected_group = gi;
            self.parliament.selected_cluster = ci;
        }
    }

    fn pl_resolve_url(&self, url: &str) -> String {
        if url.starts_with('/') {
            if let Ok(parsed) = reqwest::Url::parse(self.client.base_url()) {
                let host = parsed.host_str().unwrap_or("");
                return if let Some(port) = parsed.port() {
                    format!("{}://{}:{}{}", parsed.scheme(), host, port, url)
                } else {
                    format!("{}://{}{}", parsed.scheme(), host, url)
                };
            }
        }
        url.to_string()
    }

    /// Common setup for switching from Parliament to another mode.
    /// Saves current client, connects to the new URL, handles auth.
    /// Returns false if connection failed (caller should abort).
    async fn pl_connect_to(&mut self, url: &str, label: &str) -> bool {
        self.parliament.saved_client = Some(self.client.clone());

        let mut new_client = self.client.clone_with_url(url);
        if new_client.ensure_session().await.is_err() {
            self.force_clear = true;
            if let Err(e) = prompt_credentials_and_login(&mut new_client, url).await {
                self.status_msg = format!("Failed to connect to {} at {}: {}", label, url, e);
                self.parliament.saved_client = None;
                return false;
            }
        }
        new_client.fetch_cookie().await.ok();

        self.status_msg = format!("Connected to {}: {}", label, url);
        self.http_log = new_client.http_log();
        self.client = new_client;
        self.force_clear = true;
        true
    }

    async fn pl_switch_to_viewer(&mut self, url: &str) {
        if !self.pl_connect_to(url, "cluster").await {
            return;
        }

        // Fetch cluster name for title bar
        self.title_name = url.to_string();
        if let Ok(health) = self.client.get_eshealth().await {
            if let Some(name) = health.get("cluster_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                self.title_name = name.to_string();
            }
        }

        self.app_mode = AppMode::Viewer;
        self.active_tab = Tab::Sessions;

        // Restore saved viewer expression
        self.expression = self.parliament.saved_viewer_expression.clone();
        self.expression_edit = self.expression.clone();

        self.vr_fetch_fields().await;
        self.vr_fetch_sessions().await;
    }

    pub(crate) async fn pl_return_to_parliament(&mut self) {
        if let Some(saved) = self.parliament.saved_client.take() {
            // Save current expression for this mode
            match self.app_mode {
                AppMode::Viewer => self.parliament.saved_viewer_expression = self.expression.clone(),
                AppMode::Cont3xt => self.parliament.saved_c3_expression = self.expression.clone(),
                _ => {}
            }
            self.expression.clear();
            self.expression_edit.clear();

            self.http_log = saved.http_log();
            self.client = saved;
            self.app_mode = AppMode::Parliament;
            self.active_tab = Tab::Dashboard;
            self.status_msg = "Returned to Parliament".into();
            self.force_clear = true;
            // Refresh data
            self.pl_fetch_data().await;
        }
    }

    async fn pl_switch_to_cont3xt(&mut self, url: &str) {
        if !self.pl_connect_to(url, "Cont3xt").await {
            return;
        }

        self.app_mode = AppMode::Cont3xt;
        self.active_tab = Tab::Search;

        // Restore saved cont3xt expression
        self.expression = self.parliament.saved_c3_expression.clone();
        self.expression_edit = self.expression.clone();

        self.c3_fetch_integrations().await;
        self.c3_fetch_views().await;
        self.c3_fetch_overviews().await;
        self.c3_fetch_link_groups().await;

        // Auto-search if there's a saved expression
        if !self.expression.is_empty() {
            self.c3_request_search();
        }
    }

    async fn pl_switch_to_wise(&mut self, url: &str) {
        if !self.pl_connect_to(url, "WISE").await {
            return;
        }

        self.app_mode = AppMode::Wise;
        self.active_tab = Tab::WsStats;

        self.ws_fetch_stats().await;
        self.ws_fetch_sources_types().await;
    }

    async fn handle_parliament_settings_key(&mut self, key: KeyEvent) {
        use crate::app::types::*;

        match self.parliament.settings_level {
            PlSettingsLevel::GroupEditor => {
                self.handle_pl_group_editor_key(key).await;
                return;
            }
            PlSettingsLevel::ClusterEditor => {
                self.handle_pl_cluster_editor_key(key).await;
                return;
            }
            _ => {}
        }

        match self.parliament.settings_tab {
            PlSettingsTab::Groups => self.handle_pl_groups_key(key).await,
            PlSettingsTab::General => self.handle_pl_general_key(key).await,
        }
    }

    async fn handle_pl_groups_key(&mut self, key: KeyEvent) {
        use crate::app::types::*;
        let items_len = self.parliament.settings_items.len();

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('1') => self.parliament.settings_tab = PlSettingsTab::Groups,
            KeyCode::Char('2') => self.parliament.settings_tab = PlSettingsTab::General,
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('D') => self.show_debug = !self.show_debug,
            KeyCode::Char('r') => self.pl_fetch_data().await,
            KeyCode::Down | KeyCode::Char('j') => {
                if items_len > 0 && self.parliament.settings_selected + 1 < items_len {
                    self.parliament.settings_selected += 1;
                    self.parliament.settings_table_state.select(Some(self.parliament.settings_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.parliament.settings_selected > 0 {
                    self.parliament.settings_selected -= 1;
                    self.parliament.settings_table_state.select(Some(self.parliament.settings_selected));
                }
            }
            KeyCode::Home => {
                self.parliament.settings_selected = 0;
                self.parliament.settings_table_state.select(Some(0));
            }
            KeyCode::End => {
                if items_len > 0 {
                    self.parliament.settings_selected = items_len - 1;
                    self.parliament.settings_table_state.select(Some(items_len - 1));
                }
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(&(gi, ci_opt)) = self.parliament.settings_items.get(self.parliament.settings_selected) {
                    match ci_opt {
                        None => self.pl_open_group_editor(gi),
                        Some(ci) => self.pl_open_cluster_editor(gi, ci),
                    }
                }
            }
            KeyCode::Char('n') => {
                // New group
                self.pl_open_new_group_editor();
            }
            KeyCode::Char('a') => {
                // Add cluster to selected group
                if let Some(&(gi, _)) = self.parliament.settings_items.get(self.parliament.settings_selected) {
                    self.pl_open_new_cluster_editor(gi);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.pl_delete_selected().await;
            }
            _ => {}
        }
    }

    async fn handle_pl_group_editor_key(&mut self, key: KeyEvent) {
        use crate::app::types::*;
        let p = &mut self.parliament;

        match key.code {
            KeyCode::Esc => {
                p.settings_level = PlSettingsLevel::GroupList;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pl_save_group().await;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                p.group_editor_field = match p.group_editor_field {
                    PlGroupEditorField::Title => PlGroupEditorField::Description,
                    PlGroupEditorField::Description => PlGroupEditorField::Title,
                };
            }
            _ => {
                let (text, cursor) = match p.group_editor_field {
                    PlGroupEditorField::Title => (&mut p.group_editor_title, &mut p.group_editor_title_cursor),
                    PlGroupEditorField::Description => (&mut p.group_editor_desc, &mut p.group_editor_desc_cursor),
                };
                handle_text_input_key(key.code, text, cursor);
            }
        }
    }

    async fn handle_pl_cluster_editor_key(&mut self, key: KeyEvent) {
        use crate::app::types::*;
        let p = &mut self.parliament;

        match key.code {
            KeyCode::Esc => {
                p.settings_level = PlSettingsLevel::GroupList;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pl_save_cluster().await;
            }
            KeyCode::Tab | KeyCode::Down if p.cluster_editor_field.is_bool() || key.code == KeyCode::Tab => {
                let idx = PlClusterEditorField::ALL.iter()
                    .position(|&f| f == p.cluster_editor_field).unwrap_or(0);
                let next = (idx + 1) % PlClusterEditorField::ALL.len();
                p.cluster_editor_field = PlClusterEditorField::ALL[next];
            }
            KeyCode::BackTab | KeyCode::Up if p.cluster_editor_field.is_bool() || key.code == KeyCode::BackTab => {
                let idx = PlClusterEditorField::ALL.iter()
                    .position(|&f| f == p.cluster_editor_field).unwrap_or(0);
                let prev = if idx == 0 { PlClusterEditorField::ALL.len() - 1 } else { idx - 1 };
                p.cluster_editor_field = PlClusterEditorField::ALL[prev];
            }
            KeyCode::Char(' ') if p.cluster_editor_field.is_bool() => {
                match p.cluster_editor_field {
                    PlClusterEditorField::HideDeltaBPS => p.cluster_editor_hide_delta_bps = !p.cluster_editor_hide_delta_bps,
                    PlClusterEditorField::HideDeltaTDPS => p.cluster_editor_hide_delta_tdps = !p.cluster_editor_hide_delta_tdps,
                    PlClusterEditorField::HideMonitoring => p.cluster_editor_hide_monitoring = !p.cluster_editor_hide_monitoring,
                    PlClusterEditorField::HideArkimeNodes => p.cluster_editor_hide_arkime_nodes = !p.cluster_editor_hide_arkime_nodes,
                    PlClusterEditorField::HideDataNodes => p.cluster_editor_hide_data_nodes = !p.cluster_editor_hide_data_nodes,
                    PlClusterEditorField::HideTotalNodes => p.cluster_editor_hide_total_nodes = !p.cluster_editor_hide_total_nodes,
                    _ => {}
                }
            }
            KeyCode::Enter if p.cluster_editor_field == PlClusterEditorField::Type => {
                // Cycle type: "" -> "multiviewer" -> "disabled" -> "noAlerts" -> ""
                p.cluster_editor_type = match p.cluster_editor_type.as_str() {
                    "" => "multiviewer".to_string(),
                    "multiviewer" => "disabled".to_string(),
                    "disabled" => "noAlerts".to_string(),
                    _ => String::new(),
                };
            }
            _ if !p.cluster_editor_field.is_bool() && p.cluster_editor_field != PlClusterEditorField::Type => {
                let (text, cursor) = match p.cluster_editor_field {
                    PlClusterEditorField::Title => (&mut p.cluster_editor_title, &mut p.cluster_editor_title_cursor),
                    PlClusterEditorField::Url => (&mut p.cluster_editor_url, &mut p.cluster_editor_url_cursor),
                    PlClusterEditorField::LocalUrl => (&mut p.cluster_editor_local_url, &mut p.cluster_editor_local_url_cursor),
                    PlClusterEditorField::Description => (&mut p.cluster_editor_desc, &mut p.cluster_editor_desc_cursor),
                    _ => return,
                };
                handle_text_input_key(key.code, text, cursor);
            }
            _ => {}
        }
    }

    async fn handle_pl_general_key(&mut self, key: KeyEvent) {
        use crate::app::types::*;
        let field_count = PlGeneralField::ALL.len();

        if self.parliament.general_editing {
            match key.code {
                KeyCode::Esc => {
                    self.parliament.general_editing = false;
                }
                KeyCode::Enter => {
                    let field = PlGeneralField::ALL[self.parliament.general_selected];
                    let value = self.parliament.general_edit_value.clone();
                    self.pl_set_general_field(&field, &value);
                    self.parliament.general_editing = false;
                }
                _ => {
                    handle_text_input_key(key.code, &mut self.parliament.general_edit_value, &mut self.parliament.general_edit_cursor);
                }
            }
            return;
        }

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('1') => self.parliament.settings_tab = PlSettingsTab::Groups,
            KeyCode::Char('2') => self.parliament.settings_tab = PlSettingsTab::General,
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('D') => self.show_debug = !self.show_debug,
            KeyCode::Down | KeyCode::Char('j') => {
                if self.parliament.general_selected + 1 < field_count {
                    self.parliament.general_selected += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.parliament.general_selected > 0 {
                    self.parliament.general_selected -= 1;
                }
            }
            KeyCode::Enter => {
                let field = PlGeneralField::ALL[self.parliament.general_selected];
                if field.is_select() {
                    // Toggle between "percentage" and "gb"
                    let current = self.pl_general_field_value(&field);
                    let new_val = if current == "percentage" { "gb" } else { "percentage" };
                    self.pl_set_general_field(&field, new_val);
                } else {
                    self.parliament.general_editing = true;
                    self.parliament.general_edit_value = self.pl_general_field_value(&field);
                    self.parliament.general_edit_cursor = self.parliament.general_edit_value.len();
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pl_save_general_settings().await;
            }
            KeyCode::Char('r') => self.pl_fetch_data().await,
            _ => {}
        }
    }
}
