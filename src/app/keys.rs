use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    fn open_action_menu(&mut self, target: ActionTarget) {
        let (session_id, session_node) = match target {
            ActionTarget::Single => {
                let (id, node) = if self.session_view == SessionView::Detail {
                    let detail = self.session_detail.as_ref();
                    (
                        detail.and_then(|d| d.data.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        detail.and_then(|d| d.data.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                } else {
                    let session = self.sessions.get(self.selected_session);
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

    pub async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.show_debug {
            match key.code {
                KeyCode::Esc | KeyCode::Char('D') | KeyCode::Char('q') => self.show_debug = false,
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => self.debug_scroll = self.debug_scroll.saturating_sub(10),
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => self.debug_scroll += 10,
                KeyCode::Up | KeyCode::Char('k') => self.debug_scroll = self.debug_scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.debug_scroll += 1,
                KeyCode::Home => self.debug_scroll = 0,
                _ => {}
            }
            return;
        }
        if self.action_menu.is_some() {
            self.handle_action_menu_key(key);
            return;
        }
        if self.input_mode == InputMode::ActionPrompt {
            self.handle_action_prompt_key(key).await;
            return;
        }
        if self.input_mode == InputMode::DetailFilter {
            self.handle_detail_filter_key(key);
            return;
        }
        if self.detail_action_menu.is_some() {
            self.handle_detail_action_key(key).await;
            return;
        }
        if self.input_mode == InputMode::FieldSelector {
            self.handle_field_selector_key(key).await;
            return;
        }
        if self.packets_view.is_some() {
            self.handle_packets_key(key);
            return;
        }
        if self.input_mode == InputMode::Expression {
            self.handle_expression_key(key).await;
            return;
        }
        if self.show_column_editor {
            self.handle_column_editor_key(key).await;
            return;
        }
        if self.show_layout_popup {
            self.handle_layout_popup_key(key).await;
            return;
        }
        if self.show_view_popup {
            self.handle_view_popup_key(key).await;
            return;
        }
        if key.code == KeyCode::Char('D') {
            self.show_debug = true;
            self.debug_scroll = 0;
            return;
        }
        match self.app_mode {
            crate::app::AppMode::Viewer => {
                match self.active_tab {
                    Tab::Stats => {
                        match self.stats_view {
                            StatsView::List => self.handle_stats_key(key).await,
                            StatsView::Detail => self.handle_stats_detail_key(key),
                        }
                    }
                    Tab::Arkime => self.handle_arkime_key(key).await,
                    _ => {
                        match self.session_view {
                            SessionView::List => self.handle_list_key(key).await,
                            SessionView::Detail => self.handle_detail_key(key).await,
                        }
                    }
                }
            }
            crate::app::AppMode::Cont3xt => {
                self.handle_cont3xt_key(key).await;
            }
            _ => {
                // Placeholder modes: just tab switching
                match key.code {
                    KeyCode::Tab => self.next_tab(),
                    KeyCode::BackTab => self.prev_tab(),
                    KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
                    _ => {}
                }
            }
        }
    }

    async fn handle_expression_key(&mut self, key: KeyEvent) {
        let is_stats = self.active_tab == Tab::Stats;
        let edit = if is_stats { &mut self.stats_filter_edit } else { &mut self.expression_edit };
        match key.code {
            KeyCode::Enter => {
                if is_stats {
                    self.stats_filter = self.stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.fetch_stats().await;
                } else {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.page_start = 0;
                    match self.app_mode {
                        crate::app::AppMode::Cont3xt => {
                            self.request_c3_search();
                        }
                        _ => {
                            if self.active_tab == Tab::Arkime {
                                self.request_summary_fetch();
                            } else {
                                self.fetch_sessions().await;
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                if is_stats {
                    self.stats_filter_edit = self.stats_filter.clone();
                } else {
                    self.expression_edit = self.expression.clone();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Left => {
                if self.expression_cursor > 0 {
                    self.expression_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.expression_cursor < edit.len() {
                    self.expression_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.expression_cursor = 0;
            }
            KeyCode::End => {
                self.expression_cursor = edit.len();
            }
            KeyCode::Char(c) => {
                edit.insert(self.expression_cursor, c);
                self.expression_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.expression_cursor > 0 {
                    self.expression_cursor -= 1;
                    edit.remove(self.expression_cursor);
                }
            }
            KeyCode::Delete => {
                if self.expression_cursor < edit.len() {
                    edit.remove(self.expression_cursor);
                }
            }
            _ => {}
        }
    }

    async fn handle_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.sessions.is_empty() {
                    self.selected_session = (self.selected_session + self.visible_rows).min(self.sessions.len() - 1);
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.selected_session = self.selected_session.saturating_sub(self.visible_rows);
                self.table_state.select(Some(self.selected_session));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.sessions.is_empty() {
                    self.selected_session = (self.selected_session + 1).min(self.sessions.len() - 1);
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_session > 0 {
                    self.selected_session -= 1;
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Enter => {
                self.open_session_detail().await;
            }
            KeyCode::Char('r') => {
                self.fetch_sessions().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('t') => {
                self.time_range = self.time_range.next();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('T') => {
                self.time_range = self.time_range.prev();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('s') => {
                self.sort_column = (self.sort_column + 1) % self.session_fields.len();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('S') => {
                self.sort_desc = !self.sort_desc;
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.sessions_filtered > self.page_size {
                    let last_page = (self.sessions_filtered - 1) / self.page_size * self.page_size;
                    if self.page_start != last_page {
                        self.page_start = last_page;
                        self.fetch_sessions().await;
                    }
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.page_start > 0 {
                    self.page_start = 0;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Right => {
                let next = self.page_start + self.page_size;
                if next < self.sessions_filtered {
                    self.page_start = next;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Left => {
                if self.page_start > 0 {
                    self.page_start = self.page_start.saturating_sub(self.page_size);
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Home => {
                if self.page_start > 0 {
                    self.page_start = 0;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Char('g') => {
                let was_off = !self.graph_size.is_visible();
                self.graph_size = self.graph_size.next();
                if was_off && self.graph_size.is_visible() {
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Char('G') => {
                if self.graph_size.is_visible() {
                    self.graph_type = self.graph_type.next();
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
                self.fetch_layouts().await;
                self.layout_popup_mode = LayoutPopupMode::List;
                self.layout_popup_selected = 0;
                self.layout_filter.clear();
                self.show_layout_popup = true;
            }
            KeyCode::Char('v') => {
                self.open_view_popup().await;
            }
            _ => {}
        }
    }

    fn column_editor_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.column_editor_filter.trim_matches('\0');
        if filter_text.is_empty() {
            return (0..self.column_editor_available.len()).collect();
        }
        let filter = filter_text.to_lowercase();
        self.column_editor_available.iter().enumerate()
            .filter(|(_, item)| {
                item.exp.to_lowercase().contains(&filter)
                    || item.friendly_name.to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect()
    }

    async fn handle_column_editor_key(&mut self, key: KeyEvent) {
        let filtered = self.column_editor_filtered_indices();
        let cur_pos = filtered.iter().position(|&i| i == self.column_editor_selected);

        // When filter is active, route typing keys to filter input
        if !self.column_editor_filter.is_empty() {
            match key.code {
                KeyCode::Esc => {
                    self.column_editor_filter.clear();
                    self.column_editor_selected = 0;
                    return;
                }
                KeyCode::Backspace => {
                    self.column_editor_filter.pop();
                    let filtered = self.column_editor_filtered_indices();
                    if !filtered.is_empty() {
                        self.column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Enter => {
                    // Toggle selected field
                    if let Some(item) = self.column_editor_available.get_mut(self.column_editor_selected) {
                        item.enabled = !item.enabled;
                    }
                    return;
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = self.column_editor_available.get_mut(self.column_editor_selected) {
                        item.enabled = !item.enabled;
                    }
                    return;
                }
                KeyCode::Down => {
                    if let Some(pos) = cur_pos {
                        if pos + 1 < filtered.len() {
                            self.column_editor_selected = filtered[pos + 1];
                        }
                    } else if !filtered.is_empty() {
                        self.column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(pos) = cur_pos {
                        if pos > 0 {
                            self.column_editor_selected = filtered[pos - 1];
                        }
                    } else if !filtered.is_empty() {
                        self.column_editor_selected = filtered[0];
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    self.column_editor_filter.push(c);
                    let filtered = self.column_editor_filtered_indices();
                    if !filtered.is_empty() {
                        self.column_editor_selected = filtered[0];
                    }
                    return;
                }
                _ => { return; }
            }
        }

        // Normal mode (no filter active)
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.show_column_editor = false;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('/') => {
                self.column_editor_filter = String::new();
                // Set filter to empty string — but we need a sentinel to indicate "filter mode active"
                // Use a special state: push empty and check in the filter-active branch above
                // Actually, just set it to a placeholder that gets replaced on first char
                self.column_editor_filter = "\0".to_string(); // sentinel for "filter mode on, no chars yet"
            }
            KeyCode::Enter => {
                if self.column_editor_mode == ColumnEditorMode::Reorder {
                    self.column_editor_mode = ColumnEditorMode::Browse;
                } else if let Some(item) = self.column_editor_available.get_mut(self.column_editor_selected) {
                    item.enabled = !item.enabled;
                }
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self.column_editor_available.get_mut(self.column_editor_selected) {
                    item.enabled = !item.enabled;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(pos) = cur_pos {
                    let new_pos = (pos + 10).min(filtered.len().saturating_sub(1));
                    self.column_editor_selected = filtered[new_pos];
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(pos) = cur_pos {
                    let new_pos = pos.saturating_sub(10);
                    self.column_editor_selected = filtered[new_pos];
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.column_editor_mode == ColumnEditorMode::Reorder {
                    let len = self.column_editor_available.len();
                    if self.column_editor_selected + 1 < len {
                        self.column_editor_available.swap(self.column_editor_selected, self.column_editor_selected + 1);
                        self.column_editor_selected += 1;
                    }
                } else if let Some(pos) = cur_pos {
                    if pos + 1 < filtered.len() {
                        self.column_editor_selected = filtered[pos + 1];
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.column_editor_mode == ColumnEditorMode::Reorder {
                    if self.column_editor_selected > 0 {
                        self.column_editor_available.swap(self.column_editor_selected, self.column_editor_selected - 1);
                        self.column_editor_selected -= 1;
                    }
                } else if let Some(pos) = cur_pos {
                    if pos > 0 {
                        self.column_editor_selected = filtered[pos - 1];
                    }
                }
            }
            KeyCode::Char('m') => {
                if self.column_editor_mode == ColumnEditorMode::Reorder {
                    self.column_editor_mode = ColumnEditorMode::Browse;
                } else {
                    self.column_editor_mode = ColumnEditorMode::Reorder;
                }
            }
            KeyCode::Char('a') => {
                self.apply_column_editor();
                self.show_column_editor = false;
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('d') => {
                self.columns = default_columns();
                self.sync_session_fields();
                self.show_column_editor = false;
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            _ => {}
        }
    }

    fn layout_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.layout_filter.trim_matches('\0');
        if filter_text.is_empty() {
            return (0..self.saved_layouts.len()).collect();
        }
        let filter = filter_text.to_lowercase();
        self.saved_layouts.iter().enumerate()
            .filter(|(_, l)| l.name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    async fn handle_layout_popup_key(&mut self, key: KeyEvent) {
        match self.layout_popup_mode {
            LayoutPopupMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let name = self.layout_delete_name.clone();
                        match self.client.delete_layout(&name).await {
                            Ok(_) => {
                                self.saved_layouts.retain(|l| l.name != name);
                                self.status_msg = format!("Deleted layout '{name}'");
                                let max = self.saved_layouts.len() + 3;
                                if self.layout_popup_selected >= max {
                                    self.layout_popup_selected = max.saturating_sub(1);
                                }
                            }
                            Err(e) => self.status_msg = format!("Error deleting layout: {e}"),
                        }
                        self.layout_popup_mode = LayoutPopupMode::List;
                    }
                    _ => {
                        self.layout_popup_mode = LayoutPopupMode::List;
                    }
                }
            }
            LayoutPopupMode::List => {
                // Filter mode active
                if !self.layout_filter.is_empty() {
                    let filtered = self.layout_filtered_indices();
                    let cur_pos = filtered.iter().position(|&i| i + 3 == self.layout_popup_selected);
                    match key.code {
                        KeyCode::Esc => {
                            self.layout_filter.clear();
                            self.layout_popup_selected = 0;
                        }
                        KeyCode::Backspace => {
                            self.layout_filter.pop();
                            if self.layout_filter.is_empty() || self.layout_filter == "\0" {
                                self.layout_filter.clear();
                                self.layout_popup_selected = 0;
                            } else {
                                let filtered = self.layout_filtered_indices();
                                if let Some(&first) = filtered.first() {
                                    self.layout_popup_selected = first + 3;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(&idx) = filtered.iter().find(|&&i| i + 3 == self.layout_popup_selected) {
                                if let Some(layout) = self.saved_layouts.get(idx).cloned() {
                                    self.apply_layout(&layout);
                                    self.show_layout_popup = false;
                                    self.layout_filter.clear();
                                    self.page_start = 0;
                                    self.fetch_sessions().await;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let Some(pos) = cur_pos {
                                if pos + 1 < filtered.len() {
                                    self.layout_popup_selected = filtered[pos + 1] + 3;
                                }
                            } else if let Some(&first) = filtered.first() {
                                self.layout_popup_selected = first + 3;
                            }
                        }
                        KeyCode::Up => {
                            if let Some(pos) = cur_pos {
                                if pos > 0 {
                                    self.layout_popup_selected = filtered[pos - 1] + 3;
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            self.layout_filter.push(c);
                            let filtered = self.layout_filtered_indices();
                            if let Some(&first) = filtered.first() {
                                self.layout_popup_selected = first + 3;
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // Normal list mode
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.show_layout_popup = false;
                    }
                    KeyCode::Char('h') | KeyCode::Char('?') => {
                        self.show_help = !self.show_help;
                    }
                    KeyCode::Char('/') => {
                        self.layout_filter = "\0".to_string();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max = self.saved_layouts.len() + 3;
                        if self.layout_popup_selected + 1 < max {
                            self.layout_popup_selected += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.layout_popup_selected = self.layout_popup_selected.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if self.layout_popup_selected == 0 {
                            // Edit Columns
                            self.build_column_editor();
                            self.show_layout_popup = false;
                            self.show_column_editor = true;
                        } else if self.layout_popup_selected == 1 {
                            self.layout_popup_mode = LayoutPopupMode::SaveInput;
                            self.layout_save_name.clear();
                            self.layout_save_cursor = 0;
                        } else if self.layout_popup_selected == 2 {
                            self.columns = default_columns();
                            self.sync_session_fields();
                            self.show_layout_popup = false;
                            self.page_start = 0;
                            self.fetch_sessions().await;
                        } else {
                            let idx = self.layout_popup_selected - 3;
                            if let Some(layout) = self.saved_layouts.get(idx).cloned() {
                                self.apply_layout(&layout);
                                self.show_layout_popup = false;
                                self.page_start = 0;
                                self.fetch_sessions().await;
                            }
                        }
                    }
                    KeyCode::Char('x') | KeyCode::Delete => {
                        if let Some(idx) = self.layout_popup_selected.checked_sub(3) {
                            if let Some(layout) = self.saved_layouts.get(idx) {
                                self.layout_delete_name = layout.name.clone();
                                self.layout_popup_mode = LayoutPopupMode::ConfirmDelete;
                            }
                        }
                    }
                    _ => {}
                }
            }
            LayoutPopupMode::SaveInput => {
                match key.code {
                    KeyCode::Esc => {
                        self.layout_popup_mode = LayoutPopupMode::List;
                    }
                    KeyCode::Enter => {
                        if !self.layout_save_name.is_empty() {
                            let name = self.layout_save_name.clone();
                            let columns: Vec<String> = self.columns.iter()
                                .enumerate()
                                .filter(|(i, c)| !(*i == 0 && c.field == "ipProtocol"))
                                .map(|(_, c)| c.field.clone()).collect();
                            let sort_field = self.columns.get(self.sort_column)
                                .map(|c| c.field.clone())
                                .unwrap_or_default();
                            let sort_dir = if self.sort_desc { "desc" } else { "asc" };
                            self.status_msg = format!("Saving layout '{name}' with {} columns...", columns.len());
                            // Check if layout name already exists → update, else create
                            let exists = self.saved_layouts.iter().any(|l| l.name == name);
                            let result = if exists {
                                self.client.update_layout(&name, &columns, &sort_field, sort_dir).await
                            } else {
                                self.client.create_layout(&name, &columns, &sort_field, sort_dir).await
                            };
                            match result {
                                Ok(_) => {
                                    self.status_msg = format!("Saved layout '{name}'");
                                    self.fetch_layouts().await;
                                }
                                Err(e) => self.status_msg = format!("Error saving layout: {e}"),
                            }
                            self.show_layout_popup = false;
                        }
                    }
                    KeyCode::Backspace => {
                        if self.layout_save_cursor > 0 {
                            self.layout_save_cursor -= 1;
                            self.layout_save_name.remove(self.layout_save_cursor);
                        }
                    }
                    KeyCode::Left => {
                        self.layout_save_cursor = self.layout_save_cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        self.layout_save_cursor = (self.layout_save_cursor + 1).min(self.layout_save_name.len());
                    }
                    KeyCode::Char(c) => {
                        self.layout_save_name.insert(self.layout_save_cursor, c);
                        self.layout_save_cursor += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn open_view_popup(&mut self) {
        self.status_msg = "Fetching views...".into();
        match self.client.get_views().await {
            Ok(views) => {
                self.saved_views = views;
                self.status_msg = String::new();
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
        self.view_popup_mode = ViewPopupMode::List;
        self.view_popup_selected = 0;
        self.view_filter.clear();
        self.view_filter_active = false;
        self.show_view_popup = true;
    }

    pub fn view_filtered_indices(&self) -> Vec<usize> {
        let filter_text = self.view_filter.to_lowercase();
        if filter_text.is_empty() {
            return (0..self.saved_views.len()).collect();
        }
        self.saved_views.iter().enumerate()
            .filter(|(_, v)| v.name.to_lowercase().contains(&filter_text) || v.expression.to_lowercase().contains(&filter_text))
            .map(|(i, _)| i)
            .collect()
    }

    async fn handle_view_popup_key(&mut self, key: KeyEvent) {
        match self.view_popup_mode {
            ViewPopupMode::SaveInput => {
                match key.code {
                    KeyCode::Enter => {
                        let name = self.view_save_name.trim().to_string();
                        if !name.is_empty() && !self.expression.is_empty() {
                            let col_config = if self.view_save_columns {
                                let cols: Vec<String> = self.columns.iter().map(|c| c.exp.clone()).collect();
                                let sort_field = self.session_fields.get(self.sort_column)
                                    .cloned().unwrap_or_else(|| "firstPacket".into());
                                let sort_dir = if self.sort_desc { "desc" } else { "asc" };
                                Some((cols, sort_field, sort_dir.to_string()))
                            } else {
                                None
                            };
                            let config_ref = col_config.as_ref().map(|(c, sf, sd)| (c.as_slice(), sf.as_str(), sd.as_str()));
                            match self.client.create_view(&name, &self.expression, config_ref).await {
                                Ok(resp) => {
                                    self.status_msg = format!("View '{}' created", name);
                                    let view_id = resp.get("view")
                                        .and_then(|v| v.get("id"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&name)
                                        .to_string();
                                    self.active_view = Some(view_id);
                                    self.active_view_name = Some(name);
                                    self.page_start = 0;
                                    self.show_view_popup = false;
                                    self.refresh_for_active_tab().await;
                                }
                                Err(e) => self.status_msg = format!("Error creating view: {e}"),
                            }
                        } else if self.expression.is_empty() {
                            self.status_msg = "Cannot save view: expression is empty".into();
                        }
                        self.view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Esc => {
                        self.view_popup_mode = ViewPopupMode::List;
                    }
                    KeyCode::Tab => {
                        self.view_save_columns = !self.view_save_columns;
                    }
                    KeyCode::Left => {
                        if self.view_save_cursor > 0 {
                            self.view_save_cursor -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.view_save_cursor < self.view_save_name.len() {
                            self.view_save_cursor += 1;
                        }
                    }
                    KeyCode::Char(c) => {
                        self.view_save_name.insert(self.view_save_cursor, c);
                        self.view_save_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if self.view_save_cursor > 0 {
                            self.view_save_cursor -= 1;
                            self.view_save_name.remove(self.view_save_cursor);
                        }
                    }
                    _ => {}
                }
            }
            ViewPopupMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let id = self.view_delete_id.clone();
                        let name = self.view_delete_name.clone();
                        match self.client.delete_view(&id).await {
                            Ok(_) => {
                                self.status_msg = format!("View '{}' deleted", name);
                                if self.active_view.as_deref() == Some(&id) {
                                    self.active_view = None;
                                    self.active_view_name = None;
                                }
                                self.saved_views.retain(|v| v.id != id);
                                self.view_popup_selected = 0;
                            }
                            Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                        }
                        self.view_popup_mode = ViewPopupMode::List;
                    }
                    _ => {
                        self.view_popup_mode = ViewPopupMode::List;
                    }
                }
            }
            ViewPopupMode::List => {
                let filtered = self.view_filtered_indices();
                let total_items = 2 + filtered.len(); // 0=Save, 1=Clear, 2+=views
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        if total_items > 0 {
                            self.view_popup_selected = (self.view_popup_selected + 1).min(total_items - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.view_popup_selected = self.view_popup_selected.saturating_sub(1);
                    }
                    KeyCode::Char('x') => {
                        if self.view_popup_selected >= 2 {
                            let fi = self.view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view = &self.saved_views[idx];
                                if !view.shared {
                                    self.view_delete_id = view.id.clone();
                                    self.view_delete_name = view.name.clone();
                                    self.view_popup_mode = ViewPopupMode::ConfirmDelete;
                                } else {
                                    self.status_msg = "Cannot delete shared views".into();
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if self.view_popup_selected == 0 {
                            // Save current expression as view
                            if self.expression.is_empty() {
                                self.status_msg = "Cannot save view: expression is empty".into();
                            } else {
                                self.view_save_name.clear();
                                self.view_save_cursor = 0;
                                self.view_save_columns = false;
                                self.view_popup_mode = ViewPopupMode::SaveInput;
                            }
                        } else if self.view_popup_selected == 1 {
                            // Clear view
                            if self.active_view.is_some() {
                                self.active_view = None;
                                self.active_view_name = None;
                                self.page_start = 0;
                                self.show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            } else {
                                self.show_view_popup = false;
                            }
                        } else {
                            // Select a view
                            let fi = self.view_popup_selected - 2;
                            if let Some(&idx) = filtered.get(fi) {
                                let view_id = self.saved_views[idx].id.clone();
                                let view_name = self.saved_views[idx].name.clone();
                                self.active_view = Some(view_id);
                                self.active_view_name = Some(view_name);
                                self.page_start = 0;
                                self.show_view_popup = false;
                                self.refresh_for_active_tab().await;
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if self.view_filter_active {
                            self.view_filter.clear();
                            self.view_filter_active = false;
                        } else {
                            self.show_view_popup = false;
                        }
                    }
                    KeyCode::Char('/') => {
                        if !self.view_filter_active {
                            self.view_filter_active = true;
                            self.view_filter.clear();
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.view_filter_active {
                            self.view_filter.push(c);
                            self.view_popup_selected = 2; // reset to first view
                        }
                    }
                    KeyCode::Backspace => {
                        if self.view_filter_active {
                            self.view_filter.pop();
                            if self.view_filter.is_empty() {
                                self.view_filter_active = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    async fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.session_view = SessionView::List;
                self.session_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.session_detail
                    && detail.total_rows > 0 && detail.selected < detail.total_rows - 1 {
                        detail.selected += 1;
                    }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.session_detail
                    && detail.selected > 0 {
                        detail.selected -= 1;
                    }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = detail.total_rows.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.session_detail
                    && let Some(obj) = detail.data.as_object() {
                        let filter_lower = detail.filter.to_lowercase();
                        let mut keys: Vec<&String> = obj.keys()
                            .filter(|k| !is_hidden_detail_field(k))
                            .filter(|k| {
                                if filter_lower.is_empty() {
                                    return true;
                                }
                                let friendly = self.field_friendly_map.get(k.as_str())
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
                            let exp_name = self.field_exp_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            let friendly = self.field_friendly_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            self.detail_action_menu = Some(DetailActionMenu {
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
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
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

    fn handle_detail_filter_key(&mut self, key: KeyEvent) {
        let is_stats = self.active_tab == Tab::Stats;
        match key.code {
            KeyCode::Esc => {
                if is_stats {
                    if let Some(ref mut detail) = self.stats_detail {
                        detail.filter.clear();
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.session_detail {
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
                if is_stats {
                    if let Some(ref mut detail) = self.stats_detail {
                        detail.filter.push(c);
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.session_detail {
                    detail.filter.push(c);
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
            }
            KeyCode::Backspace => {
                if is_stats {
                    if let Some(ref mut detail) = self.stats_detail {
                        detail.filter.pop();
                        detail.scroll = 0;
                    }
                } else if let Some(ref mut detail) = self.session_detail {
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
        if let Some(ref mut detail) = self.session_detail
            && let Some(obj) = detail.data.as_object() {
                let filter_lower = detail.filter.to_lowercase();
                detail.total_rows = obj.keys()
                    .filter(|k| !is_hidden_detail_field(k))
                    .filter(|k| {
                        if filter_lower.is_empty() {
                            return true;
                        }
                        let friendly = self.field_friendly_map.get(k.as_str())
                            .map(|s| s.as_str())
                            .unwrap_or(k.as_str());
                        k.to_lowercase().contains(&filter_lower)
                            || friendly.to_lowercase().contains(&filter_lower)
                    })
                    .count();
            }
    }

    fn handle_action_menu_key(&mut self, key: KeyEvent) {
        let remove_enabled = self.remove_enabled();
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

    async fn handle_action_prompt_key(&mut self, key: KeyEvent) {
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
        self.sessions.iter()
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
                match self.client.download_session_pcap(node, id).await {
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
                    self.client.download_sessions_pcap_ids(&ids).await
                } else {
                    self.client.download_sessions_pcap(&self.expression, date, &self.active_view).await
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
                    self.client.export_sessions_csv_ids(&ids, &self.session_fields).await
                } else {
                    self.client.export_sessions_csv(&self.expression, date, &self.session_fields, &self.active_view).await
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
                match self.client.add_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::AddTags, ActionTarget::All) => {
                self.status_msg = "Adding tags...".into();
                match self.client.add_sessions_tags(&self.expression, date, &prompt.input, &self.active_view).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                self.status_msg = "Removing tags...".into();
                match self.client.remove_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::All) => {
                self.status_msg = "Removing tags...".into();
                match self.client.remove_sessions_tags(&self.expression, date, &prompt.input, &self.active_view).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            _ => {}
        }
    }

    async fn handle_detail_action_key(&mut self, key: KeyEvent) {
        let in_values = self.detail_action_menu.as_ref()
            .map(|m| m.values.is_some()).unwrap_or(false);

        match key.code {
            KeyCode::Esc => {
                self.detail_action_menu = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut menu) = self.detail_action_menu {
                    if in_values {
                        let len = menu.values.as_ref().unwrap().len();
                        menu.value_selected = (menu.value_selected + 1).min(len - 1);
                    } else {
                        menu.selected = (menu.selected + 1).min(DetailActionMenu::OPTIONS.len() - 1);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut menu) = self.detail_action_menu {
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
                    if let Some(ref mut menu) = self.detail_action_menu {
                        let chosen = menu.values.as_ref().unwrap()[menu.value_selected].clone();
                        menu.value = chosen;
                        menu.values = None;
                        menu.selected = 0;
                    }
                } else if let Some(menu) = self.detail_action_menu.take() {
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
                        self.request_summary_fetch();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_packets_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('q') => {
                self.packets_view = None;
                self.packets_scroll = 0;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.packets_scroll = self.packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.packets_scroll = self.packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.packets_scroll = self.packets_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.packets_scroll = self.packets_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.packets_scroll = self.packets_scroll.saturating_add(self.visible_rows as u16);
            }
            KeyCode::PageUp => {
                self.packets_scroll = self.packets_scroll.saturating_sub(self.visible_rows as u16);
            }
            KeyCode::Home | KeyCode::Left => {
                self.packets_scroll = 0;
            }
            KeyCode::Right => {
                self.packets_scroll = u16::MAX;
            }
            KeyCode::Char('r') => {
                self.packets_raw = !self.packets_raw;
                self.request_packets();
            }
            KeyCode::Char('l') => {
                self.packets_line = self.packets_line.next();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    pub fn request_packets(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_session) {
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
            self.packets_total_pending = total;
            self.packets_node_pending = node.to_string();
            self.packets_id_pending = id.to_string();
            self.status_msg = "Fetching packets...".into();
            if total > 500 {
                self.show_loading = true;
            }
            self.pending_packets_fetch = true;
        }
    }

    async fn handle_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
            }
            KeyCode::BackTab => {
                self.prev_tab();
            }
            KeyCode::Char('1') => {
                if self.stats_tab != StatsTab::Capture {
                    self.stats_tab = StatsTab::Capture;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Char('2') => {
                if self.stats_tab != StatsTab::DBStats {
                    self.stats_tab = StatsTab::DBStats;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Char('3') => {
                if self.stats_tab != StatsTab::DBIndices {
                    self.stats_tab = StatsTab::DBIndices;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.stats_data.is_empty() {
                    self.stats_selected = (self.stats_selected + 1).min(self.stats_data.len() - 1);
                    self.stats_table_state.select(Some(self.stats_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.stats_selected > 0 {
                    self.stats_selected -= 1;
                    self.stats_table_state.select(Some(self.stats_selected));
                }
            }
            KeyCode::Enter => {
                self.open_stats_detail();
            }
            KeyCode::Char('r') => {
                self.fetch_stats().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.stats_filter_edit = self.stats_filter.clone();
                self.expression_cursor = self.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let num_cols = self.stats_tab.columns().len();
                self.stats_sort_column = (self.stats_sort_column + 1) % num_cols;
                self.fetch_stats().await;
            }
            KeyCode::Char('S') => {
                self.stats_sort_desc = !self.stats_sort_desc;
                self.fetch_stats().await;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn handle_stats_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.stats_view = StatsView::List;
                self.stats_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Left | KeyCode::Home => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = 0;
                }
            }
            KeyCode::Right | KeyCode::End => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = u16::MAX;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('E') => {
                self.stats_filter_edit = self.stats_filter.clone();
                self.expression_cursor = self.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    async fn handle_arkime_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('f') => {
                self.field_filter.clear();
                self.field_filter_selected = 0;
                self.input_mode = InputMode::FieldSelector;
            }
            KeyCode::Char('G') => {
                self.summary_metric = self.summary_metric.next();
            }
            KeyCode::Char('s') => {
                self.summary_sort = self.summary_sort.next();
                self.sort_summary_data();
            }
            KeyCode::Char('S') => {
                self.summary_sort_desc = !self.summary_sort_desc;
                self.sort_summary_data();
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.summary_data.is_empty() {
                    self.summary_selected = (self.summary_selected + self.visible_rows).min(self.summary_data.len() - 1);
                    self.summary_table_state.select(Some(self.summary_selected));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.summary_selected = self.summary_selected.saturating_sub(self.visible_rows);
                self.summary_table_state.select(Some(self.summary_selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.summary_data.is_empty() {
                    self.summary_selected = (self.summary_selected + 1).min(self.summary_data.len() - 1);
                    self.summary_table_state.select(Some(self.summary_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.summary_selected > 0 {
                    self.summary_selected -= 1;
                    self.summary_table_state.select(Some(self.summary_selected));
                }
            }
            KeyCode::PageDown => {
                if !self.summary_data.is_empty() {
                    self.summary_selected = (self.summary_selected + self.visible_rows).min(self.summary_data.len() - 1);
                    self.summary_table_state.select(Some(self.summary_selected));
                }
            }
            KeyCode::PageUp => {
                self.summary_selected = self.summary_selected.saturating_sub(self.visible_rows);
                self.summary_table_state.select(Some(self.summary_selected));
            }
            KeyCode::Left | KeyCode::Home => {
                self.summary_selected = 0;
                self.summary_table_state.select(Some(self.summary_selected));
            }
            KeyCode::Right | KeyCode::End => {
                if !self.summary_data.is_empty() {
                    self.summary_selected = self.summary_data.len() - 1;
                    self.summary_table_state.select(Some(self.summary_selected));
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.summary_data.get(self.summary_selected) {
                    let value = match &item.item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    self.detail_action_menu = Some(DetailActionMenu {
                        field: self.summary_field.clone(),
                        display: self.summary_field.clone(),
                        value,
                        selected: 0,
                        values: None,
                        value_selected: 0,
                    });
                }
            }
            KeyCode::Char('r') => {
                self.request_summary_fetch();
            }
            KeyCode::Char('t') => {
                self.time_range = self.time_range.next();
                self.request_summary_fetch();
            }
            KeyCode::Char('T') => {
                self.time_range = self.time_range.prev();
                self.request_summary_fetch();
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

    async fn handle_field_selector_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.field_filter.clear();
            }
            KeyCode::Enter => {
                let filtered = self.filtered_fields();
                if let Some(field) = filtered.get(self.field_filter_selected) {
                    self.summary_field = field.exp.clone();
                    self.input_mode = InputMode::Normal;
                    self.field_filter.clear();
                    self.request_summary_fetch();
                }
            }
            KeyCode::Down => {
                let count = self.filtered_fields().len();
                if count > 0 {
                    self.field_filter_selected = (self.field_filter_selected + 1).min(count - 1);
                }
            }
            KeyCode::Up => {
                if self.field_filter_selected > 0 {
                    self.field_filter_selected -= 1;
                }
            }
            KeyCode::Char(c) => {
                self.field_filter.push(c);
                self.field_filter_selected = 0;
            }
            KeyCode::Backspace => {
                self.field_filter.pop();
                self.field_filter_selected = 0;
            }
            _ => {}
        }
    }

    async fn handle_cont3xt_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Expression {
            match key.code {
                KeyCode::Enter => {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.request_c3_search();
                }
                KeyCode::Esc => {
                    self.expression_edit = self.expression.clone();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Left => {
                    if self.expression_cursor > 0 {
                        self.expression_cursor -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.expression_cursor < self.expression_edit.len() {
                        self.expression_cursor += 1;
                    }
                }
                KeyCode::Home => self.expression_cursor = 0,
                KeyCode::End => self.expression_cursor = self.expression_edit.len(),
                KeyCode::Backspace => {
                    if self.expression_cursor > 0 {
                        self.expression_cursor -= 1;
                        self.expression_edit.remove(self.expression_cursor);
                    }
                }
                KeyCode::Delete => {
                    if self.expression_cursor < self.expression_edit.len() {
                        self.expression_edit.remove(self.expression_cursor);
                    }
                }
                KeyCode::Char(c) => {
                    self.expression_edit.insert(self.expression_cursor, c);
                    self.expression_cursor += 1;
                }
                _ => {}
            }
            return;
        }

        // Integration popup handler
        if self.show_integration_popup {
            let filtered: Vec<usize> = self.c3_integrations.iter().enumerate()
                .filter(|(_, int)| {
                    self.integration_popup_filter.is_empty()
                    || int.name.to_lowercase().contains(&self.integration_popup_filter.to_lowercase())
                })
                .map(|(i, _)| i)
                .collect();

            // When filtering mode is active, capture text input
            if self.integration_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.integration_popup_filtering = false;
                        if self.integration_popup_filter.is_empty() {
                            // nothing to clear, close popup
                        }
                    }
                    KeyCode::Enter => {
                        self.integration_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.integration_popup_filter.pop();
                        self.integration_popup_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.integration_popup_filter.push(c);
                        self.integration_popup_selected = 0;
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Esc => {
                    if !self.integration_popup_filter.is_empty() {
                        self.integration_popup_filter.clear();
                        self.integration_popup_selected = 0;
                    } else {
                        self.show_integration_popup = false;
                    }
                }
                KeyCode::Char('q') => self.show_integration_popup = false,
                KeyCode::Down | KeyCode::Char('j') => {
                    if !filtered.is_empty() {
                        self.integration_popup_selected = (self.integration_popup_selected + 1).min(filtered.len() - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.integration_popup_selected = self.integration_popup_selected.saturating_sub(1);
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(&idx) = filtered.get(self.integration_popup_selected) {
                        let name = self.c3_integrations[idx].name.clone();
                        if self.c3_disabled_integrations.contains(&name) {
                            self.c3_disabled_integrations.remove(&name);
                        } else {
                            self.c3_disabled_integrations.insert(name);
                        }
                    }
                }
                KeyCode::Char('/') => {
                    self.integration_popup_filtering = true;
                }
                KeyCode::Char('a') => {
                    // All on — enable all (clear disabled set)
                    self.c3_disabled_integrations.clear();
                }
                KeyCode::Char('n') => {
                    // None — disable all
                    for int in &self.c3_integrations {
                        self.c3_disabled_integrations.insert(int.name.clone());
                    }
                }
                KeyCode::Char('!') => {
                    // Invert selection
                    let all_names: Vec<String> = self.c3_integrations.iter().map(|i| i.name.clone()).collect();
                    for name in all_names {
                        if self.c3_disabled_integrations.contains(&name) {
                            self.c3_disabled_integrations.remove(&name);
                        } else {
                            self.c3_disabled_integrations.insert(name);
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab if self.active_tab == Tab::Search => {
                // Tab toggles focus between results list and detail pane
                self.c3_focus = match self.c3_focus {
                    Cont3xtFocus::Results => Cont3xtFocus::Detail,
                    Cont3xtFocus::Detail => Cont3xtFocus::Results,
                };
            }
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('R') if self.active_tab == Tab::Search => {
                self.c3_raw_view = !self.c3_raw_view;
                self.c3_detail_scroll = 0;
                self.c3_detail_hscroll = 0;
            }
            KeyCode::Char('r') => {
                self.request_c3_search();
            }
            KeyCode::Char('i') if self.active_tab == Tab::Search => {
                self.show_integration_popup = true;
                self.integration_popup_selected = 0;
                self.integration_popup_filter.clear();
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            if !self.c3_results.is_empty() {
                                self.c3_selected = (self.c3_selected + self.visible_rows).min(self.c3_results.len() - 1);
                                self.c3_detail_scroll = 0;
                                self.c3_detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_add(self.visible_rows as u16);
                        }
                    }
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            self.c3_selected = self.c3_selected.saturating_sub(self.visible_rows);
                            self.c3_detail_scroll = 0;
                            self.c3_detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_sub(self.visible_rows as u16);
                        }
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            if !self.c3_results.is_empty() {
                                self.c3_selected = (self.c3_selected + 1).min(self.c3_results.len() - 1);
                                self.c3_detail_scroll = 0;
                                self.c3_detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_add(1);
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            self.c3_selected = self.c3_selected.saturating_sub(1);
                            self.c3_detail_scroll = 0;
                            self.c3_detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_sub(1);
                        }
                    }
                }
            }
            KeyCode::PageDown => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_scroll = self.c3_detail_scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_scroll = self.c3_detail_scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Home => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_scroll = 0;
                    self.c3_detail_hscroll = 0;
                }
            }
            KeyCode::End => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_scroll = u16::MAX;
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_sub(20);
                }
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_add(20);
                }
            }
            KeyCode::Left => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_sub(4);
                }
            }
            KeyCode::Right => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_add(4);
                }
            }
            _ => {}
        }
    }
}
