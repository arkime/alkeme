mod types;
mod keys;
mod keys_shared;
mod keys_viewer;
mod keys_cont3xt;
mod keys_cont3xt_settings;
mod keys_parliament;
mod keys_wise;
mod keys_users;
mod viewer;
mod cont3xt;
mod cont3xt_settings;
mod parliament;
mod wise;

pub use types::*;

use crate::api::{ArkimeClient, HttpLog};
use chrono::{Datelike, Duration, Timelike, Utc};
use crossterm::event::KeyCode;
use serde_json::Value;
use std::collections::HashMap;

/// Handle common text input key events (char insert, backspace, cursor movement).
/// Returns true if the key was handled.
pub fn handle_text_input_key(key: KeyCode, text: &mut String, cursor: &mut usize) -> bool {
    match key {
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                text.remove(*cursor);
            }
            true
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        KeyCode::Right => {
            *cursor = (*cursor + 1).min(text.len());
            true
        }
        KeyCode::Char(c) => {
            text.insert(*cursor, c);
            *cursor += 1;
            true
        }
        _ => false,
    }
}

pub struct App {
    pub client: ArkimeClient,
    pub app_mode: AppMode,
    pub title_name: String,
    pub user: Value,
    pub active_tab: Tab,
    pub time_range: TimeRange,
    pub time_ranges: Vec<TimeRange>,
    pub expression: String,
    pub expression_edit: String,
    pub expression_cursor: usize,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub confirm_dialog: Option<ConfirmDialog>,
    pub action_menu: Option<ActionMenu>,
    pub action_prompt: Option<ActionPrompt>,
    pub viewer: ViewerState,
    pub show_loading: bool,
    pub loading_owl_x: u16,
    pub loading_owl_dx: i16,
    pub loading_owl_tick: std::time::Instant,
    pub status_msg: String,
    pub show_debug: bool,
    pub debug_scroll: usize,
    pub debug_selected: usize,
    pub debug_expanded: bool,
    pub http_log: HttpLog,
    // Stats tab state
    pub visible_rows: usize,
    // Files tab state
    // Owl animation
    pub owl_x: f32,
    pub owl_y: f32,
    pub owl_dx: f32,
    pub owl_dy: f32,
    pub owl_frame: usize,
    pub owl_tick: std::time::Instant,
    pub anim_start: std::time::Instant,
    // Cont3xt state
    pub cont3xt: Cont3xtState,
    // Parliament state
    pub parliament: ParliamentState,
    pub force_clear: bool, // force terminal clear after okta redirect

    // WISE mode state
    pub wise: WiseState,

    // Cached background buffer for popup double-buffering
    pub popup_bg_cache: Option<ratatui::buffer::Buffer>,
    pub visible_tabs: Vec<Tab>,

    // Users tab state
    pub us_users: Vec<Value>,
    pub us_total: usize,
    pub us_filtered: usize,
    pub us_selected: usize,
    pub us_table_state: ratatui::widgets::TableState,
    pub us_filter: String,
    pub us_sort_field: String,
    pub us_sort_desc: bool,
    pub us_page_start: usize,
    pub us_page_size: usize,
    pub us_needs_fetch: bool,
    // User editor state
    pub us_editing: bool,
    pub us_editor_user: Value,
    pub us_editor_field: usize,
    pub us_editor_text: String,
    pub us_editor_cursor: usize,
    // Role popup state (within user editor)
    pub us_all_roles: Vec<String>,
    pub us_role_popup_open: bool,
    pub us_role_popup_selected: usize,
    pub us_role_popup_filter: String,
    pub us_role_popup_cursor: usize,
    pub us_role_popup_filtering: bool,
    pub us_editor_roles: Vec<(String, bool)>,
    // Create user/role state
    pub us_creating: bool, // true = creating new user/role, false = editing existing
}

impl App {
    pub fn new(base_url: &str, auth_mode: crate::api::AuthMode, username: Option<String>, password: Option<String>, app_mode: AppMode) -> Self {
        let client = ArkimeClient::new(base_url, auth_mode, username, password, None);
        let http_log = client.http_log();
        let active_tab = app_mode.default_tab();
        Self {
            client,
            app_mode,
            title_name: base_url.to_string(),
            user: Value::Null,
            active_tab,
            time_range: TimeRange::Hours1,
            time_ranges: TimeRange::defaults(),
            expression: String::new(),
            expression_edit: String::new(),
            expression_cursor: 0,
            input_mode: InputMode::Normal,
            show_help: false,
            confirm_dialog: None,
            action_menu: None,
            action_prompt: None,
            viewer: ViewerState::default(),
            show_loading: false,
            loading_owl_x: 0,
            loading_owl_dx: 1,
            loading_owl_tick: std::time::Instant::now(),
            status_msg: String::new(),
            show_debug: false,
            debug_scroll: 0,
            debug_selected: 0,
            debug_expanded: false,
            http_log,
            visible_rows: 20,
            // Owl animation
            owl_x: 5.0,
            owl_y: 3.0,
            owl_dx: 1.0,
            owl_dy: 0.5,
            owl_frame: 0,
            owl_tick: std::time::Instant::now(),
            anim_start: std::time::Instant::now(),
            // Cont3xt state
            cont3xt: Cont3xtState::default(),
            // Parliament state
            parliament: ParliamentState::default(),
            force_clear: false,

            // WISE state
            wise: WiseState::default(),

            popup_bg_cache: None,
            visible_tabs: app_mode.tabs().to_vec(),

            // Users tab state
            us_users: Vec::new(),
            us_total: 0,
            us_filtered: 0,
            us_selected: 0,
            us_table_state: ratatui::widgets::TableState::default(),
            us_filter: String::new(),
            us_sort_field: "userId".to_string(),
            us_sort_desc: false,
            us_page_start: 0,
            us_page_size: 100,
            us_needs_fetch: true,
            us_editing: false,
            us_editor_user: Value::Null,
            us_editor_field: 0,
            us_editor_text: String::new(),
            us_editor_cursor: 0,
            us_all_roles: Vec::new(),
            us_role_popup_open: false,
            us_role_popup_selected: 0,
            us_role_popup_filter: String::new(),
            us_role_popup_cursor: 0,
            us_role_popup_filtering: false,
            us_editor_roles: Vec::new(),
            us_creating: false,
        }
    }

    pub fn is_detail_view(&self) -> bool {
        self.viewer.session_view == SessionView::Detail || self.viewer.stats_view == StatsView::Detail
    }

    pub fn needs_animation(&self) -> bool {
        if self.show_loading {
            return true;
        }
        match self.app_mode {
            AppMode::Viewer => self.active_tab == Tab::Settings,
            AppMode::Cont3xt => {
                self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews
            }
            AppMode::Wise => self.active_tab == Tab::Settings,
            AppMode::Parliament => self.active_tab == Tab::Settings,
        }
    }

    /// Returns true if any popup overlay is open that could use background caching
    pub fn has_popup_open(&self) -> bool {
        self.confirm_dialog.is_some()
            || self.us_editing
            || self.us_creating
            || self.us_role_popup_open
            || self.cont3xt.show_card_popup
            || self.cont3xt.show_overview_popup
            || self.cont3xt.show_link_popup
            || self.cont3xt.show_integration_popup
            || self.cont3xt.show_tags_popup
            || self.cont3xt.show_date_popup
            || self.cont3xt.save_json_prompt.is_some()
            || self.cont3xt.backup_prompt.is_some()
            || self.cont3xt.view_editor_open
            || self.cont3xt.role_popup_open
            || self.cont3xt.int_editor_open
            || self.cont3xt.ov_fe_popup_open
            || self.show_help
            || self.show_debug
    }

    /// Returns true if q should close a popup instead of quitting the app
    pub fn q_closes_popup(&self) -> bool {
        self.confirm_dialog.is_some()
            || self.us_editing || self.us_creating || self.us_role_popup_open
            || self.show_help || self.show_debug || self.parliament.show_detail
            || self.viewer.show_column_editor || self.viewer.show_layout_popup || self.viewer.show_view_popup
            || self.viewer.stats_show_column_editor || self.viewer.stats_show_layout_popup
            || self.cont3xt.show_integration_popup || self.cont3xt.show_overview_popup
            || self.cont3xt.show_link_popup || self.cont3xt.show_card_popup
            || self.cont3xt.show_tags_popup || self.cont3xt.show_date_popup
            || self.cont3xt.view_editor_open || self.cont3xt.role_popup_open
            || self.cont3xt.int_editor_open
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.visible_tabs
    }

    pub fn next_tab(&mut self) {
        let tabs = &self.visible_tabs;
        let idx = tabs.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = &self.visible_tabs;
        let idx = tabs.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }

    pub async fn handle_confirm(&mut self, action: String) {
        if let Some(index) = action.strip_prefix("delete_esindex:") {
            match self.client.vr_delete_esindex(index).await {
                Ok(_) => {
                    self.status_msg = format!("Deleted index '{index}'");
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error deleting index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("optimize_esindex:") {
            match self.client.vr_esindex_action(index, "optimize").await {
                Ok(_) => {
                    self.status_msg = format!("Force merge started for '{index}'");
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error force merging index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("close_esindex:") {
            match self.client.vr_esindex_action(index, "close").await {
                Ok(_) => {
                    self.status_msg = format!("Closed index '{index}'");
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error closing index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("open_esindex:") {
            match self.client.vr_esindex_action(index, "open").await {
                Ok(_) => {
                    self.status_msg = format!("Opened index '{index}'");
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error opening index: {e}"),
            }
        } else if let Some(rest) = action.strip_prefix("esshards:") {
            // format: "esshards:kind:value:action" where action is "exclude" or "include"
            // Use rfind to handle IPv6 colons in value
            if let Some(last_colon) = rest.rfind(':') {
                let op = &rest[last_colon+1..];
                let before_op = &rest[..last_colon];
                if let Some(first_colon) = before_op.find(':') {
                    let kind = &before_op[..first_colon];
                    let value = &before_op[first_colon+1..];
                    match self.client.vr_esshards_toggle(kind, value, op).await {
                        Ok(_) => {
                            let label = if kind == "ip" { "IP" } else { "node" };
                            self.status_msg = format!("{} {label} '{value}'", if op == "exclude" { "Excluded" } else { "Included" });
                            let detail_name = self.viewer.stats_detail.as_ref()
                                .map(|d| crate::api::str_val(&d.data, "name"));
                            let detail_scroll = self.viewer.stats_detail.as_ref().map(|d| d.scroll).unwrap_or(0);
                            let detail_filter = self.viewer.stats_detail.as_ref().map(|d| d.filter.clone()).unwrap_or_default();
                            let detail_filter_cursor = self.viewer.stats_detail.as_ref().map(|d| d.filter_cursor).unwrap_or(0);
                            self.vr_fetch_stats().await;
                            if let Some(name) = detail_name {
                                if let Some(row) = self.viewer.stats_data.iter().find(|r| crate::api::str_val(r, "name") == name) {
                                    self.viewer.stats_detail = Some(StatsDetail { data: row.clone(), scroll: detail_scroll, filter: detail_filter, filter_cursor: detail_filter_cursor });
                                }
                            }
                        }
                        Err(e) => self.status_msg = format!("Error: {e}"),
                    }
                }
            }
        } else if let Some(task_id) = action.strip_prefix("cancel_estask:") {
            match self.client.vr_cancel_estask(task_id).await {
                Ok(_) => {
                    self.status_msg = format!("Cancel requested for task '{task_id}'");
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error cancelling task: {e}"),
            }
        } else if action == "cancel_all_estasks" {
            match self.client.vr_cancel_all_estasks().await {
                Ok(_) => {
                    self.status_msg = "Cancel requested for all cancellable tasks".into();
                    self.vr_request_stats_fetch();
                }
                Err(e) => self.status_msg = format!("Error cancelling tasks: {e}"),
            }
        } else if let Some(user_id) = action.strip_prefix("delete_user:") {
            let prefix = self.us_api_prefix().to_string();
            match self.client.delete_user(&prefix, user_id).await {
                Ok(msg) => {
                    self.status_msg = msg;
                    self.us_fetch_users().await;
                }
                Err(e) => self.status_msg = format!("Error deleting: {e}"),
            }
        }
    }

    pub async fn fetch_user(&mut self) {
        let api_prefix = match self.app_mode {
            AppMode::Parliament => "/parliament/api",
            _ => "/api",
        };
        match self.client.get_user(api_prefix).await {
            Ok(user) => {
                self.user = user;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching user: {e}");
            }
        }
        self.rebuild_visible_tabs();
    }

    pub fn rebuild_visible_tabs(&mut self) {
        let mut tabs = self.app_mode.tabs().to_vec();
        let roles = self.user.get("roles");
        let has_users_admin = roles
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some("usersAdmin")))
            .unwrap_or(false);
        if has_users_admin && self.app_mode != AppMode::Wise {
            tabs.push(Tab::Users);
        }
        self.visible_tabs = tabs;
    }

    pub fn enter_expression_mode(&mut self) {
        self.expression_edit = self.expression.clone();
        self.expression_cursor = self.expression_edit.len();
        self.input_mode = InputMode::Expression;
    }

    pub fn time_range_next(&mut self) {
        let idx = self.time_ranges.iter().position(|t| t == &self.time_range).unwrap_or(0);
        self.time_range = self.time_ranges[(idx + 1) % self.time_ranges.len()].clone();
    }

    pub fn time_range_prev(&mut self) {
        let idx = self.time_ranges.iter().position(|t| t == &self.time_range).unwrap_or(0);
        let len = self.time_ranges.len();
        self.time_range = self.time_ranges[(idx + len - 1) % len].clone();
    }

    // --- Users tab methods ---

    pub fn us_api_prefix(&self) -> &str {
        match self.app_mode {
            AppMode::Parliament => "/parliament/api",
            _ => "/api",
        }
    }

    pub async fn us_fetch_users(&mut self) {
        let prefix = self.us_api_prefix().to_string();
        let sort_field = self.us_sort_field.clone();
        let filter = self.us_filter.clone();
        match self.client.get_users(&prefix, &sort_field, self.us_sort_desc, &filter, self.us_page_start, self.us_page_size).await {
            Ok(val) => {
                self.us_users = val.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();
                self.us_total = val.get("recordsTotal").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                self.us_filtered = val.get("recordsFiltered").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                if self.us_selected >= self.us_users.len() && !self.us_users.is_empty() {
                    self.us_selected = self.us_users.len() - 1;
                }
            }
            Err(e) => {
                self.status_msg = format!("Error fetching users: {e}");
            }
        }
    }

    pub async fn us_save_user(&mut self) {
        let user_id = self.us_editor_user.get("userId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if user_id.is_empty() { return; }
        let prefix = self.us_api_prefix().to_string();
        if self.us_creating {
            let data = self.us_editor_user.clone();
            match self.client.create_user(&prefix, &data).await {
                Ok(msg) => {
                    self.status_msg = msg;
                    self.us_editing = false;
                    self.us_creating = false;
                    self.us_fetch_users().await;
                }
                Err(e) => {
                    self.status_msg = format!("Error creating: {e}");
                }
            }
        } else {
            let mut data = self.us_editor_user.clone();
            if let Some(obj) = data.as_object_mut() {
                obj.remove("userId");
            }
            match self.client.update_user(&prefix, &user_id, &data).await {
                Ok(msg) => {
                    self.status_msg = msg;
                    self.us_editing = false;
                    self.us_fetch_users().await;
                }
                Err(e) => {
                    self.status_msg = format!("Error saving user: {e}");
                }
            }
        }
    }

    pub fn us_start_create(&mut self, is_role: bool) {
        self.us_editor_user = serde_json::json!({
            "userId": if is_role { "role:" } else { "" },
            "userName": "",
            "password": "",
            "enabled": true,
            "webEnabled": false,
            "headerAuthEnabled": false,
            "emailSearch": false,
            "removeEnabled": false,
            "packetSearch": false,
            "hideStats": false,
            "hideFiles": false,
            "hidePcap": false,
            "disablePcapDownload": false,
            "expression": "",
            "timeLimit": 0,
            "roles": [],
        });
        self.us_editor_field = 0; // start on userId
        self.us_editing = true;
        self.us_creating = true;
        self.us_load_editor_field();
    }

    pub fn us_editor_fields(&self) -> Vec<(&'static str, &'static str)> {
        let mut fields = vec![
            ("userId", "text"),
            ("userName", "text"),
        ];
        if self.us_creating {
            fields.push(("password", "text"));
        }
        fields.extend_from_slice(&[
            ("enabled", "bool"),
            ("webEnabled", "bool"),
            ("headerAuthEnabled", "bool"),
            ("emailSearch", "bool"),
            ("removeEnabled", "bool"),
            ("packetSearch", "bool"),
            ("hideStats", "bool"),
            ("hideFiles", "bool"),
            ("hidePcap", "bool"),
            ("disablePcapDownload", "bool"),
            ("expression", "text"),
            ("timeLimit", "text"),
            ("roles", "roles"),
        ]);
        fields
    }

    pub fn us_commit_editor_field(&mut self) {
        let fields = self.us_editor_fields();
        if self.us_editor_field >= fields.len() { return; }
        let (field_name, field_type) = fields[self.us_editor_field];
        if field_type == "bool" || field_type == "roles" { return; }
        // userId is editable when creating, readonly when editing
        if field_name == "userId" && !self.us_creating { return; }

        if field_name == "timeLimit" {
            if let Ok(n) = self.us_editor_text.parse::<u64>() {
                self.us_editor_user["timeLimit"] = Value::Number(n.into());
            }
        } else {
            self.us_editor_user[field_name] = Value::String(self.us_editor_text.clone());
        }
    }

    pub fn us_load_editor_field(&mut self) {
        let fields = self.us_editor_fields();
        if self.us_editor_field >= fields.len() { return; }
        let (field_name, field_type) = fields[self.us_editor_field];
        if field_type == "bool" || field_type == "roles" {
            self.us_editor_text.clear();
            self.us_editor_cursor = 0;
            return;
        }

        if field_name == "timeLimit" {
            self.us_editor_text = self.us_editor_user.get("timeLimit")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_default();
        } else {
            self.us_editor_text = self.us_editor_user.get(field_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
        self.us_editor_cursor = self.us_editor_text.len();
    }

    /// Commit role selections from us_editor_roles back to us_editor_user
    pub fn us_commit_roles(&mut self) {
        let roles: Vec<Value> = self.us_editor_roles.iter()
            .filter(|(_, sel)| *sel)
            .map(|(name, _)| Value::String(name.clone()))
            .collect();
        self.us_editor_user["roles"] = Value::Array(roles);
    }

    /// Build us_editor_roles from us_all_roles + current user's roles
    pub fn us_build_editor_roles(&mut self) {
        let user_roles: std::collections::HashSet<String> = self.us_editor_user.get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        self.us_editor_roles = self.us_all_roles.iter()
            .map(|r| (r.clone(), user_roles.contains(r)))
            .collect();
    }

    /// Filtered indices into us_all_roles for role popup
    pub fn us_role_popup_filtered(&self) -> Vec<usize> {
        let filter = self.us_role_popup_filter.to_lowercase();
        self.us_editor_roles.iter().enumerate()
            .filter(|(_, (name, _))| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    pub async fn us_fetch_roles(&mut self) {
        let prefix = self.us_api_prefix().to_string();
        match self.client.get_roles(&prefix).await {
            Ok(roles) => {
                self.us_all_roles = roles;
            }
            Err(e) => self.status_msg = format!("Error fetching roles: {e}"),
        }
    }
}

/// Decode percent-encoded characters in a URL (e.g., %20 → space, %22 → ").
/// Used on macOS because `open` re-encodes the URL, causing double-encoding.
#[cfg(target_os = "macos")]
pub fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_val(h), hex_val(l)) {
                    out.push((hv << 4 | lv) as char);
                    continue;
                }
                // Not valid hex, keep as-is
                out.push('%');
                out.push(h as char);
                out.push(l as char);
            } else {
                out.push('%');
                if let Some(h) = hi { out.push(h as char); }
            }
        } else {
            out.push(b as char);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse a date input string (Splunk-style relative or absolute) into a DateTime<Utc>.
/// Supports: "now", relative like "-5h", "+1d", "-1w", "-3M", "-1y", "@day",
/// "+2h@day", and absolute ISO 8601 / YYYY/MM/DDTHH:mm:ss formats.
fn parse_date_input(s: &str) -> Option<chrono::DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() || s == "now" {
        return Some(Utc::now());
    }

    // Relative: +/-N unit[@snap]
    if s.starts_with('+') || s.starts_with('-') {
        let sign: i64 = if s.starts_with('-') { -1 } else { 1 };
        let rest = &s[1..];

        // Split on @ for optional snap
        let (time_part, _snap_part) = if let Some(at) = rest.find('@') {
            (&rest[..at], Some(&rest[at + 1..]))
        } else {
            (rest, None)
        };

        // Parse number + unit
        let num_end = time_part.find(|c: char| !c.is_ascii_digit()).unwrap_or(time_part.len());
        let num: i64 = if num_end == 0 { 1 } else { time_part[..num_end].parse().ok()? };
        let unit = &time_part[num_end..];

        let dur = match unit {
            "s" | "sec" | "secs" | "second" | "seconds" => Duration::seconds(num),
            "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(num),
            "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(num),
            "d" | "day" | "days" => Duration::days(num),
            "w" | "week" | "weeks" => Duration::weeks(num),
            "M" | "mon" | "mons" | "month" | "months" => Duration::days(num * 30),
            "q" | "qtr" | "qtrs" | "quarter" | "quarters" => Duration::days(num * 91),
            "y" | "yr" | "yrs" | "year" | "years" => Duration::days(num * 365),
            _ => return None,
        };

        return Some(Utc::now() + dur * sign as i32);
    }

    // @snap only (e.g., "@day" = start of today)
    if s.starts_with('@') {
        let snap = &s[1..];
        let now = Utc::now();
        return match snap {
            "s" | "sec" | "second" | "seconds" => Some(now.with_nanosecond(0)?),
            "m" | "min" | "minute" | "minutes" => Some(now.with_nanosecond(0)?.with_second(0)?),
            "h" | "hr" | "hour" | "hours" => Some(now.with_nanosecond(0)?.with_second(0)?.with_minute(0)?),
            "d" | "day" | "days" => Some(now.date_naive().and_hms_opt(0, 0, 0)?.and_utc()),
            "w" | "week" | "weeks" => {
                let weekday = now.weekday().num_days_from_sunday();
                Some((now.date_naive() - Duration::days(weekday as i64)).and_hms_opt(0, 0, 0)?.and_utc())
            },
            "M" | "mon" | "month" | "months" => {
                Some(now.date_naive().with_day(1)?.and_hms_opt(0, 0, 0)?.and_utc())
            },
            "y" | "yr" | "year" | "years" => {
                Some(chrono::NaiveDate::from_ymd_opt(now.year(), 1, 1)?.and_hms_opt(0, 0, 0)?.and_utc())
            },
            _ => None,
        };
    }

    // Absolute: try ISO 8601 and YYYY/MM/DDTHH:mm:ss
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y/%m/%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }

    None
}

/// Refang an indicator (reverse of defang: hXXp→http, [.]→.)
fn refang(s: &str) -> String {
    s.replace("hXXp", "http").replace("[.]", ".")
}

/// Convert a YYYY-MM-DD style format string to chrono strftime format
fn convert_date_format(fmt: &str) -> String {
    fmt.replace("YYYY", "%Y")
        .replace("YY", "%y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("hh", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

/// Parse a timeSnap string like "1d", "-1w", "2h" into a chrono Duration
fn parse_time_snap(snap: &str) -> Option<Duration> {
    let snap = snap.trim();
    if snap.is_empty() { return None; }
    let (negative, rest) = if let Some(s) = snap.strip_prefix('-') { (true, s) } else { (false, snap) };
    let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let num: i64 = rest[..num_end].parse().ok()?;
    let unit = &rest[num_end..];
    let dur = match unit {
        "s" => Duration::seconds(num),
        "m" => Duration::minutes(num),
        "h" => Duration::hours(num),
        "d" => Duration::days(num),
        "w" => Duration::weeks(num),
        _ => return None,
    };
    Some(if negative { -dur } else { dur })
}

/// Substitute all placeholders in a link URL
fn substitute_link_url(
    url: &str,
    indicator: &str,
    itype: &str,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    indicators_by_itype: &HashMap<String, Vec<String>>,
    top_indicators_by_itype: &HashMap<String, Vec<String>>,
) -> String {
    let refanged = refang(indicator);
    let num_days = (end - start).num_days();
    let num_hours = (end - start).num_hours();

    // Simple placeholders
    let result = url
        .replace("${indicator}", &refanged)
        .replace("${type}", itype)
        .replace("${numDays}", &num_days.to_string())
        .replace("${numHours}", &num_hours.to_string())
        .replace("${startDate}", &start.format("%Y-%m-%d").to_string())
        .replace("${endDate}", &end.format("%Y-%m-%d").to_string())
        .replace("${startTS}", &start.format("%Y-%m-%dT%H.%M.%SZ").to_string())
        .replace("${endTS}", &end.format("%Y-%m-%dT%H.%M.%SZ").to_string())
        .replace("${startEpoch}", &start.timestamp().to_string())
        .replace("${endEpoch}", &end.timestamp().to_string())
        .replace("${startSplunk}", &start.format("%m/%d/%Y:%H:%M:%S").to_string())
        .replace("${endSplunk}", &end.format("%m/%d/%Y:%H:%M:%S").to_string());

    // ${start,...}, ${end,...}, and ${array,...} — scan for remaining ${ placeholders
    let mut out = String::with_capacity(result.len());
    let mut rest = result.as_str();
    while let Some(pos) = rest.find("${") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 2..];
        // Find matching closing brace, accounting for nested JSON braces and escaped chars
        if let Some(close) = find_matching_brace(after) {
            let inner = &after[..close];
            if let Some(replacement) = process_advanced_placeholder(inner, start, end, indicators_by_itype, top_indicators_by_itype) {
                out.push_str(&replacement);
            }
            // On parse failure, placeholder is removed
            rest = &after[close + 1..];
        } else {
            out.push_str("${");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Find the position of the closing `}` for a `${...}` placeholder,
/// accounting for nested JSON braces and backslash-escaped characters.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1; // we're already inside the opening ${
    let mut chars = s.char_indices();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '\\' => { chars.next(); } // skip escaped char
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

/// Process ${start,...}, ${end,...}, and ${array,...} placeholders
fn process_advanced_placeholder(
    inner: &str,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    indicators_by_itype: &HashMap<String, Vec<String>>,
    top_indicators_by_itype: &HashMap<String, Vec<String>>,
) -> Option<String> {
    let comma_pos = inner.find(',')?;
    let keyword = inner[..comma_pos].trim();
    let json_str = inner[comma_pos + 1..].trim();

    match keyword {
        "start" | "end" => {
            let obj: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let fmt = obj.get("format").and_then(|v| v.as_str()).unwrap_or("YYYY-MM-DD");
            let chrono_fmt = convert_date_format(fmt);
            let mut dt = if keyword == "start" { start } else { end };
            if let Some(snap_str) = obj.get("timeSnap").and_then(|v| v.as_str()) {
                if let Some(dur) = parse_time_snap(snap_str) {
                    dt = dt + dur;
                }
            }
            Some(dt.format(&chrono_fmt).to_string())
        }
        "array" => {
            let obj: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let target_itype = obj.get("iType").and_then(|v| v.as_str())?;
            let include = obj.get("include").and_then(|v| v.as_str()).unwrap_or("all");
            let sep = obj.get("sep").and_then(|v| v.as_str()).unwrap_or(",");
            let quote = obj.get("quote").and_then(|v| v.as_str()).unwrap_or("");

            let source = if include == "top" { top_indicators_by_itype } else { indicators_by_itype };
            let values = source.get(target_itype)?;
            let formatted: Vec<String> = values.iter()
                .map(|v| format!("{quote}{v}{quote}"))
                .collect();
            Some(formatted.join(sep))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_array_with_quotes_and_sep() {
        let mut all = HashMap::new();
        all.insert("domain".to_string(), vec!["threathole.com".to_string(), "threattroll.com".to_string()]);
        let top = all.clone();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = r#"str_representation:%20(%20${array,{"iType":"domain","include":"all","sep":" OR ","quote":"\""}}%20)"#;
        let result = substitute_link_url(url, "threathole.com", "domain", start, now, &all, &top);
        assert_eq!(result, r#"str_representation:%20(%20"threathole.com" OR "threattroll.com"%20)"#);
    }

    #[test]
    fn test_substitute_simple_placeholders() {
        let all = HashMap::new();
        let top = HashMap::new();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = "https://example.com?q=${indicator}&t=${type}&d=${numDays}&h=${numHours}";
        let result = substitute_link_url(url, "test[.]com", "domain", start, now, &all, &top);
        assert_eq!(result, "https://example.com?q=test.com&t=domain&d=7&h=168");
    }

    #[test]
    fn test_substitute_custom_date_format() {
        let all = HashMap::new();
        let top = HashMap::new();
        let end = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:00Z").unwrap().with_timezone(&Utc);
        let start = end - Duration::days(7);

        let url = r#"https://example.com?s=${start,{"format":"DD.MM.YYYY"}}&e=${end,{"format":"YYYY-MM-DDThh:mm:ssZ"}}"#;
        let result = substitute_link_url(url, "test", "ip", start, end, &all, &top);
        assert_eq!(result, "https://example.com?s=08.06.2024&e=2024-06-15T12:30:00Z");
    }

    #[test]
    fn test_find_matching_brace() {
        assert_eq!(find_matching_brace("simple}"), Some(6));
        assert_eq!(find_matching_brace(r#"array,{"iType":"ip"}}"#), Some(20));
        assert_eq!(find_matching_brace(r#"array,{"quote":"\""}}"#), Some(20));
        assert_eq!(find_matching_brace("no close"), None);
    }

    #[test]
    fn test_refang() {
        assert_eq!(refang("hXXps://evil[.]com"), "https://evil.com");
        assert_eq!(refang("normal.com"), "normal.com");
    }

    #[test]
    fn test_substitute_array_full_url() {
        let mut all = HashMap::new();
        all.insert("domain".to_string(), vec!["threathole.com".to_string(), "threattroll.com".to_string()]);
        let top = all.clone();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = r#"https://HOST:9999/app/dashboards#/view/123?_a=(query:(query:'str_representation:%20(%20${array,{"iType":"domain","include":"all","sep":" OR ","quote":"\""}}%20)'))"#;
        let result = substitute_link_url(url, "threathole.com", "domain", start, now, &all, &top);
        // %20 in template preserved as-is, quotes and spaces from sep are literal
        assert_eq!(result, r#"https://HOST:9999/app/dashboards#/view/123?_a=(query:(query:'str_representation:%20(%20"threathole.com" OR "threattroll.com"%20)'))"#);
    }

    #[test]
    fn test_parse_time_snap() {
        assert_eq!(parse_time_snap("1d"), Some(Duration::days(1)));
        assert_eq!(parse_time_snap("-1w"), Some(Duration::weeks(-1)));
        assert_eq!(parse_time_snap("2h"), Some(Duration::hours(2)));
        assert_eq!(parse_time_snap(""), None);
    }

    #[test]
    fn test_parse_date_input() {
        // "now" should be close to current time
        let now = Utc::now();
        let parsed = parse_date_input("now").unwrap();
        assert!((parsed - now).num_seconds().abs() < 2);

        // Empty string = now
        let parsed = parse_date_input("").unwrap();
        assert!((parsed - now).num_seconds().abs() < 2);

        // Relative: -7d
        let parsed = parse_date_input("-7d").unwrap();
        let expected = now - Duration::days(7);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Relative: -1h
        let parsed = parse_date_input("-1h").unwrap();
        let expected = now - Duration::hours(1);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Relative: +30m
        let parsed = parse_date_input("+30m").unwrap();
        let expected = now + Duration::minutes(30);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Absolute ISO 8601
        let parsed = parse_date_input("2024-06-15T12:30:00Z").unwrap();
        assert_eq!(parsed.year(), 2024);
        assert_eq!(parsed.month(), 6);
        assert_eq!(parsed.day(), 15);

        // Absolute date only
        let parsed = parse_date_input("2024-01-01").unwrap();
        assert_eq!(parsed.year(), 2024);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 1);
        assert_eq!(parsed.hour(), 0);

        // Invalid
        assert!(parse_date_input("garbage").is_none());
        assert!(parse_date_input("-7z").is_none());
    }
}
