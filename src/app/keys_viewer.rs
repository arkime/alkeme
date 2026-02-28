use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    pub(crate) fn open_action_menu(&mut self, target: ActionTarget) {
        let (session_id, session_node) = match target {
            ActionTarget::Single => {
                let (id, node) = if self.vr_session_view == SessionView::Detail {
                    let detail = self.vr_session_detail.as_ref();
                    (
                        detail.and_then(|d| d.data.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        detail.and_then(|d| d.data.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                } else {
                    let session = self.vr_sessions.get(self.vr_selected_session);
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
                if self.active_tab == Tab::Stats && self.vr_stats_data.is_empty() {
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.vr_stats_data.is_empty() {
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.vr_sessions.is_empty() {
                    self.vr_selected_session = (self.vr_selected_session + self.visible_rows).min(self.vr_sessions.len() - 1);
                    self.vr_table_state.select(Some(self.vr_selected_session));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.vr_selected_session = self.vr_selected_session.saturating_sub(self.visible_rows);
                self.vr_table_state.select(Some(self.vr_selected_session));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.vr_sessions.is_empty() {
                    self.vr_selected_session = (self.vr_selected_session + 1).min(self.vr_sessions.len() - 1);
                    self.vr_table_state.select(Some(self.vr_selected_session));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.vr_selected_session > 0 {
                    self.vr_selected_session -= 1;
                    self.vr_table_state.select(Some(self.vr_selected_session));
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
                self.time_range = self.time_range.next();
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('T') => {
                self.time_range = self.time_range.prev();
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('s') => {
                self.vr_sort_column = (self.vr_sort_column + 1) % self.vr_session_fields.len();
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('S') => {
                self.vr_sort_desc = !self.vr_sort_desc;
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.vr_sessions_filtered > self.vr_page_size {
                    let last_page = (self.vr_sessions_filtered - 1) / self.vr_page_size * self.vr_page_size;
                    if self.vr_page_start != last_page {
                        self.vr_page_start = last_page;
                        self.vr_fetch_sessions().await;
                    }
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.vr_page_start > 0 {
                    self.vr_page_start = 0;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Right => {
                let next = self.vr_page_start + self.vr_page_size;
                if next < self.vr_sessions_filtered {
                    self.vr_page_start = next;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Left => {
                if self.vr_page_start > 0 {
                    self.vr_page_start = self.vr_page_start.saturating_sub(self.vr_page_size);
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Home => {
                if self.vr_page_start > 0 {
                    self.vr_page_start = 0;
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Char('g') => {
                let was_off = !self.vr_graph_size.is_visible();
                self.vr_graph_size = self.vr_graph_size.next();
                if was_off && self.vr_graph_size.is_visible() {
                    self.vr_fetch_sessions().await;
                }
            }
            KeyCode::Char('G') => {
                if self.vr_graph_size.is_visible() {
                    self.vr_graph_type = self.vr_graph_type.next();
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
                self.vr_layout_popup_mode = LayoutPopupMode::List;
                self.vr_layout_popup_selected = 0;
                self.vr_layout_filter.clear();
                self.vr_show_layout_popup = true;
            }
            KeyCode::Char('v') => {
                self.open_view_popup().await;
            }
            _ => {}
        }
    }

    fn column_editor_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.vr_column_editor_filter.trim_matches('\0');
        if filter_text.is_empty() {
            return (0..self.vr_column_editor_available.len()).collect();
        }
        let filter = filter_text.to_lowercase();
        self.vr_column_editor_available.iter().enumerate()
            .filter(|(_, item)| {
                item.exp.to_lowercase().contains(&filter)
                    || item.friendly_name.to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) async fn handle_column_editor_key(&mut self, key: KeyEvent) {
        let filtered = self.column_editor_filtered_indices();
        let cur_pos = filtered.iter().position(|&i| i == self.vr_column_editor_selected);

        // When filter is active, route typing keys to filter input
        if !self.vr_column_editor_filter.is_empty() {
            match key.code {
                KeyCode::Esc => {
                    self.vr_column_editor_filter.clear();
                    self.vr_column_editor_selected = 0;
                    return;
                }
                KeyCode::Backspace => {
                    self.vr_column_editor_filter.pop();
                    let filtered = self.column_editor_filtered_indices();
                    if !filtered.is_empty() {
                        self.vr_column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Enter => {
                    // Toggle selected field
                    if let Some(item) = self.vr_column_editor_available.get_mut(self.vr_column_editor_selected) {
                        item.enabled = !item.enabled;
                    }
                    return;
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = self.vr_column_editor_available.get_mut(self.vr_column_editor_selected) {
                        item.enabled = !item.enabled;
                    }
                    return;
                }
                KeyCode::Down => {
                    if let Some(pos) = cur_pos {
                        if pos + 1 < filtered.len() {
                            self.vr_column_editor_selected = filtered[pos + 1];
                        }
                    } else if !filtered.is_empty() {
                        self.vr_column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(pos) = cur_pos {
                        if pos > 0 {
                            self.vr_column_editor_selected = filtered[pos - 1];
                        }
                    } else if !filtered.is_empty() {
                        self.vr_column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    self.vr_column_editor_filter.push(c);
                    let filtered = self.column_editor_filtered_indices();
                    if !filtered.is_empty() {
                        self.vr_column_editor_selected = filtered[0];
                    }
                    return;
                }
                _ => { return; }
            }
        }

        // Normal mode (no filter active)
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.vr_show_column_editor = false;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('/') => {
                self.vr_column_editor_filter = String::new();
                // Set filter to empty string — but we need a sentinel to indicate "filter mode active"
                // Use a special state: push empty and check in the filter-active branch above
                // Actually, just set it to a placeholder that gets replaced on first char
                self.vr_column_editor_filter = "\0".to_string(); // sentinel for "filter mode on, no chars yet"
            }
            KeyCode::Enter => {
                if self.vr_column_editor_mode == ColumnEditorMode::Reorder {
                    self.vr_column_editor_mode = ColumnEditorMode::Browse;
                } else if let Some(item) = self.vr_column_editor_available.get_mut(self.vr_column_editor_selected) {
                    item.enabled = !item.enabled;
                }
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self.vr_column_editor_available.get_mut(self.vr_column_editor_selected) {
                    item.enabled = !item.enabled;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(pos) = cur_pos {
                    let new_pos = (pos + 10).min(filtered.len().saturating_sub(1));
                    self.vr_column_editor_selected = filtered[new_pos];
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(pos) = cur_pos {
                    let new_pos = pos.saturating_sub(10);
                    self.vr_column_editor_selected = filtered[new_pos];
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.vr_column_editor_mode == ColumnEditorMode::Reorder {
                    let len = self.vr_column_editor_available.len();
                    if self.vr_column_editor_selected + 1 < len {
                        self.vr_column_editor_available.swap(self.vr_column_editor_selected, self.vr_column_editor_selected + 1);
                        self.vr_column_editor_selected += 1;
                    }
                } else if let Some(pos) = cur_pos {
                    if pos + 1 < filtered.len() {
                        self.vr_column_editor_selected = filtered[pos + 1];
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.vr_column_editor_mode == ColumnEditorMode::Reorder {
                    if self.vr_column_editor_selected > 0 {
                        self.vr_column_editor_available.swap(self.vr_column_editor_selected, self.vr_column_editor_selected - 1);
                        self.vr_column_editor_selected -= 1;
                    }
                } else if let Some(pos) = cur_pos {
                    if pos > 0 {
                        self.vr_column_editor_selected = filtered[pos - 1];
                    }
                }
            }
            KeyCode::Char('m') => {
                if self.vr_column_editor_mode == ColumnEditorMode::Reorder {
                    self.vr_column_editor_mode = ColumnEditorMode::Browse;
                } else {
                    self.vr_column_editor_mode = ColumnEditorMode::Reorder;
                }
            }
            KeyCode::Char('a') => {
                self.vr_apply_column_editor();
                self.vr_show_column_editor = false;
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            KeyCode::Char('d') => {
                self.vr_columns = default_columns();
                self.vr_sync_session_fields();
                self.vr_show_column_editor = false;
                self.vr_page_start = 0;
                self.vr_fetch_sessions().await;
            }
            _ => {}
        }
    }

    fn layout_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.vr_layout_filter.trim_matches('\0');
        if filter_text.is_empty() {
            return (0..self.vr_saved_layouts.len()).collect();
        }
        let filter = filter_text.to_lowercase();
        self.vr_saved_layouts.iter().enumerate()
            .filter(|(_, l)| l.name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) async fn handle_layout_popup_key(&mut self, key: KeyEvent) {
        match self.vr_layout_popup_mode {
            LayoutPopupMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let name = self.vr_layout_delete_name.clone();
                        match self.client.vr_delete_layout(&name).await {
                            Ok(_) => {
                                self.vr_saved_layouts.retain(|l| l.name != name);
                                self.status_msg = format!("Deleted layout '{name}'");
                                let max = self.vr_saved_layouts.len() + 3;
                                if self.vr_layout_popup_selected >= max {
                                    self.vr_layout_popup_selected = max.saturating_sub(1);
                                }
                            }
                            Err(e) => self.status_msg = format!("Error deleting layout: {e}"),
                        }
                        self.vr_layout_popup_mode = LayoutPopupMode::List;
                    }
                    _ => {
                        self.vr_layout_popup_mode = LayoutPopupMode::List;
                    }
                }
            }
            LayoutPopupMode::List => {
                // Filter mode active
                if !self.vr_layout_filter.is_empty() {
                    let filtered = self.layout_filtered_indices();
                    let cur_pos = filtered.iter().position(|&i| i + 3 == self.vr_layout_popup_selected);
                    match key.code {
                        KeyCode::Esc => {
                            self.vr_layout_filter.clear();
                            self.vr_layout_popup_selected = 0;
                        }
                        KeyCode::Backspace => {
                            self.vr_layout_filter.pop();
                            if self.vr_layout_filter.is_empty() || self.vr_layout_filter == "\0" {
                                self.vr_layout_filter.clear();
                                self.vr_layout_popup_selected = 0;
                            } else {
                                let filtered = self.layout_filtered_indices();
                                if let Some(&first) = filtered.first() {
                                    self.vr_layout_popup_selected = first + 3;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(&idx) = filtered.iter().find(|&&i| i + 3 == self.vr_layout_popup_selected) {
                                if let Some(layout) = self.vr_saved_layouts.get(idx).cloned() {
                                    self.vr_apply_layout(&layout);
                                    self.vr_show_layout_popup = false;
                                    self.vr_layout_filter.clear();
                                    self.vr_page_start = 0;
                                    self.vr_fetch_sessions().await;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let Some(pos) = cur_pos {
                                if pos + 1 < filtered.len() {
                                    self.vr_layout_popup_selected = filtered[pos + 1] + 3;
                                }
                            } else if let Some(&first) = filtered.first() {
                                self.vr_layout_popup_selected = first + 3;
                            }
                        }
                        KeyCode::Up => {
                            if let Some(pos) = cur_pos {
                                if pos > 0 {
                                    self.vr_layout_popup_selected = filtered[pos - 1] + 3;
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            self.vr_layout_filter.push(c);
                            let filtered = self.layout_filtered_indices();
                            if let Some(&first) = filtered.first() {
                                self.vr_layout_popup_selected = first + 3;
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // Normal list mode
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.vr_show_layout_popup = false;
                    }
                    KeyCode::Char('h') | KeyCode::Char('?') => {
                        self.show_help = !self.show_help;
                    }
                    KeyCode::Char('/') => {
                        self.vr_layout_filter = "\0".to_string();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max = self.vr_saved_layouts.len() + 3;
                        if self.vr_layout_popup_selected + 1 < max {
                            self.vr_layout_popup_selected += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.vr_layout_popup_selected = self.vr_layout_popup_selected.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if self.vr_layout_popup_selected == 0 {
                            // Edit Columns
                            self.vr_build_column_editor();
                            self.vr_show_layout_popup = false;
                            self.vr_show_column_editor = true;
                        } else if self.vr_layout_popup_selected == 1 {
                            self.vr_layout_popup_mode = LayoutPopupMode::SaveInput;
                            self.vr_layout_save_name.clear();
                            self.vr_layout_save_cursor = 0;
                        } else if self.vr_layout_popup_selected == 2 {
                            self.vr_columns = default_columns();
                            self.vr_sync_session_fields();
                            self.vr_show_layout_popup = false;
                            self.vr_page_start = 0;
                            self.vr_fetch_sessions().await;
                        } else {
                            let idx = self.vr_layout_popup_selected - 3;
                            if let Some(layout) = self.vr_saved_layouts.get(idx).cloned() {
                                self.vr_apply_layout(&layout);
                                self.vr_show_layout_popup = false;
                                self.vr_page_start = 0;
                                self.vr_fetch_sessions().await;
                            }
                        }
                    }
                    KeyCode::Char('x') | KeyCode::Delete => {
                        if let Some(idx) = self.vr_layout_popup_selected.checked_sub(3) {
                            if let Some(layout) = self.vr_saved_layouts.get(idx) {
                                self.vr_layout_delete_name = layout.name.clone();
                                self.vr_layout_popup_mode = LayoutPopupMode::ConfirmDelete;
                            }
                        }
                    }
                    _ => {}
                }
            }
            LayoutPopupMode::SaveInput => {
                match key.code {
                    KeyCode::Esc => {
                        self.vr_layout_popup_mode = LayoutPopupMode::List;
                    }
                    KeyCode::Enter => {
                        if !self.vr_layout_save_name.is_empty() {
                            let name = self.vr_layout_save_name.clone();
                            let columns: Vec<String> = self.vr_columns.iter()
                                .enumerate()
                                .filter(|(i, c)| !(*i == 0 && c.field == "ipProtocol"))
                                .map(|(_, c)| c.field.clone()).collect();
                            let sort_field = self.vr_columns.get(self.vr_sort_column)
                                .map(|c| c.field.clone())
                                .unwrap_or_default();
                            let sort_dir = if self.vr_sort_desc { "desc" } else { "asc" };
                            self.status_msg = format!("Saving layout '{name}' with {} columns...", columns.len());
                            // Check if layout name already exists → update, else create
                            let exists = self.vr_saved_layouts.iter().any(|l| l.name == name);
                            let result = if exists {
                                self.client.vr_update_layout(&name, &columns, &sort_field, sort_dir).await
                            } else {
                                self.client.vr_create_layout(&name, &columns, &sort_field, sort_dir).await
                            };
                            match result {
                                Ok(_) => {
                                    self.status_msg = format!("Saved layout '{name}'");
                                    self.vr_fetch_layouts().await;
                                }
                                Err(e) => self.status_msg = format!("Error saving layout: {e}"),
                            }
                            self.vr_show_layout_popup = false;
                        }
                    }
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Right | KeyCode::Char(_) => {
                        handle_text_input_key(key.code, &mut self.vr_layout_save_name, &mut self.vr_layout_save_cursor);
                    }
                    _ => {}
                }
            }
        }
    }

    async fn open_view_popup(&mut self) {
        self.status_msg = "Fetching views...".into();
        match self.client.vr_get_views().await {
            Ok(views) => {
                self.vr_saved_views = views;
                self.status_msg = String::new();
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
        self.vr_view_popup_mode = ViewPopupMode::List;
        self.vr_view_popup_selected = 0;
        self.vr_view_filter.clear();
        self.vr_view_filter_active = false;
        self.vr_show_view_popup = true;
    }

    pub fn view_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.vr_view_filter.to_lowercase();
        if filter_text.is_empty() {
            return (0..self.vr_saved_views.len()).collect();
        }
        self.vr_saved_views.iter().enumerate()
            .filter(|(_, v)| v.name.to_lowercase().contains(&filter_text) || v.expression.to_lowercase().contains(&filter_text))
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) async fn handle_view_popup_key(&mut self, key: KeyEvent) {
        match self.vr_view_popup_mode {
            ViewPopupMode::SaveInput => {
                match key.code {
                    KeyCode::Enter => {
                        let name = self.vr_view_save_name.trim().to_string();
                        if !name.is_empty() && !self.expression.is_empty() {
                            let col_config = if self.vr_view_save_columns {
                                let cols: Vec<String> = self.vr_columns.iter().map(|c| c.exp.clone()).collect();
                                let sort_field = self.vr_session_fields.get(self.vr_sort_column)
                                    .cloned().unwrap_or_else(|| "firstPacket".into());
                                let sort_dir = if self.vr_sort_desc { "desc" } else { "asc" };
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
                                    self.vr_active_view = Some(view_id);
                                    self.vr_active_view_name = Some(name);
                                    self.vr_page_start = 0;
                                    self.vr_show_view_popup = false;
                                    self.refresh_for_active_tab().await;
                                }
                                Err(e) => self.status_msg = format!("Error creating view: {e}"),
                            }
                        } else if self.expression.is_empty() {
                            self.status_msg = "Cannot save view: expression is empty".into();
                        }
                        self.vr_view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Esc => {
                        self.vr_view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Tab => {
                        self.vr_view_save_columns = !self.vr_view_save_columns;
                    }
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Right | KeyCode::Char(_) => {
                        handle_text_input_key(key.code, &mut self.vr_view_save_name, &mut self.vr_view_save_cursor);
                    }
                    _ => {}
                }
            }
            ViewPopupMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let id = self.vr_view_delete_id.clone();
                        let name = self.vr_view_delete_name.clone();
                        match self.client.vr_delete_view(&id).await {
                            Ok(_) => {
                                self.status_msg = format!("View '{}' deleted", name);
                                if self.vr_active_view.as_deref() == Some(&id) {
                                    self.vr_active_view = None;
                                    self.vr_active_view_name = None;
                                }
                                self.vr_saved_views.retain(|v| v.id != id);
                                self.vr_view_popup_selected = 0;
                            }
                            Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                        }
                        self.vr_view_popup_mode = ViewPopupMode::List;
                    }
                    _ => {
                        self.vr_view_popup_mode = ViewPopupMode::List;
                    }
                }
            }
            ViewPopupMode::List => {
                let filtered = self.view_filtered_indices();
                let total_items = 2 + filtered.len(); // 0=Save, 1=Clear, 2+=views
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        if total_items > 0 {
                            self.vr_view_popup_selected = (self.vr_view_popup_selected + 1).min(total_items - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.vr_view_popup_selected = self.vr_view_popup_selected.saturating_sub(1);
                    }
                    KeyCode::Char('x') => {
                        if self.vr_view_popup_selected >= 2 {
                            let fi = self.vr_view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view = &self.vr_saved_views[idx];
                                if !view.shared {
                                    self.vr_view_delete_id = view.id.clone();
                                    self.vr_view_delete_name = view.name.clone();
                                    self.vr_view_popup_mode = ViewPopupMode::ConfirmDelete;
                                } else {
                                    self.status_msg = "Cannot delete shared views".into();
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if self.vr_view_popup_selected == 0 {
                            // Save current expression as view
                            if self.expression.is_empty() {
                                self.status_msg = "Cannot save view: expression is empty".into();
                            } else {
                                self.vr_view_save_name.clear();
                                self.vr_view_save_cursor = 0;
                                self.vr_view_save_columns = false;
                                self.vr_view_popup_mode = ViewPopupMode::SaveInput;
                            }
                        } else if self.vr_view_popup_selected == 1 {
                            // Clear view
                            if self.vr_active_view.is_some() {
                                self.vr_active_view = None;
                                self.vr_active_view_name = None;
                                self.vr_page_start = 0;
                                self.vr_show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            } else {
                                self.vr_show_view_popup = false;
                            }
                        } else {
                            // Select a view
                            let fi = self.vr_view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view_id = self.vr_saved_views[idx].id.clone();
                                let view_name = self.vr_saved_views[idx].name.clone();
                                self.vr_active_view = Some(view_id);
                                self.vr_active_view_name = Some(view_name);
                                self.vr_page_start = 0;
                                self.vr_show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if self.vr_view_filter_active {
                            self.vr_view_filter.clear();
                            self.vr_view_filter_active = false;
                        } else {
                            self.vr_show_view_popup = false;
                        }
                    }
                    KeyCode::Char('/') => {
                        if !self.vr_view_filter_active {
                            self.vr_view_filter_active = true;
                            self.vr_view_filter.clear();
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.vr_view_filter_active {
                            self.vr_view_filter.push(c);
                            self.vr_view_popup_selected = 2; // reset to first view
                        }
                    }
                    KeyCode::Backspace => {
                        if self.vr_view_filter_active {
                            self.vr_view_filter.pop();
                            if self.vr_view_filter.is_empty() {
                                self.vr_view_filter_active = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) async fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.vr_session_view = SessionView::List;
                self.vr_session_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.vr_session_detail
                    && detail.total_rows > 0 && detail.selected < detail.total_rows - 1 {
                        detail.selected += 1;
                    }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.vr_session_detail
                    && detail.selected > 0 {
                        detail.selected -= 1;
                    }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.vr_session_detail {
                    detail.selected = detail.total_rows.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.vr_session_detail
                    && let Some(obj) = detail.data.as_object() {
                        let filter_lower = detail.filter.to_lowercase();
                        let mut keys: Vec<&String> = obj.keys()
                            .filter(|k| !is_hidden_detail_field(k))
                            .filter(|k| {
                                if filter_lower.is_empty() {
                                    return true;
                                }
                                let friendly = self.vr_field_friendly_map.get(k.as_str())
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
                                    if items.len() == 1 {
                                        (items[0].clone(), None)
                                    } else {
                                        (items[0].clone(), Some(items))
                                    }
                                }
                                serde_json::Value::Null => ("-".into(), None),
                                other => (other.to_string(), None),
                            };
                            let exp_name = self.vr_field_exp_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            let friendly = self.vr_field_friendly_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            self.vr_detail_action_menu = Some(DetailActionMenu {
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
        let is_c3_detail = self.app_mode == crate::app::AppMode::Cont3xt && self.active_tab == Tab::Search;
        match key.code {
            KeyCode::Esc => {
                if is_c3_detail {
                    self.c3_detail_filter.clear();
                    self.c3_detail_scroll = 0;
                } else if is_stats {
                    if let Some(ref mut detail) = self.vr_stats_detail {
                        detail.filter.clear();
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.vr_session_detail {
                    detail.filter.clear();
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) => {
                if is_c3_detail {
                    self.c3_detail_filter.push(c);
                    self.c3_detail_scroll = 0;
                } else if is_stats {
                    if let Some(ref mut detail) = self.vr_stats_detail {
                        detail.filter.push(c);
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.vr_session_detail {
                    detail.filter.push(c);
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
            }
            KeyCode::Backspace => {
                if is_c3_detail {
                    self.c3_detail_filter.pop();
                    self.c3_detail_scroll = 0;
                } else if is_stats {
                    if let Some(ref mut detail) = self.vr_stats_detail {
                        detail.filter.pop();
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.vr_session_detail {
                    detail.filter.pop();
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
            }
            _ => {}
        }
    }

    fn recalc_detail_rows(&mut self) {
        if let Some(ref mut detail) = self.vr_session_detail
            && let Some(obj) = detail.data.as_object() {
                let filter_lower = detail.filter.to_lowercase();
                detail.total_rows = obj.keys()
                    .filter(|k| !is_hidden_detail_field(k))
                    .filter(|k| {
                        if filter_lower.is_empty() {
                            return true;
                        }
                        let friendly = self.vr_field_friendly_map.get(k.as_str())
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
            KeyCode::Char(c) => {
                if let Some(ref mut prompt) = self.action_prompt {
                    prompt.input.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut prompt) = self.action_prompt {
                    prompt.input.pop();
                }
            }
            _ => {}
        }
    }

    fn visible_session_ids(&self) -> Vec<String> {
        self.vr_sessions.iter()
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
                    self.client.vr_download_sessions_pcap(&self.expression, date, &self.vr_active_view).await
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
                    self.client.vr_export_sessions_csv_ids(&ids, &self.vr_session_fields).await
                } else {
                    self.client.vr_export_sessions_csv(&self.expression, date, &self.vr_session_fields, &self.vr_active_view).await
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
                match self.client.vr_add_sessions_tags(&self.expression, date, &prompt.input, &self.vr_active_view).await {
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
                match self.client.vr_remove_sessions_tags(&self.expression, date, &prompt.input, &self.vr_active_view).await {
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
        let in_values = self.vr_detail_action_menu.as_ref()
            .map(|m| m.values.is_some()).unwrap_or(false);

        match key.code {
            KeyCode::Esc => {
                self.vr_detail_action_menu = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut menu) = self.vr_detail_action_menu {
                    if in_values {
                        let len = menu.values.as_ref().unwrap().len();
                        menu.value_selected = (menu.value_selected + 1).min(len - 1);
                    } else {
                        menu.selected = (menu.selected + 1).min(DetailActionMenu::OPTIONS.len() - 1);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut menu) = self.vr_detail_action_menu {
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
                    if let Some(ref mut menu) = self.vr_detail_action_menu {
                        let chosen = menu.values.as_ref().unwrap()[menu.value_selected].clone();
                        menu.value = chosen;
                        menu.values = None;
                        menu.selected = 0;
                    }
                } else if let Some(menu) = self.vr_detail_action_menu.take() {
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
                self.vr_packets_view = None;
                self.vr_packets_scroll = 0;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::PageUp => {
                self.vr_packets_scroll = self.vr_packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Home | KeyCode::Left => {
                self.vr_packets_scroll = 0;
            }
            KeyCode::Right => {
                self.vr_packets_scroll = u16::MAX;
            }
            KeyCode::Char('r') => {
                self.vr_packets_raw = !self.vr_packets_raw;
                self.request_packets();
            }
            KeyCode::Char('l') => {
                self.vr_packets_line = self.vr_packets_line.next();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub fn request_packets(&mut self) {
        if let Some(session) = self.vr_sessions.get(self.vr_selected_session) {
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
            self.vr_packets_total_pending = total;
            self.vr_packets_node_pending = node.to_string();
            self.vr_packets_id_pending = id.to_string();
            self.status_msg = "Fetching packets...".into();
            if total > 500 {
                self.show_loading = true;
            }
            self.vr_pending_packets_fetch = true;
        }
    }

    pub(crate) async fn handle_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
            }
            KeyCode::BackTab => {
                self.prev_tab();
            }
            KeyCode::Char('1') => {
                if self.vr_stats_tab != StatsTab::Capture {
                    self.vr_stats_tab = StatsTab::Capture;
                    self.vr_stats_sort_column = 0;
                    self.vr_stats_sort_desc = false;
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::Char('2') => {
                if self.vr_stats_tab != StatsTab::DBStats {
                    self.vr_stats_tab = StatsTab::DBStats;
                    self.vr_stats_sort_column = 0;
                    self.vr_stats_sort_desc = false;
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::Char('3') => {
                if self.vr_stats_tab != StatsTab::DBIndices {
                    self.vr_stats_tab = StatsTab::DBIndices;
                    self.vr_stats_sort_column = 0;
                    self.vr_stats_sort_desc = false;
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.vr_stats_data.is_empty() {
                    self.vr_stats_selected = (self.vr_stats_selected + 1).min(self.vr_stats_data.len() - 1);
                    self.vr_stats_table_state.select(Some(self.vr_stats_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.vr_stats_selected > 0 {
                    self.vr_stats_selected -= 1;
                    self.vr_stats_table_state.select(Some(self.vr_stats_selected));
                }
            }
            KeyCode::Enter => {
                self.vr_open_stats_detail();
            }
            KeyCode::Char('r') => {
                self.vr_fetch_stats().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.vr_stats_filter_edit = self.vr_stats_filter.clone();
                self.expression_cursor = self.vr_stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let num_cols = self.vr_stats_tab.columns().len();
                self.vr_stats_sort_column = (self.vr_stats_sort_column + 1) % num_cols;
                self.vr_fetch_stats().await;
            }
            KeyCode::Char('S') => {
                self.vr_stats_sort_desc = !self.vr_stats_sort_desc;
                self.vr_fetch_stats().await;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_stats_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.vr_stats_view = StatsView::List;
                self.vr_stats_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.vr_stats_detail {
                    detail.scroll = u16::MAX;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('E') => {
                self.vr_stats_filter_edit = self.vr_stats_filter.clone();
                self.expression_cursor = self.vr_stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
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
                if self.active_tab == Tab::Stats && self.vr_stats_data.is_empty() {
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.vr_stats_data.is_empty() {
                    self.vr_fetch_stats().await;
                }
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.enter_expression_mode();
            }
            KeyCode::Char('f') => {
                self.vr_field_filter.clear();
                self.vr_field_filter_selected = 0;
                self.input_mode = InputMode::FieldSelector;
            }
            KeyCode::Char('G') => {
                self.vr_summary_metric = self.vr_summary_metric.next();
            }
            KeyCode::Char('s') => {
                self.vr_summary_sort = self.vr_summary_sort.next();
                self.vr_sort_summary_data();
            }
            KeyCode::Char('S') => {
                self.vr_summary_sort_desc = !self.vr_summary_sort_desc;
                self.vr_sort_summary_data();
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.vr_summary_data.is_empty() {
                    self.vr_summary_selected = (self.vr_summary_selected + self.visible_rows).min(self.vr_summary_data.len() - 1);
                    self.vr_summary_table_state.select(Some(self.vr_summary_selected));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.vr_summary_selected = self.vr_summary_selected.saturating_sub(self.visible_rows);
                self.vr_summary_table_state.select(Some(self.vr_summary_selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.vr_summary_data.is_empty() {
                    self.vr_summary_selected = (self.vr_summary_selected + 1).min(self.vr_summary_data.len() - 1);
                    self.vr_summary_table_state.select(Some(self.vr_summary_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.vr_summary_selected > 0 {
                    self.vr_summary_selected -= 1;
                    self.vr_summary_table_state.select(Some(self.vr_summary_selected));
                }
            }
            KeyCode::PageDown => {
                if !self.vr_summary_data.is_empty() {
                    self.vr_summary_selected = (self.vr_summary_selected + self.visible_rows).min(self.vr_summary_data.len() - 1);
                    self.vr_summary_table_state.select(Some(self.vr_summary_selected));
                }
            }
            KeyCode::PageUp => {
                self.vr_summary_selected = self.vr_summary_selected.saturating_sub(self.visible_rows);
                self.vr_summary_table_state.select(Some(self.vr_summary_selected));
            }
            KeyCode::Left | KeyCode::Home => {
                self.vr_summary_selected = 0;
                self.vr_summary_table_state.select(Some(self.vr_summary_selected));
            }
            KeyCode::Right | KeyCode::End => {
                if !self.vr_summary_data.is_empty() {
                    self.vr_summary_selected = self.vr_summary_data.len() - 1;
                    self.vr_summary_table_state.select(Some(self.vr_summary_selected));
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.vr_summary_data.get(self.vr_summary_selected) {
                    let value = match &item.item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    self.vr_detail_action_menu = Some(DetailActionMenu {
                        field: self.vr_summary_field.clone(),
                        display: self.vr_summary_field.clone(),
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
                self.time_range = self.time_range.next();
                self.vr_request_summary_fetch();
            }
            KeyCode::Char('T') => {
                self.time_range = self.time_range.prev();
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
                self.vr_field_filter.clear();
            }
            KeyCode::Enter => {
                let filtered = self.vr_filtered_fields();
                if let Some(field) = filtered.get(self.vr_field_filter_selected) {
                    self.vr_summary_field = field.exp.clone();
                    self.input_mode = InputMode::Normal;
                    self.vr_field_filter.clear();
                    self.vr_request_summary_fetch();
                }
            }
            KeyCode::Down => {
                let count = self.vr_filtered_fields().len();
                if count > 0 {
                    self.vr_field_filter_selected = (self.vr_field_filter_selected + 1).min(count - 1);
                }
            }
            KeyCode::Up => {
                if self.vr_field_filter_selected > 0 {
                    self.vr_field_filter_selected -= 1;
                }
            }
            KeyCode::Char(c) => {
                self.vr_field_filter.push(c);
                self.vr_field_filter_selected = 0;
            }
            KeyCode::Backspace => {
                self.vr_field_filter.pop();
                self.vr_field_filter_selected = 0;
            }
            _ => {}
        }
    }
}
