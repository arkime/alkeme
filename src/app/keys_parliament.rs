use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    pub async fn handle_parliament_key(&mut self, key: KeyEvent) {
        match self.active_tab {
            Tab::Dashboard => self.handle_dashboard_key(key).await,
            Tab::Issues => self.handle_issues_key(key).await,
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
        if self.pl_show_detail {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.pl_show_detail = false;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.pl_detail_scroll = self.pl_detail_scroll.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.pl_detail_scroll += 10;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.pl_detail_scroll = self.pl_detail_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.pl_detail_scroll += 1;
                }
                KeyCode::Home | KeyCode::Left => self.pl_detail_scroll = 0,
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
                        let url = cluster.url.clone();
                        self.pl_switch_to_viewer(&url).await;
                    } else {
                        // Show detail overlay for this cluster
                        self.pl_show_detail = true;
                        self.pl_detail_scroll = 0;
                    }
                }
            }
            KeyCode::Char('i') => {
                // Show detail overlay
                self.pl_show_detail = true;
                self.pl_detail_scroll = 0;
            }
            KeyCode::Char('c') => {
                if !self.pl_cont3xt_url.is_empty() {
                    let url = self.pl_cont3xt_url.clone();
                    self.pl_switch_to_cont3xt(&url).await;
                } else {
                    self.status_msg = "No Cont3xt URL configured in Parliament settings".into();
                }
            }
            KeyCode::Char('w') => {
                if !self.pl_wise_url.is_empty() {
                    let url = self.pl_wise_url.clone();
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
                self.pl_issues_filter_edit = self.pl_issues_filter.clone();
                self.input_mode = InputMode::Expression;
                self.expression_cursor = self.pl_issues_filter_edit.len();
            }
            KeyCode::Char('s') => {
                self.pl_issues_sort = self.pl_issues_sort.next();
                self.pl_sort_issues();
            }
            KeyCode::Char('S') => {
                self.pl_issues_sort_desc = !self.pl_issues_sort_desc;
                self.pl_sort_issues();
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() {
                    self.pl_issues_selected = self.pl_issues_selected.saturating_sub(self.visible_rows);
                    self.pl_issues_table_state.select(Some(self.pl_issues_selected));
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let filtered = self.pl_filtered_issues();
                let max = filtered.len().saturating_sub(1);
                self.pl_issues_selected = (self.pl_issues_selected + self.visible_rows).min(max);
                self.pl_issues_table_state.select(Some(self.pl_issues_selected));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() && self.pl_issues_selected > 0 {
                    self.pl_issues_selected -= 1;
                    self.pl_issues_table_state.select(Some(self.pl_issues_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let filtered = self.pl_filtered_issues();
                if !filtered.is_empty() && self.pl_issues_selected < filtered.len() - 1 {
                    self.pl_issues_selected += 1;
                    self.pl_issues_table_state.select(Some(self.pl_issues_selected));
                }
            }
            KeyCode::Home => {
                self.pl_issues_selected = 0;
                self.pl_issues_table_state.select(Some(0));
            }
            KeyCode::End => {
                let filtered = self.pl_filtered_issues();
                self.pl_issues_selected = filtered.len().saturating_sub(1);
                self.pl_issues_table_state.select(Some(self.pl_issues_selected));
            }
            _ => {}
        }
    }

    fn pl_nav_next(&mut self) {
        if self.pl_cluster_list.is_empty() {
            return;
        }
        let cur = self.pl_dashboard_nav_index();
        if cur + 1 < self.pl_cluster_list.len() {
            let (gi, ci) = self.pl_cluster_list[cur + 1];
            self.pl_selected_group = gi;
            self.pl_selected_cluster = ci;
        }
    }

    fn pl_nav_prev(&mut self) {
        if self.pl_cluster_list.is_empty() {
            return;
        }
        let cur = self.pl_dashboard_nav_index();
        if cur > 0 {
            let (gi, ci) = self.pl_cluster_list[cur - 1];
            self.pl_selected_group = gi;
            self.pl_selected_cluster = ci;
        }
    }

    async fn pl_switch_to_viewer(&mut self, url: &str) {
        // Save parliament client for Ctrl+P return
        self.pl_saved_client = Some(self.client.clone());

        // Build a new client pointing to the cluster's URL
        let auth_mode = self.client.auth_mode();
        let mut new_client = crate::api::ArkimeClient::new(
            url,
            auth_mode,
            self.client.username(),
            self.client.password(),
        );
        if let Err(e) = new_client.login().await {
            self.status_msg = format!("Failed to connect to {}: {}", url, e);
            self.pl_saved_client = None;
            return;
        }
        new_client.fetch_cookie().await.ok();

        self.status_msg = format!("Connected to cluster: {}", url);
        self.http_log = new_client.http_log();
        self.client = new_client;

        // Switch mode
        self.app_mode = AppMode::Viewer;
        self.active_tab = Tab::Sessions;

        // Initialize viewer data
        self.vr_fetch_fields().await;
        self.vr_fetch_sessions().await;
    }

    pub(crate) async fn pl_return_to_parliament(&mut self) {
        if let Some(saved) = self.pl_saved_client.take() {
            self.http_log = saved.http_log();
            self.client = saved;
            self.app_mode = AppMode::Parliament;
            self.active_tab = Tab::Dashboard;
            self.status_msg = "Returned to Parliament".into();
            // Refresh data
            self.pl_fetch_data().await;
        }
    }

    async fn pl_switch_to_cont3xt(&mut self, url: &str) {
        // Save parliament client for Ctrl+P return
        self.pl_saved_client = Some(self.client.clone());

        let auth_mode = self.client.auth_mode();
        let mut new_client = crate::api::ArkimeClient::new(
            url,
            auth_mode,
            self.client.username(),
            self.client.password(),
        );
        if let Err(e) = new_client.login().await {
            self.status_msg = format!("Failed to connect to Cont3xt at {}: {}", url, e);
            self.pl_saved_client = None;
            return;
        }
        new_client.fetch_cookie().await.ok();

        self.status_msg = format!("Connected to Cont3xt: {}", url);
        self.http_log = new_client.http_log();
        self.client = new_client;

        // Switch mode
        self.app_mode = AppMode::Cont3xt;
        self.active_tab = Tab::Search;

        // Initialize cont3xt data
        self.c3_fetch_integrations().await;
        self.c3_fetch_views().await;
        self.c3_fetch_link_groups().await;
    }

    async fn pl_switch_to_wise(&mut self, url: &str) {
        // Save parliament client for Ctrl+P return
        self.pl_saved_client = Some(self.client.clone());

        let auth_mode = self.client.auth_mode();
        let mut new_client = crate::api::ArkimeClient::new(
            url,
            auth_mode,
            self.client.username(),
            self.client.password(),
        );
        // WISE may not require auth — try login but don't fail
        if let Err(e) = new_client.login().await {
            // Try without auth
            new_client = crate::api::ArkimeClient::new(
                url,
                crate::api::AuthMode::None,
                None,
                None,
            );
            self.status_msg = format!("WISE auth failed ({}), trying without auth", e);
        }
        new_client.fetch_cookie().await.ok();

        self.status_msg = format!("Connected to WISE: {}", url);
        self.http_log = new_client.http_log();
        self.client = new_client;

        // Switch mode
        self.app_mode = AppMode::Wise;
        self.active_tab = Tab::WsStats;

        // Initialize WISE data
        self.ws_fetch_stats().await;
        self.ws_fetch_sources_types().await;
    }
}
