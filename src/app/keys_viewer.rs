use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;
use super::keys_shared::*;

impl App {
    pub(crate) fn open_action_menu(&mut self, target: ActionTarget) {
        let (session_id, session_node) = match target {
            ActionTarget::Single => {
                let (id, node) = if self.viewer.session_view == SessionView::Detail {
                    let detail = self.viewer.session_detail.as_ref();
                    (
                        detail.and_then(|d| d.data.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        detail.and_then(|d| d.data.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                } else {
                    let session = self.viewer.sessions.get(self.viewer.selected_session);
                    (
                        session.and_then(|s| s.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        session.and_then(|s| s.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                };
                if id.is_none() {
                    self.status_msg = "No session selected".into();
                    return;
                }
                (id, node)
            }
            ActionTarget::All => (None, None),
        };
        self.action_menu = Some(ActionMenu {
            target,
            selected: 0,
            session_id,
            session_node,
            scope: None,
            pending_kind: None,
        });
    }

    pub(crate) async fn handle_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.viewer.sessions.is_empty() {
                    self.viewer.selected_session = (self.viewer.selected_session + self.visible_rows).min(self.viewer.sessions.len() - 1);
                    self.viewer.table_state.select(Some(self.viewer.selected_session));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.selected_session = self.viewer.selected_session.saturating_sub(self.visible_rows);
                self.viewer.table_state.select(Some(self.viewer.selected_session));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.viewer.sessions.is_empty() {
                    self.viewer.selected_session = (self.viewer.selected_session + 1).min(self.viewer.sessions.len() - 1);
                    self.viewer.table_state.select(Some(self.viewer.selected_session));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.viewer.selected_session > 0 {
                    self.viewer.selected_session -= 1;
                    self.viewer.table_state.select(Some(self.viewer.selected_session));
                }
            }
            KeyCode::Enter => {
                self.vr_open_session_detail().await;
            }
            KeyCode::Char('r') => {
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.enter_expression_mode();
            }
            KeyCode::Char('t') => {
                self.time_range_next();
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('T') => {
                self.time_range_prev();
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('s') => {
                self.viewer.sort_column = (self.viewer.sort_column + 1) % self.viewer.session_fields.len();
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('S') => {
                self.viewer.sort_desc = !self.viewer.sort_desc;
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.viewer.sessions_filtered > self.viewer.page_size {
                    let last_page = (self.viewer.sessions_filtered - 1) / self.viewer.page_size * self.viewer.page_size;
                    if self.viewer.page_start != last_page {
                        self.viewer.page_start = last_page;
                        self.vr_fetch_sessions().await;
                    }
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.viewer.page_start > 0 {
                    self.viewer.page_start = 0;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Right => {
                let next = self.viewer.page_start + self.viewer.page_size;
                if next < self.viewer.sessions_filtered {
                    self.viewer.page_start = next;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Left => {
                if self.viewer.page_start > 0 {
                    self.viewer.page_start = self.viewer.page_start.saturating_sub(self.viewer.page_size);
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Home => {
                if self.viewer.page_start > 0 {
                    self.viewer.page_start = 0;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Char('g') => {
                let was_off = !self.viewer.graph_size.is_visible();
                self.viewer.graph_size = self.viewer.graph_size.next();
                if was_off && self.viewer.graph_size.is_visible() {
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Char('G') => {
                if self.viewer.graph_size.is_visible() {
                    self.viewer.graph_type = self.viewer.graph_type.next();
                }
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('a') => {
                self.open_action_menu(ActionTarget::Single);
            }
            KeyCode::Char('A') => {
                self.open_action_menu(ActionTarget::All);
            }
            KeyCode::Char('p') => {
                self.request_packets();
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.vr_fetch_layouts().await;
                self.viewer.layout_popup_mode = LayoutPopupMode::List;
                self.viewer.layout_popup_selected = 0;
                self.viewer.layout_filter.clear();
                self.viewer.layout_filter_cursor = 0;
                self.viewer.show_layout_popup = true;
            }
            KeyCode::Char('v') => {
                self.open_view_popup().await;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_column_editor_key(&mut self, key: KeyEvent) {
        let mut state = ColumnEditorState {
            filter: &mut self.viewer.column_editor_filter,
            filter_cursor: &mut self.viewer.column_editor_filter_cursor,
            selected: &mut self.viewer.column_editor_selected,
            mode: &mut self.viewer.column_editor_mode,
            show: &mut self.viewer.show_column_editor,
        };
        let action = handle_column_editor_key_generic(
            key, &mut state, &mut self.viewer.column_editor_available, &mut self.show_help,
        );
        match action {
            Some("apply") => {
                self.vr_apply_column_editor();
                self.viewer.show_column_editor = false;
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            Some("default") => {
                self.viewer.columns = default_columns();
                self.vr_sync_session_fields();
                self.viewer.show_column_editor = false;
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_layout_popup_key(&mut self, key: KeyEvent) {
        let items: Vec<LayoutItem> = self.viewer.saved_layouts.iter()
            .map(|l| LayoutItem { name: l.name.clone(), shared: false })
            .collect();
        let mut state = LayoutPopupState {
            mode: &mut self.viewer.layout_popup_mode,
            selected: &mut self.viewer.layout_popup_selected,
            filter: &mut self.viewer.layout_filter,
            filter_cursor: &mut self.viewer.layout_filter_cursor,
            save_name: &mut self.viewer.layout_save_name,
            save_cursor: &mut self.viewer.layout_save_cursor,
            show: &mut self.viewer.show_layout_popup,
            delete_name: &mut self.viewer.layout_delete_name,
        };
        let action = handle_layout_popup_key_generic(key, &mut state, &items, &mut self.show_help);

        if let Some(cmd) = action {
            if cmd == "edit" {
                self.vr_build_column_editor();
                self.viewer.show_layout_popup = false;
                self.viewer.show_column_editor = true;
            } else if cmd == "default" {
                self.viewer.columns = default_columns();
                self.vr_sync_session_fields();
                self.viewer.page_start = 0;
                self.vr_fetch_sessions().await;
            } else if let Some(name) = cmd.strip_prefix("confirm_delete:") {
                match self.client.vr_delete_layout(name).await {
                    Ok(_) => {
                        self.viewer.saved_layouts.retain(|l| l.name != name);
                        self.status_msg = format!("Deleted layout '{name}'");
                        let max = self.viewer.saved_layouts.len() + 3;
                        if self.viewer.layout_popup_selected >= max {
                            self.viewer.layout_popup_selected = max.saturating_sub(1);
                        }
                    }
                    Err(e) => self.status_msg = format!("Error deleting layout: {e}"),
                }
            } else if let Some(name) = cmd.strip_prefix("save:") {
                let columns: Vec<String> = self.viewer.columns.iter()
                    .enumerate()
                    .filter(|(i, c)| !(*i == 0 && c.field == "ipProtocol"))
                    .map(|(_, c)| c.field.clone()).collect();
                let sort_field = self.viewer.columns.get(self.viewer.sort_column)
                    .map(|c| c.field.clone())
                    .unwrap_or_default();
                let sort_dir = if self.viewer.sort_desc { "desc" } else { "asc" };
                self.status_msg = format!("Saving layout '{name}' with {} columns...", columns.len());
                let exists = self.viewer.saved_layouts.iter().any(|l| l.name == name);
                let result = if exists {
                    self.client.vr_update_layout(name, &columns, &sort_field, sort_dir).await
                } else {
                    self.client.vr_create_layout(name, &columns, &sort_field, sort_dir).await
                };
                match result {
                    Ok(_) => {
                        self.status_msg = format!("Saved layout '{name}'");
                        self.vr_fetch_layouts().await;
                    }
                    Err(e) => self.status_msg = format!("Error saving layout: {e}"),
                }
            } else if let Some(idx_str) = cmd.strip_prefix("select:")
                && let Ok(idx) = idx_str.parse::<usize>()
                    && let Some(layout) = self.viewer.saved_layouts.get(idx).cloned() {
                        self.vr_apply_layout(&layout);
                        self.viewer.page_start = 0;
                        self.vr_fetch_sessions().await;
                    }
        }
    }

    async fn open_view_popup(&mut self) {
        self.status_msg = "Fetching views...".into();
        match self.client.vr_get_views().await {
            Ok(views) => {
                self.viewer.saved_views = views;
                self.status_msg = String::new();
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
        self.viewer.view_popup_mode = ViewPopupMode::List;
        self.viewer.view_popup_selected = 0;
        self.viewer.view_filter.clear();
        self.viewer.view_filter_cursor = 0;
        self.viewer.view_filter_active = false;
        self.viewer.show_view_popup = true;
    }

    pub fn view_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.viewer.view_filter.to_lowercase();
        if filter_text.is_empty() {
            return (0..self.viewer.saved_views.len()).collect();
        }
        self.viewer.saved_views.iter().enumerate()
            .filter(|(_, v)| v.name.to_lowercase().contains(&filter_text) || v.expression.to_lowercase().contains(&filter_text))
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) async fn handle_view_popup_key(&mut self, key: KeyEvent) {
        match self.viewer.view_popup_mode {
            ViewPopupMode::SaveInput => {
                match key.code {
                    KeyCode::Enter => {
                        let name = self.viewer.view_save_name.trim().to_string();
                        if !name.is_empty() && !self.expression.is_empty() {
                            let col_config = if self.viewer.view_save_columns {
                                let cols: Vec<String> = self.viewer.columns.iter().map(|c| c.exp.clone()).collect();
                                let sort_field = self.viewer.session_fields.get(self.viewer.sort_column)
                                    .cloned().unwrap_or_else(|| "firstPacket".into());
                                let sort_dir = if self.viewer.sort_desc { "desc" } else { "asc" };
                                Some((cols, sort_field, sort_dir.to_string()))
                            } else {
                                None
                            };
                            let config_ref = col_config.as_ref().map(|(c, sf, sd)| (c.as_slice(), sf.as_str(), sd.as_str()));
                            match self.client.vr_create_view(&name, &self.expression, config_ref).await {
                                Ok(resp) => {
                                    self.status_msg = format!("View '{}' created", name);
                                    let view_id = resp.get("view")
                                        .and_then(|v| v.get("id"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&name)
                                        .to_string();
                                    self.viewer.active_view = Some(view_id);
                                    self.viewer.active_view_name = Some(name);
                                    self.viewer.page_start = 0;
                                    self.viewer.show_view_popup = false;
                                    self.refresh_for_active_tab().await;
                                }
                                Err(e) => self.status_msg = format!("Error creating view: {e}"),
                            }
                        } else if self.expression.is_empty() {
                            self.status_msg = "Cannot save view: expression is empty".into();
                        }
                        self.viewer.view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Esc => {
                        self.viewer.view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Tab => {
                        self.viewer.view_save_columns = !self.viewer.view_save_columns;
                    }
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Right | KeyCode::Char(_) => {
                        handle_text_input_key(key.code, &mut self.viewer.view_save_name, &mut self.viewer.view_save_cursor);
                    }
                    _ => {}
                }
            }
            ViewPopupMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let id = self.viewer.view_delete_id.clone();
                        let name = self.viewer.view_delete_name.clone();
                        match self.client.vr_delete_view(&id).await {
                            Ok(_) => {
                                self.status_msg = format!("View '{}' deleted", name);
                                if self.viewer.active_view.as_deref() == Some(&id) {
                                    self.viewer.active_view = None;
                                    self.viewer.active_view_name = None;
                                }
                                self.viewer.saved_views.retain(|v| v.id != id);
                                self.viewer.view_popup_selected = 0;
                            }
                            Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                        }
                        self.viewer.view_popup_mode = ViewPopupMode::List;
                    }
                    _ => {
                        self.viewer.view_popup_mode = ViewPopupMode::List;
                    }
                }
            }
            ViewPopupMode::List => {
                let filtered = self.view_filtered_indices();
                let total_items = 2 + filtered.len(); // 0=Save, 1=Clear, 2+=views
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        if total_items > 0 {
                            self.viewer.view_popup_selected = (self.viewer.view_popup_selected + 1).min(total_items - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.viewer.view_popup_selected = self.viewer.view_popup_selected.saturating_sub(1);
                    }
                    KeyCode::Char('x') => {
                        if self.viewer.view_popup_selected >= 2 {
                            let fi = self.viewer.view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view = &self.viewer.saved_views[idx];
                                if !view.shared {
                                    self.viewer.view_delete_id = view.id.clone();
                                    self.viewer.view_delete_name = view.name.clone();
                                    self.viewer.view_popup_mode = ViewPopupMode::ConfirmDelete;
                                } else {
                                    self.status_msg = "Cannot delete shared views".into();
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if self.viewer.view_popup_selected == 0 {
                            // Save current expression as view
                            if self.expression.is_empty() {
                                self.status_msg = "Cannot save view: expression is empty".into();
                            } else {
                                self.viewer.view_save_name.clear();
                                self.viewer.view_save_cursor = 0;
                                self.viewer.view_save_columns = false;
                                self.viewer.view_popup_mode = ViewPopupMode::SaveInput;
                            }
                        } else if self.viewer.view_popup_selected == 1 {
                            // Clear view
                            if self.viewer.active_view.is_some() {
                                self.viewer.active_view = None;
                                self.viewer.active_view_name = None;
                                self.viewer.page_start = 0;
                                self.viewer.show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            } else {
                                self.viewer.show_view_popup = false;
                            }
                        } else {
                            // Select a view
                            let fi = self.viewer.view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view_id = self.viewer.saved_views[idx].id.clone();
                                let view_name = self.viewer.saved_views[idx].name.clone();
                                self.viewer.active_view = Some(view_id);
                                self.viewer.active_view_name = Some(view_name);
                                self.viewer.page_start = 0;
                                self.viewer.show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if self.viewer.view_filter_active {
                            self.viewer.view_filter.clear();
                            self.viewer.view_filter_cursor = 0;
                            self.viewer.view_filter_active = false;
                        } else {
                            self.viewer.show_view_popup = false;
                        }
                    }
                    KeyCode::Char('/') => {
                        if !self.viewer.view_filter_active {
                            self.viewer.view_filter_active = true;
                            self.viewer.view_filter.clear();
                            self.viewer.view_filter_cursor = 0;
                        }
                    }
                    _ => {
                        if self.viewer.view_filter_active
                            && handle_text_input_key(key.code, &mut self.viewer.view_filter, &mut self.viewer.view_filter_cursor) {
                                if self.viewer.view_filter.is_empty() {
                                    self.viewer.view_filter_active = false;
                                } else {
                                    self.viewer.view_popup_selected = 2;
                                }
                            }
                    }
                }
            }
        }
    }

    pub(crate) async fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.viewer.session_view = SessionView::List;
                self.viewer.session_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.viewer.session_detail
                    && detail.total_rows > 0 && detail.selected < detail.total_rows - 1 {
                        detail.selected += 1;
                    }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.viewer.session_detail
                    && detail.selected > 0 {
                        detail.selected -= 1;
                    }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.selected = detail.total_rows.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.viewer.session_detail
                    && let Some(obj) = detail.data.as_object() {
                        let filter_lower = detail.filter.to_lowercase();
                        let mut keys: Vec<&String> = obj.keys()
                            .filter(|k| !is_hidden_detail_field(k))
                            .filter(|k| {
                                if filter_lower.is_empty() {
                                    return true;
                                }
                                let friendly = self.viewer.field_friendly_map.get(k.as_str())
                                    .map(|s| s.as_str())
                                    .unwrap_or(k.as_str());
                                k.to_lowercase().contains(&filter_lower)
                                    || friendly.to_lowercase().contains(&filter_lower)
                            })
                            .collect();
                        keys.sort();
                        if let Some(db_field) = keys.get(detail.selected) {
                            if is_non_actionable_field(db_field) {
                                return;
                            }
                            let val = &obj[*db_field];
                            let (val_str, values) = match val {
                                serde_json::Value::String(s) => (s.clone(), None),
                                serde_json::Value::Array(arr) => {
                                    let items: Vec<String> = arr.iter()
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            other => other.to_string(),
                                        })
                                        .collect();
                                    if items.is_empty() {
                                        ("-".into(), None)
                                    } else if items.len() == 1 {
                                        (items[0].clone(), None)
                                    } else {
                                        (items[0].clone(), Some(items))
                                    }
                                }
                                serde_json::Value::Null => ("-".into(), None),
                                other => (other.to_string(), None),
                            };
                            let exp_name = self.viewer.field_exp_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            let friendly = self.viewer.field_friendly_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            self.viewer.detail_action_menu = Some(DetailActionMenu {
                                field: exp_name,
                                display: friendly,
                                value: val_str,
                                selected: 0,
                                values,
                                value_selected: 0,
                            });
                        }
                    }
            }
            KeyCode::Char('a') => {
                self.open_action_menu(ActionTarget::Single);
            }
            KeyCode::Char('A') => {
                self.open_action_menu(ActionTarget::All);
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('E') => {
                self.enter_expression_mode();
            }
            KeyCode::Char('p') => {
                self.request_packets();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_detail_filter_key(&mut self, key: KeyEvent) {
        let is_stats = self.active_tab == Tab::Stats;
        let is_shards = is_stats && self.viewer.stats_tab == crate::app::StatsTab::DBShards;
        let is_files = self.active_tab == Tab::Files;
        let is_c3_detail = self.app_mode == crate::app::AppMode::Cont3xt && self.active_tab == Tab::Search;
        match key.code {
            KeyCode::Esc => {
                if is_c3_detail {
                    self.cont3xt.detail_filter.clear();
                    self.cont3xt.detail_filter_cursor = 0;
                    self.cont3xt.detail_scroll = 0;
                } else if is_shards {
                    if let Some(ref mut detail) = self.viewer.shards_detail {
                        detail.filter.clear();
                        detail.filter_cursor = 0;
                        detail.scroll = 0;
                    }
                } else if is_stats {
                    if let Some(ref mut detail) = self.viewer.stats_detail {
                        detail.filter.clear();
                        detail.filter_cursor = 0;
                        detail.scroll = 0;
                    }
                } else if is_files {
                    if let Some(ref mut detail) = self.viewer.files_detail {
                        detail.filter.clear();
                        detail.filter_cursor = 0;
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.viewer.session_detail {
                    detail.filter.clear();
                    detail.filter_cursor = 0;
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            _ => {
                if is_c3_detail {
                    if handle_text_input_key(key.code, &mut self.cont3xt.detail_filter, &mut self.cont3xt.detail_filter_cursor) {
                        self.cont3xt.detail_scroll = 0;
                    }
                } else if is_shards {
                    if let Some(ref mut detail) = self.viewer.shards_detail
                        && handle_text_input_key(key.code, &mut detail.filter, &mut detail.filter_cursor) {
                            detail.scroll = 0;
                        }
                } else if is_stats {
                    if let Some(ref mut detail) = self.viewer.stats_detail
                        && handle_text_input_key(key.code, &mut detail.filter, &mut detail.filter_cursor) {
                            detail.scroll = 0;
                        }
                } else if is_files {
                    if let Some(ref mut detail) = self.viewer.files_detail
                        && handle_text_input_key(key.code, &mut detail.filter, &mut detail.filter_cursor) {
                            detail.scroll = 0;
                        }
                } else if let Some(ref mut detail) = self.viewer.session_detail
                    && handle_text_input_key(key.code, &mut detail.filter, &mut detail.filter_cursor) {
                        detail.selected = 0;
                        detail.scroll = 0;
                        self.recalc_detail_rows();
                    }
            }
        }
    }

    fn recalc_detail_rows(&mut self) {
        if let Some(ref mut detail) = self.viewer.session_detail
            && let Some(obj) = detail.data.as_object() {
                let filter_lower = detail.filter.to_lowercase();
                detail.total_rows = obj.keys()
                    .filter(|k| !is_hidden_detail_field(k))
                    .filter(|k| {
                        if filter_lower.is_empty() {
                            return true;
                        }
                        let friendly = self.viewer.field_friendly_map.get(k.as_str())
                            .map(|s| s.as_str())
                            .unwrap_or(k.as_str());
                        k.to_lowercase().contains(&filter_lower)
                            || friendly.to_lowercase().contains(&filter_lower)
                    })
                    .count();
            }
    }

    pub(crate) fn handle_action_menu_key(&mut self, key: KeyEvent) {
        let remove_enabled = self.vr_remove_enabled();
        let menu = match &mut self.action_menu {
            Some(m) => m,
            None => return,
        };
        let in_scope = menu.scope.is_some();
        match key.code {
            KeyCode::Esc => {
                if in_scope {
                    let menu = self.action_menu.as_mut().unwrap();
                    menu.scope = None;
                    menu.selected = 0;
                } else {
                    self.action_menu = None;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if in_scope {
                    menu.selected = (menu.selected + 1).min(1);
                } else {
                    let len = menu.options(remove_enabled).len();
                    menu.selected = (menu.selected + 1).min(len - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if menu.selected > 0 {
                    menu.selected -= 1;
                }
            }
            KeyCode::Enter => {
                if in_scope {
                    let scope = if menu.selected == 0 { ActionScope::Visible } else { ActionScope::Matching };
                    let kind = menu.pending_kind.unwrap();
                    let target = menu.target;
                    let session_id = menu.session_id.clone();
                    let session_node = menu.session_node.clone();
                    let default_input = match kind {
                        ActionKind::DownloadPcap => "sessions.pcap".to_string(),
                        ActionKind::ExportCsv => "sessions.csv".to_string(),
                        _ => String::new(),
                    };
                    self.action_menu = None;
                    self.action_prompt = Some(ActionPrompt {
                        kind,
                        target,
                        scope,
                        session_id,
                        session_node,
                        input_cursor: default_input.len(),
                        input: default_input,
                    });
                    self.input_mode = InputMode::ActionPrompt;
                    return;
                }

                let options = menu.options(remove_enabled);
                let kind = options[menu.selected];
                let target = menu.target;
                let session_id = menu.session_id.clone();
                let session_node = menu.session_node.clone();

                // For ALL PCAP/CSV, show scope selector first
                if target == ActionTarget::All
                    && (kind == ActionKind::DownloadPcap || kind == ActionKind::ExportCsv)
                {
                    let menu = self.action_menu.as_mut().unwrap();
                    menu.pending_kind = Some(kind);
                    menu.scope = Some(ActionScope::Visible);
                    menu.selected = 0;
                    return;
                }

                let default_input = match kind {
                    ActionKind::DownloadPcap => {
                        match target {
                            ActionTarget::Single => {
                                format!("{}.pcap", session_id.as_deref().unwrap_or("session"))
                            }
                            ActionTarget::All => "sessions.pcap".to_string(),
                        }
                    }
                    ActionKind::ExportCsv => "sessions.csv".to_string(),
                    ActionKind::AddTags | ActionKind::RemoveTags => String::new(),
                };
                self.action_menu = None;
                self.action_prompt = Some(ActionPrompt {
                    kind,
                    target,
                    scope: ActionScope::Matching,
                    session_id,
                    session_node,
                    input_cursor: default_input.len(),
                    input: default_input,
                });
                self.input_mode = InputMode::ActionPrompt;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_action_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.action_prompt = None;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                let prompt = match self.action_prompt.take() {
                    Some(p) => p,
                    None => return,
                };
                self.input_mode = InputMode::Normal;
                if prompt.input.is_empty() {
                    self.status_msg = "No input provided".into();
                    return;
                }
                self.execute_action(prompt).await;
            }
            _ => {
                if let Some(ref mut prompt) = self.action_prompt {
                    handle_text_input_key(key.code, &mut prompt.input, &mut prompt.input_cursor);
                }
            }
        }
    }

    fn visible_session_ids(&self) -> Vec<String> {
        self.viewer.sessions.iter()
            .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect()
    }

    async fn execute_action(&mut self, prompt: ActionPrompt) {
        let date = self.time_range.date_value();
        match (prompt.kind, prompt.target) {
            (ActionKind::DownloadPcap, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                let node = prompt.session_node.as_deref().unwrap_or("");
                self.status_msg = "Downloading PCAP...".into();
                match self.client.vr_download_session_pcap(node, id).await {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::DownloadPcap, ActionTarget::All) => {
                self.status_msg = "Downloading PCAP...".into();
                let result = if prompt.scope == ActionScope::Visible {
                    let ids = self.visible_session_ids();
                    self.client.vr_download_sessions_pcap_ids(&ids).await
                } else {
                    self.client.vr_download_sessions_pcap(&self.expression, date, &self.viewer.active_view).await
                };
                match result {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::ExportCsv, ActionTarget::All) => {
                self.status_msg = "Exporting CSV...".into();
                let result = if prompt.scope == ActionScope::Visible {
                    let ids = self.visible_session_ids();
                    self.client.vr_export_sessions_csv_ids(&ids, &self.viewer.session_fields).await
                } else {
                    self.client.vr_export_sessions_csv(&self.expression, date, &self.viewer.session_fields, &self.viewer.active_view).await
                };
                match result {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::AddTags, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                self.status_msg = "Adding tags...".into();
                match self.client.vr_add_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.vr_fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::AddTags, ActionTarget::All) => {
                self.status_msg = "Adding tags...".into();
                match self.client.vr_add_sessions_tags(&self.expression, date, &prompt.input, &self.viewer.active_view).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.vr_fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                self.status_msg = "Removing tags...".into();
                match self.client.vr_remove_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.vr_fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::All) => {
                self.status_msg = "Removing tags...".into();
                match self.client.vr_remove_sessions_tags(&self.expression, date, &prompt.input, &self.viewer.active_view).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.vr_fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_detail_action_key(&mut self, key: KeyEvent) {
        let in_values = self.viewer.detail_action_menu.as_ref()
            .map(|m| m.values.is_some()).unwrap_or(false);

        match key.code {
            KeyCode::Esc => {
                self.viewer.detail_action_menu = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut menu) = self.viewer.detail_action_menu {
                    if in_values {
                        let len = menu.values.as_ref().unwrap().len();
                        menu.value_selected = (menu.value_selected + 1).min(len - 1);
                    } else {
                        menu.selected = (menu.selected + 1).min(DetailActionMenu::OPTIONS.len() - 1);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut menu) = self.viewer.detail_action_menu {
                    if in_values {
                        if menu.value_selected > 0 {
                            menu.value_selected -= 1;
                        }
                    } else if menu.selected > 0 {
                        menu.selected -= 1;
                    }
                }
            }
            KeyCode::Enter => {
                if in_values {
                    // Pick the selected value, then show AND/OR options
                    if let Some(ref mut menu) = self.viewer.detail_action_menu {
                        let chosen = menu.values.as_ref().unwrap()[menu.value_selected].clone();
                        menu.value = chosen;
                        menu.values = None;
                        menu.selected = 0;
                    }
                } else if let Some(menu) = self.viewer.detail_action_menu.take() {
                    let needs_quotes = menu.value.parse::<f64>().is_err();
                    let quoted_val = if needs_quotes {
                        format!("\"{}\"", menu.value)
                    } else {
                        menu.value.clone()
                    };

                    let (connector, op) = match menu.selected {
                        0 => ("&&", "=="),
                        1 => ("&&", "!="),
                        2 => ("||", "=="),
                        3 => ("||", "!="),
                        _ => ("&&", "=="),
                    };

                    let clause = format!("{} {} {}", menu.field, op, quoted_val);

                    if self.expression.is_empty() {
                        self.expression = clause;
                    } else {
                        self.expression = format!("{} {} {}", self.expression, connector, clause);
                    }
                    self.expression_edit = self.expression.clone();
                    if self.active_tab == Tab::Arkime {
                        self.vr_request_summary_fetch();
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_packets_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('q') => {
                self.viewer.packets_view = None;
                self.viewer.packets_scroll = 0;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::PageUp => {
                self.viewer.packets_scroll = self.viewer.packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Home | KeyCode::Left => {
                self.viewer.packets_scroll = 0;
            }
            KeyCode::Right => {
                self.viewer.packets_scroll = u16::MAX;
            }
            KeyCode::Char('r') => {
                self.viewer.packets_raw = !self.viewer.packets_raw;
                self.request_packets();
            }
            KeyCode::Char('l') => {
                self.viewer.packets_line = self.viewer.packets_line.next();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub fn request_packets(&mut self) {
        if let Some(session) = self.viewer.sessions.get(self.viewer.selected_session) {
            let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let node = session.get("node").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || node.is_empty() {
                self.status_msg = "No session id/node".into();
                return;
            }
            let src_pkts = session.pointer("/source/packets").and_then(|v| v.as_u64())
                .or_else(|| session.get("source.packets").and_then(|v| v.as_u64()))
                .unwrap_or(0);
            let dst_pkts = session.pointer("/destination/packets").and_then(|v| v.as_u64())
                .or_else(|| session.get("destination.packets").and_then(|v| v.as_u64()))
                .unwrap_or(0);
            let total = src_pkts + dst_pkts;
            self.viewer.packets_total_pending = total;
            self.viewer.packets_node_pending = node.to_string();
            self.viewer.packets_id_pending = id.to_string();
            self.status_msg = "Fetching packets...".into();
            if total > 500 {
                self.show_loading = true;
            }
            self.viewer.pending_packets_fetch = true;
        }
    }

    pub(crate) async fn handle_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::Char('1') => {
                if self.viewer.stats_tab != StatsTab::CaptureGraphs {
                    self.viewer.stats_tab = StatsTab::CaptureGraphs;
                    if !self.viewer.cg_loaded {
                        self.vr_request_stats_fetch();
                    }
                }
            }
            KeyCode::Char('2') => {
                if self.viewer.stats_tab != StatsTab::Capture {
                    self.viewer.stats_tab = StatsTab::Capture;
                    if !self.viewer.stats_state_loaded[0] {
                        self.vr_load_stats_state(StatsTab::Capture).await;
                        self.viewer.stats_state_loaded[0] = true;
                    }
                    self.vr_request_stats_fetch();
                }
            }
            KeyCode::Char('3') => {
                if self.viewer.stats_tab != StatsTab::DBStats {
                    self.viewer.stats_tab = StatsTab::DBStats;
                    if !self.viewer.stats_state_loaded[1] {
                        self.vr_load_stats_state(StatsTab::DBStats).await;
                        self.viewer.stats_state_loaded[1] = true;
                    }
                    self.vr_request_stats_fetch();
                }
            }
            KeyCode::Char('4') => {
                if self.viewer.stats_tab != StatsTab::DBIndices {
                    self.viewer.stats_tab = StatsTab::DBIndices;
                    if !self.viewer.stats_state_loaded[2] {
                        self.vr_load_stats_state(StatsTab::DBIndices).await;
                        self.viewer.stats_state_loaded[2] = true;
                    }
                    self.vr_request_stats_fetch();
                }
            }
            KeyCode::Char('5') => {
                if self.viewer.stats_tab != StatsTab::DBTasks {
                    self.viewer.stats_tab = StatsTab::DBTasks;
                    if !self.viewer.stats_state_loaded[3] {
                        self.vr_load_stats_state(StatsTab::DBTasks).await;
                        self.viewer.stats_state_loaded[3] = true;
                    }
                    self.vr_request_stats_fetch();
                }
            }
            KeyCode::Char('6') => {
                if self.viewer.stats_tab != StatsTab::DBShards {
                    self.viewer.stats_tab = StatsTab::DBShards;
                    if !self.viewer.shards_loaded {
                        self.vr_request_stats_fetch();
                    }
                }
            }
            KeyCode::Char('7') => {
                if self.viewer.stats_tab != StatsTab::DBRecovery {
                    self.viewer.stats_tab = StatsTab::DBRecovery;
                    if !self.viewer.stats_state_loaded[4] {
                        self.vr_load_stats_state(StatsTab::DBRecovery).await;
                        self.viewer.stats_state_loaded[4] = true;
                    }
                    self.vr_request_stats_fetch();
                }
            }
            // --- CaptureGraphs-specific keys ---
            _ if self.viewer.stats_tab == StatsTab::CaptureGraphs => {
                self.handle_capture_graphs_key(key).await;
            }
            // --- DB Shards-specific keys ---
            _ if self.viewer.stats_tab == StatsTab::DBShards => {
                self.handle_shards_key(key).await;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.viewer.stats_data.is_empty() {
                    self.viewer.stats_selected = (self.viewer.stats_selected + 1).min(self.viewer.stats_data.len() - 1);
                    self.viewer.stats_table_state.select(Some(self.viewer.stats_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.viewer.stats_selected > 0 {
                    self.viewer.stats_selected -= 1;
                    self.viewer.stats_table_state.select(Some(self.viewer.stats_selected));
                }
            }
            KeyCode::Enter => {
                self.vr_open_stats_detail();
            }
            KeyCode::Char('r') => {
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.viewer.stats_filter_edit = self.viewer.stats_filter.clone();
                self.expression_cursor = self.viewer.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let num_cols = self.vr_stats_active_columns().len();
                self.viewer.stats_sort_column = (self.viewer.stats_sort_column + 1) % num_cols;
                self.vr_request_stats_fetch();
                self.vr_save_stats_state().await;
            }
            KeyCode::Char('S') => {
                self.viewer.stats_sort_desc = !self.viewer.stats_sort_desc;
                self.vr_request_stats_fetch();
                self.vr_save_stats_state().await;
            }
            KeyCode::Char('d') if self.viewer.stats_tab == StatsTab::DBIndices => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let index = crate::api::str_val(row, "index");
                    if !index.is_empty() {
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: "Delete Index".into(),
                            message: format!("Delete index '{index}'?"),
                            action: format!("delete_esindex:{index}"),
                        });
                    }
                }
            }
            KeyCode::Char('f') if self.viewer.stats_tab == StatsTab::DBIndices => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let index = crate::api::str_val(row, "index");
                    if !index.is_empty() {
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: "Force Merge".into(),
                            message: format!("Force merge index '{index}'?"),
                            action: format!("optimize_esindex:{index}"),
                        });
                    }
                }
            }
            KeyCode::Char('C') if self.viewer.stats_tab == StatsTab::DBIndices => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let index = crate::api::str_val(row, "index");
                    let status = crate::api::str_val(row, "status");
                    if !index.is_empty() && status == "open" {
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: "Close Index".into(),
                            message: format!("Close index '{index}'?"),
                            action: format!("close_esindex:{index}"),
                        });
                    }
                }
            }
            KeyCode::Char('O') if self.viewer.stats_tab == StatsTab::DBIndices => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let index = crate::api::str_val(row, "index");
                    let status = crate::api::str_val(row, "status");
                    if !index.is_empty() && status == "close" {
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: "Open Index".into(),
                            message: format!("Open index '{index}'?"),
                            action: format!("open_esindex:{index}"),
                        });
                    }
                }
            }
            KeyCode::Char('e') if self.viewer.stats_tab == StatsTab::DBStats => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let name = crate::api::str_val(row, "name");
                    let excluded = row.get("nodeExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !name.is_empty() {
                        let (action, title, verb) = if excluded {
                            ("include", "Include Node", "Include")
                        } else {
                            ("exclude", "Exclude Node", "Exclude")
                        };
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: title.into(),
                            message: format!("{verb} node '{name}'?"),
                            action: format!("esshards:name:{name}:{action}"),
                        });
                    }
                }
            }
            KeyCode::Char('x') if self.viewer.stats_tab == StatsTab::DBStats => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let ip = crate::api::str_val(row, "ip");
                    let excluded = row.get("ipExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !ip.is_empty() {
                        let (action, title, verb) = if excluded {
                            ("include", "Include IP", "Include")
                        } else {
                            ("exclude", "Exclude IP", "Exclude")
                        };
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: title.into(),
                            message: format!("{verb} IP '{ip}'?"),
                            action: format!("esshards:ip:{ip}:{action}"),
                        });
                    }
                }
            }
            KeyCode::Char('d') if self.viewer.stats_tab == StatsTab::DBTasks => {
                if let Some(row) = self.viewer.stats_data.get(self.viewer.stats_selected) {
                    let task_id = crate::api::str_val(row, "taskId");
                    let action = crate::api::str_val(row, "action");
                    if !task_id.is_empty() {
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: "Cancel Task".into(),
                            message: format!("Cancel task '{action}' ({task_id})?"),
                            action: format!("cancel_estask:{task_id}"),
                        });
                    }
                }
            }
            KeyCode::Char('X') if self.viewer.stats_tab == StatsTab::DBTasks => {
                self.confirm_dialog = Some(ConfirmDialog {
                    title: "Cancel All Tasks".into(),
                    message: "Cancel all cancellable tasks?".into(),
                    action: "cancel_all_estasks".into(),
                });
            }
            KeyCode::Char('m') if self.viewer.stats_tab == StatsTab::DBRecovery => {
                self.viewer.recovery_show_all = !self.viewer.recovery_show_all;
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('c') => {
                self.vr_stats_fetch_shareables().await;
                self.viewer.stats_layout_popup_mode = LayoutPopupMode::List;
                self.viewer.stats_layout_popup_selected = 0;
                self.viewer.stats_layout_filter.clear();
                self.viewer.stats_layout_filter_cursor = 0;
                self.viewer.stats_show_layout_popup = true;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_stats_column_editor_key(&mut self, key: KeyEvent) {
        let mut state = ColumnEditorState {
            filter: &mut self.viewer.stats_column_editor_filter,
            filter_cursor: &mut self.viewer.stats_column_editor_filter_cursor,
            selected: &mut self.viewer.stats_column_editor_selected,
            mode: &mut self.viewer.stats_column_editor_mode,
            show: &mut self.viewer.stats_show_column_editor,
        };
        let action = handle_column_editor_key_generic(
            key, &mut state, &mut self.viewer.stats_column_editor_items, &mut self.show_help,
        );
        match action {
            Some("apply") => {
                self.vr_stats_apply_column_editor();
                self.viewer.stats_show_column_editor = false;
                self.vr_request_stats_fetch();
                self.vr_save_stats_state().await;
            }
            Some("default") => {
                self.vr_stats_reset_default_columns();
                self.viewer.stats_show_column_editor = false;
                self.vr_request_stats_fetch();
                self.vr_save_stats_state().await;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_stats_layout_popup_key(&mut self, key: KeyEvent) {
        let items: Vec<LayoutItem> = self.viewer.stats_saved_shareables.iter()
            .map(|s| LayoutItem { name: s.name.clone(), shared: s.shared })
            .collect();
        // We need to stash the delete_id separately since LayoutPopupState uses delete_name
        let mut delete_name_for_id = String::new();
        let mut state = LayoutPopupState {
            mode: &mut self.viewer.stats_layout_popup_mode,
            selected: &mut self.viewer.stats_layout_popup_selected,
            filter: &mut self.viewer.stats_layout_filter,
            filter_cursor: &mut self.viewer.stats_layout_filter_cursor,
            save_name: &mut self.viewer.stats_layout_save_name,
            save_cursor: &mut self.viewer.stats_layout_save_cursor,
            show: &mut self.viewer.stats_show_layout_popup,
            delete_name: &mut delete_name_for_id,
        };
        let action = handle_layout_popup_key_generic(key, &mut state, &items, &mut self.show_help);

        if let Some(cmd) = action {
            if cmd == "edit" {
                self.vr_stats_build_column_editor();
                self.viewer.stats_show_layout_popup = false;
                self.viewer.stats_show_column_editor = true;
            } else if cmd == "default" {
                self.vr_stats_reset_default_columns();
                self.vr_request_stats_fetch();
                self.vr_save_stats_state().await;
            } else if cmd.starts_with("confirm_delete:") {
                // For stats, we need the shareable ID, which we find by name
                if let Some(s) = self.viewer.stats_saved_shareables.iter().find(|s| s.name == delete_name_for_id) {
                    let id = s.id.clone();
                    self.vr_stats_delete_shareable(&id).await;
                    let max = self.viewer.stats_saved_shareables.len() + 3;
                    if self.viewer.stats_layout_popup_selected >= max {
                        self.viewer.stats_layout_popup_selected = max.saturating_sub(1);
                    }
                }
            } else if let Some(name) = cmd.strip_prefix("save:") {
                self.vr_stats_save_shareable(name).await;
            } else if let Some(idx_str) = cmd.strip_prefix("select:")
                && let Ok(idx) = idx_str.parse::<usize>()
                    && let Some(shareable) = self.viewer.stats_saved_shareables.get(idx).cloned() {
                        self.vr_stats_apply_shareable(&shareable);
                        self.vr_request_stats_fetch();
                        self.vr_save_stats_state().await;
                    }
        }
        // Sync delete name back for UI display
        self.viewer.stats_layout_delete_name = delete_name_for_id;
    }

    pub(crate) async fn handle_capture_graphs_key(&mut self, key: KeyEvent) {
        use crate::app::types::CAPTURE_GRAPH_METRICS;

        // Metric popup is open
        if self.viewer.cg_show_metric_popup {
            match key.code {
                KeyCode::Esc => {
                    self.viewer.cg_show_metric_popup = false;
                    self.viewer.cg_metric_popup_filter.clear();
                    self.viewer.cg_metric_popup_filter_cursor = 0;
                }
                KeyCode::Enter => {
                    // Apply the selected metric
                    let filtered: Vec<usize> = CAPTURE_GRAPH_METRICS.iter().enumerate()
                        .filter(|(_, m)| {
                            if self.viewer.cg_metric_popup_filter.is_empty() {
                                true
                            } else {
                                let f = self.viewer.cg_metric_popup_filter.to_lowercase();
                                m.label.to_lowercase().contains(&f) || m.field.to_lowercase().contains(&f)
                            }
                        })
                        .map(|(i, _)| i)
                        .collect();
                    if let Some(&idx) = filtered.get(self.viewer.cg_metric_popup_selected) {
                        self.viewer.cg_metric_index = idx;
                        self.viewer.cg_show_metric_popup = false;
                        self.viewer.cg_metric_popup_filter.clear();
                        self.viewer.cg_metric_popup_filter_cursor = 0;
                        self.vr_request_stats_fetch();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let count = self.cg_filtered_metric_count();
                    if count > 0 {
                        self.viewer.cg_metric_popup_selected = (self.viewer.cg_metric_popup_selected + 1).min(count - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.viewer.cg_metric_popup_selected = self.viewer.cg_metric_popup_selected.saturating_sub(1);
                }
                _ => {
                    if handle_text_input_key(key.code, &mut self.viewer.cg_metric_popup_filter, &mut self.viewer.cg_metric_popup_filter_cursor) {
                        self.viewer.cg_metric_popup_selected = 0;
                    }
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('m') => {
                // Open metric selector popup
                self.viewer.cg_show_metric_popup = true;
                self.viewer.cg_metric_popup_selected = self.viewer.cg_metric_index;
                self.viewer.cg_metric_popup_filter.clear();
                self.viewer.cg_metric_popup_filter_cursor = 0;
            }
            KeyCode::Char('i') => {
                // Cycle interval
                self.viewer.cg_interval = self.viewer.cg_interval.next();
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('H') => {
                // Cycle hide mode
                self.viewer.cg_hide = self.viewer.cg_hide.next();
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('r') => {
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.viewer.stats_filter_edit = self.viewer.stats_filter.clone();
                self.expression_cursor = self.viewer.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let page = self.visible_rows.max(1);
                self.viewer.cg_scroll = (self.viewer.cg_scroll + page).min(self.viewer.cg_nodes.len().saturating_sub(1));
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let page = self.visible_rows.max(1);
                self.viewer.cg_scroll = self.viewer.cg_scroll.saturating_sub(page);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.viewer.cg_nodes.len().saturating_sub(1);
                if self.viewer.cg_scroll < max {
                    self.viewer.cg_scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.viewer.cg_scroll = self.viewer.cg_scroll.saturating_sub(1);
            }
            KeyCode::Left | KeyCode::Home => {
                self.viewer.cg_scroll = 0;
            }
            KeyCode::Right | KeyCode::End => {
                self.viewer.cg_scroll = self.viewer.cg_nodes.len().saturating_sub(1);
            }
            _ => {}
        }
    }

    fn cg_filtered_metric_count(&self) -> usize {
        use crate::app::types::CAPTURE_GRAPH_METRICS;
        if self.viewer.cg_metric_popup_filter.is_empty() {
            CAPTURE_GRAPH_METRICS.len()
        } else {
            let f = self.viewer.cg_metric_popup_filter.to_lowercase();
            CAPTURE_GRAPH_METRICS.iter()
                .filter(|m| m.label.to_lowercase().contains(&f) || m.field.to_lowercase().contains(&f))
                .count()
        }
    }

    pub(crate) async fn handle_shards_key(&mut self, key: KeyEvent) {
        // If sub-detail (single shard + explain) is open
        if self.viewer.shards_sub_detail.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.viewer.shards_sub_detail = None;
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_add(self.visible_rows as u16);
                    }
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_sub(self.visible_rows as u16);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_add(1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_sub(1);
                    }
                }
                KeyCode::PageDown => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_add(self.visible_rows as u16);
                    }
                }
                KeyCode::PageUp => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = d.scroll.saturating_sub(self.visible_rows as u16);
                    }
                }
                KeyCode::Left | KeyCode::Home => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = 0;
                    }
                }
                KeyCode::Right | KeyCode::End => {
                    if let Some(ref mut d) = self.viewer.shards_sub_detail {
                        d.scroll = u16::MAX;
                    }
                }
                _ => {}
            }
            return;
        }
        // If detail overlay (shard list for an index) is open
        if self.viewer.shards_detail.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.viewer.shards_detail = None;
                }
                KeyCode::Enter => {
                    self.vr_open_shard_sub_detail().await;
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_add(self.visible_rows as u16);
                    }
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_sub(self.visible_rows as u16);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_add(1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_sub(1);
                    }
                }
                KeyCode::PageDown => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_add(self.visible_rows as u16);
                    }
                }
                KeyCode::PageUp => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = d.scroll.saturating_sub(self.visible_rows as u16);
                    }
                }
                KeyCode::Left | KeyCode::Home => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = 0;
                    }
                }
                KeyCode::Right | KeyCode::End => {
                    if let Some(ref mut d) = self.viewer.shards_detail {
                        d.scroll = u16::MAX;
                    }
                }
                KeyCode::Char('/') => {
                    self.input_mode = InputMode::DetailFilter;
                }
                KeyCode::Char('h') | KeyCode::Char('?') => {
                    self.show_help = true;
                }
                KeyCode::Char('D') => {
                    self.show_debug = true;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::Char('1') => {
                self.viewer.stats_tab = StatsTab::Capture;
                if !self.viewer.stats_state_loaded[0] {
                    self.vr_load_stats_state(StatsTab::Capture).await;
                    self.viewer.stats_state_loaded[0] = true;
                }
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('2') => {
                self.viewer.stats_tab = StatsTab::DBStats;
                if !self.viewer.stats_state_loaded[1] {
                    self.vr_load_stats_state(StatsTab::DBStats).await;
                    self.viewer.stats_state_loaded[1] = true;
                }
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('3') => {
                self.viewer.stats_tab = StatsTab::DBIndices;
                if !self.viewer.stats_state_loaded[2] {
                    self.vr_load_stats_state(StatsTab::DBIndices).await;
                    self.viewer.stats_state_loaded[2] = true;
                }
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('4') => {
                self.viewer.stats_tab = StatsTab::DBTasks;
                if !self.viewer.stats_state_loaded[3] {
                    self.vr_load_stats_state(StatsTab::DBTasks).await;
                    self.viewer.stats_state_loaded[3] = true;
                }
                self.vr_request_stats_fetch();
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.shards_selected_row = (self.viewer.shards_selected_row + self.visible_rows).min(self.viewer.shards_indices.len().saturating_sub(1));
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.shards_selected_row = self.viewer.shards_selected_row.saturating_sub(self.visible_rows);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.shards_hscroll = self.viewer.shards_hscroll.saturating_add(5);
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.shards_hscroll = self.viewer.shards_hscroll.saturating_sub(5);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.viewer.shards_indices.is_empty() {
                    self.viewer.shards_selected_row = (self.viewer.shards_selected_row + 1).min(self.viewer.shards_indices.len() - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.viewer.shards_selected_row > 0 {
                    self.viewer.shards_selected_row -= 1;
                }
            }
            KeyCode::Right => {
                self.viewer.shards_hscroll = self.viewer.shards_hscroll.saturating_add(1);
            }
            KeyCode::Left => {
                self.viewer.shards_hscroll = self.viewer.shards_hscroll.saturating_sub(1);
            }
            KeyCode::Home => {
                self.viewer.shards_selected_row = 0;
                self.viewer.shards_hscroll = 0;
            }
            KeyCode::End => {
                self.viewer.shards_selected_row = self.viewer.shards_indices.len().saturating_sub(1);
            }
            KeyCode::Enter => {
                self.vr_open_shards_detail();
            }
            KeyCode::Char('m') => {
                self.viewer.shards_show = self.viewer.shards_show.next();
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('M') => {
                self.viewer.shards_show = self.viewer.shards_show.prev();
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('r') => {
                self.vr_request_stats_fetch();
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.viewer.stats_filter_edit = self.viewer.stats_filter.clone();
                self.expression_cursor = self.viewer.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('D') => {
                self.show_debug = true;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_stats_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.viewer.stats_view = StatsView::List;
                self.viewer.stats_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.viewer.stats_detail {
                    detail.scroll = u16::MAX;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('E') => {
                self.viewer.stats_filter_edit = self.viewer.stats_filter.clone();
                self.expression_cursor = self.viewer.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('e') if self.viewer.stats_tab == StatsTab::DBStats => {
                if let Some(ref detail) = self.viewer.stats_detail {
                    let name = crate::api::str_val(&detail.data, "name");
                    let excluded = detail.data.get("nodeExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !name.is_empty() {
                        let (action, title, verb) = if excluded {
                            ("include", "Include Node", "Include")
                        } else {
                            ("exclude", "Exclude Node", "Exclude")
                        };
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: title.into(),
                            message: format!("{verb} node '{name}'?"),
                            action: format!("esshards:name:{name}:{action}"),
                        });
                    }
                }
            }
            KeyCode::Char('x') if self.viewer.stats_tab == StatsTab::DBStats => {
                if let Some(ref detail) = self.viewer.stats_detail {
                    let ip = crate::api::str_val(&detail.data, "ip");
                    let excluded = detail.data.get("ipExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !ip.is_empty() {
                        let (action, title, verb) = if excluded {
                            ("include", "Include IP", "Include")
                        } else {
                            ("exclude", "Exclude IP", "Exclude")
                        };
                        self.confirm_dialog = Some(ConfirmDialog {
                            title: title.into(),
                            message: format!("{verb} IP '{ip}'?"),
                            action: format!("esshards:ip:{ip}:{action}"),
                        });
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_arkime_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
                if self.active_tab == Tab::Files && self.viewer.files_data.is_empty() {
                    self.vr_init_files_tab().await;
                }
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.enter_expression_mode();
            }
            KeyCode::Char('f') => {
                self.viewer.field_filter.clear();
                self.viewer.field_filter_cursor = 0;
                self.viewer.field_filter_selected = 0;
                self.input_mode = InputMode::FieldSelector;
            }
            KeyCode::Char('G') => {
                self.viewer.summary_metric = self.viewer.summary_metric.next();
            }
            KeyCode::Char('s') => {
                self.viewer.summary_sort = self.viewer.summary_sort.next();
                self.vr_sort_summary_data();
            }
            KeyCode::Char('S') => {
                self.viewer.summary_sort_desc = !self.viewer.summary_sort_desc;
                self.vr_sort_summary_data();
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.viewer.summary_data.is_empty() {
                    self.viewer.summary_selected = (self.viewer.summary_selected + self.visible_rows).min(self.viewer.summary_data.len() - 1);
                    self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.summary_selected = self.viewer.summary_selected.saturating_sub(self.visible_rows);
                self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.viewer.summary_data.is_empty() {
                    self.viewer.summary_selected = (self.viewer.summary_selected + 1).min(self.viewer.summary_data.len() - 1);
                    self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.viewer.summary_selected > 0 {
                    self.viewer.summary_selected -= 1;
                    self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
                }
            }
            KeyCode::PageDown => {
                if !self.viewer.summary_data.is_empty() {
                    self.viewer.summary_selected = (self.viewer.summary_selected + self.visible_rows).min(self.viewer.summary_data.len() - 1);
                    self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
                }
            }
            KeyCode::PageUp => {
                self.viewer.summary_selected = self.viewer.summary_selected.saturating_sub(self.visible_rows);
                self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
            }
            KeyCode::Left | KeyCode::Home => {
                self.viewer.summary_selected = 0;
                self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
            }
            KeyCode::Right | KeyCode::End => {
                if !self.viewer.summary_data.is_empty() {
                    self.viewer.summary_selected = self.viewer.summary_data.len() - 1;
                    self.viewer.summary_table_state.select(Some(self.viewer.summary_selected));
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.viewer.summary_data.get(self.viewer.summary_selected) {
                    let value = match &item.item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    self.viewer.detail_action_menu = Some(DetailActionMenu {
                        field: self.viewer.summary_field.clone(),
                        display: self.viewer.summary_field.clone(),
                        value,
                        selected: 0,
                        values: None,
                        value_selected: 0,
                    });
                }
            }
            KeyCode::Char('r') => {
                self.vr_request_summary_fetch();
            }
            KeyCode::Char('t') => {
                self.time_range_next();
                self.vr_request_summary_fetch();
            }
            KeyCode::Char('T') => {
                self.time_range_prev();
                self.vr_request_summary_fetch();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('v') => {
                self.open_view_popup().await;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_field_selector_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.viewer.field_filter.clear();
                self.viewer.field_filter_cursor = 0;
            }
            KeyCode::Enter => {
                let filtered = self.vr_filtered_fields();
                if let Some(field) = filtered.get(self.viewer.field_filter_selected) {
                    self.viewer.summary_field = field.exp.clone();
                    self.input_mode = InputMode::Normal;
                    self.viewer.field_filter.clear();
                    self.viewer.field_filter_cursor = 0;
                    self.vr_request_summary_fetch();
                }
            }
            KeyCode::Down => {
                let count = self.vr_filtered_fields().len();
                if count > 0 {
                    self.viewer.field_filter_selected = (self.viewer.field_filter_selected + 1).min(count - 1);
                }
            }
            KeyCode::Up => {
                if self.viewer.field_filter_selected > 0 {
                    self.viewer.field_filter_selected -= 1;
                }
            }
            _ => {
                if handle_text_input_key(key.code, &mut self.viewer.field_filter, &mut self.viewer.field_filter_cursor) {
                    self.viewer.field_filter_selected = 0;
                }
            }
        }
    }

    // --- Files tab key handlers ---

    pub(crate) async fn handle_files_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.viewer.stats_data.is_empty() {
                    self.vr_init_stats_tab().await;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.viewer.files_data.is_empty() {
                    self.viewer.files_selected = (self.viewer.files_selected + self.visible_rows).min(self.viewer.files_data.len() - 1);
                    self.viewer.files_table_state.select(Some(self.viewer.files_selected));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.viewer.files_selected = self.viewer.files_selected.saturating_sub(self.visible_rows);
                self.viewer.files_table_state.select(Some(self.viewer.files_selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.viewer.files_data.is_empty() {
                    self.viewer.files_selected = (self.viewer.files_selected + 1).min(self.viewer.files_data.len() - 1);
                    self.viewer.files_table_state.select(Some(self.viewer.files_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.viewer.files_selected > 0 {
                    self.viewer.files_selected -= 1;
                    self.viewer.files_table_state.select(Some(self.viewer.files_selected));
                }
            }
            KeyCode::Enter => {
                self.vr_open_files_detail();
            }
            KeyCode::Right => {
                let total = self.viewer.files_filtered as usize;
                let next = self.viewer.files_page_start + self.viewer.files_page_size;
                if next < total {
                    self.viewer.files_page_start = next;
                    self.viewer.files_selected = 0;
                    self.vr_fetch_files().await;
                }
            }
            KeyCode::Left => {
                if self.viewer.files_page_start > 0 {
                    self.viewer.files_page_start = self.viewer.files_page_start.saturating_sub(self.viewer.files_page_size);
                    self.viewer.files_selected = 0;
                    self.vr_fetch_files().await;
                }
            }
            KeyCode::Home => {
                if self.viewer.files_page_start > 0 {
                    self.viewer.files_page_start = 0;
                    self.viewer.files_selected = 0;
                    self.vr_fetch_files().await;
                }
            }
            KeyCode::Char('r') => {
                self.vr_fetch_files().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.viewer.files_filter_edit = self.viewer.files_filter.clone();
                self.expression_cursor = self.viewer.files_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let num_cols = self.viewer.files_columns.len();
                self.viewer.files_sort_column = (self.viewer.files_sort_column + 1) % num_cols;
                self.viewer.files_page_start = 0;
                self.viewer.files_selected = 0;
                self.vr_fetch_files().await;
                self.vr_save_files_state().await;
            }
            KeyCode::Char('S') => {
                self.viewer.files_sort_desc = !self.viewer.files_sort_desc;
                self.viewer.files_page_start = 0;
                self.viewer.files_selected = 0;
                self.vr_fetch_files().await;
                self.vr_save_files_state().await;
            }
            KeyCode::Char('c') => {
                self.vr_files_fetch_shareables().await;
                self.viewer.files_layout_popup_mode = LayoutPopupMode::List;
                self.viewer.files_layout_popup_selected = 0;
                self.viewer.files_layout_filter.clear();
                self.viewer.files_layout_filter_cursor = 0;
                self.viewer.files_show_layout_popup = true;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_files_column_editor_key(&mut self, key: KeyEvent) {
        let mut state = ColumnEditorState {
            filter: &mut self.viewer.files_column_editor_filter,
            filter_cursor: &mut self.viewer.files_column_editor_filter_cursor,
            selected: &mut self.viewer.files_column_editor_selected,
            mode: &mut self.viewer.files_column_editor_mode,
            show: &mut self.viewer.files_show_column_editor,
        };
        let action = handle_column_editor_key_generic(
            key, &mut state, &mut self.viewer.files_column_editor_items, &mut self.show_help,
        );
        match action {
            Some("apply") => {
                self.vr_files_apply_column_editor();
                self.viewer.files_show_column_editor = false;
                self.viewer.files_page_start = 0;
                self.viewer.files_selected = 0;
                self.vr_fetch_files().await;
                self.vr_save_files_state().await;
            }
            Some("default") => {
                self.vr_files_reset_default_columns();
                self.viewer.files_show_column_editor = false;
                self.viewer.files_page_start = 0;
                self.viewer.files_selected = 0;
                self.vr_fetch_files().await;
                self.vr_save_files_state().await;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_files_layout_popup_key(&mut self, key: KeyEvent) {
        let items: Vec<LayoutItem> = self.viewer.files_saved_shareables.iter()
            .map(|s| LayoutItem { name: s.name.clone(), shared: s.shared })
            .collect();
        let mut delete_name_for_id = String::new();
        let mut state = LayoutPopupState {
            mode: &mut self.viewer.files_layout_popup_mode,
            selected: &mut self.viewer.files_layout_popup_selected,
            filter: &mut self.viewer.files_layout_filter,
            filter_cursor: &mut self.viewer.files_layout_filter_cursor,
            save_name: &mut self.viewer.files_layout_save_name,
            save_cursor: &mut self.viewer.files_layout_save_cursor,
            show: &mut self.viewer.files_show_layout_popup,
            delete_name: &mut delete_name_for_id,
        };
        let action = handle_layout_popup_key_generic(key, &mut state, &items, &mut self.show_help);

        if let Some(cmd) = action {
            if cmd == "edit" {
                self.vr_files_build_column_editor();
                self.viewer.files_show_layout_popup = false;
                self.viewer.files_show_column_editor = true;
            } else if cmd == "default" {
                self.vr_files_reset_default_columns();
                self.viewer.files_page_start = 0;
                self.viewer.files_selected = 0;
                self.vr_fetch_files().await;
                self.vr_save_files_state().await;
            } else if cmd.starts_with("confirm_delete:") {
                if let Some(s) = self.viewer.files_saved_shareables.iter().find(|s| s.name == delete_name_for_id) {
                    let id = s.id.clone();
                    self.vr_files_delete_shareable(&id).await;
                    let max = self.viewer.files_saved_shareables.len() + 3;
                    if self.viewer.files_layout_popup_selected >= max {
                        self.viewer.files_layout_popup_selected = max.saturating_sub(1);
                    }
                }
            } else if let Some(name) = cmd.strip_prefix("save:") {
                self.vr_files_save_shareable(name).await;
            } else if let Some(idx_str) = cmd.strip_prefix("select:")
                && let Ok(idx) = idx_str.parse::<usize>()
                    && let Some(shareable) = self.viewer.files_saved_shareables.get(idx).cloned() {
                        self.vr_files_apply_shareable(&shareable);
                        self.viewer.files_page_start = 0;
                        self.viewer.files_selected = 0;
                        self.vr_fetch_files().await;
                        self.vr_save_files_state().await;
                    }
        }
        self.viewer.files_layout_delete_name = delete_name_for_id;
    }

    pub(crate) fn handle_files_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.viewer.files_view = StatsView::List;
                self.viewer.files_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.viewer.files_detail {
                    detail.scroll = u16::MAX;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }
}
