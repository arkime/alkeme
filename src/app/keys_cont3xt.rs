use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    pub(crate) async fn handle_cont3xt_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Expression {
            match key.code {
                KeyCode::Enter => {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.c3_request_search();
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

        // Link groups popup handler
        if self.c3_show_link_popup {
            if self.c3_link_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.c3_link_popup_filtering = false;
                        self.c3_link_popup_filter.clear();
                        self.c3_build_link_flat();
                    }
                    KeyCode::Enter => {
                        self.c3_link_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.c3_link_popup_filter.pop();
                        self.c3_build_link_flat();
                    }
                    KeyCode::Char(c) => {
                        self.c3_link_popup_filter.push(c);
                        self.c3_build_link_flat();
                    }
                    _ => {}
                }
                return;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => {
                    self.c3_show_link_popup = false;
                }
                KeyCode::Char('/') => {
                    self.c3_link_popup_filtering = true;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.c3_link_popup_selected = self.c3_link_popup_selected.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if !self.c3_link_flat.is_empty() {
                        self.c3_link_popup_selected = (self.c3_link_popup_selected + 10).min(self.c3_link_flat.len() - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.c3_link_popup_selected = self.c3_link_popup_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.c3_link_flat.is_empty() {
                        self.c3_link_popup_selected = (self.c3_link_popup_selected + 1).min(self.c3_link_flat.len() - 1);
                    }
                }
                KeyCode::Enter => {
                    if let Some((_, _, url, _)) = self.c3_link_flat.get(self.c3_link_popup_selected) {
                        let url = url.clone();
                        #[cfg(target_os = "macos")]
                        { let _ = std::process::Command::new("open").arg(&url).spawn(); }
                        #[cfg(target_os = "linux")]
                        { let _ = std::process::Command::new("xdg-open").arg(&url).spawn(); }
                        self.status_msg = format!("Opening: {url}");
                    }
                }
                KeyCode::Char('h') | KeyCode::Char('?') => {
                    self.show_help = true;
                }
                _ => {}
            }
            return;
        }

        // Integration popup handler
        if self.c3_show_integration_popup {
            match self.c3_integration_popup_mode {
                IntegrationPopupMode::SaveInput => {
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                        }
                        KeyCode::Enter => {
                            if !self.c3_view_save_name.is_empty() {
                                let name = self.c3_view_save_name.clone();
                                let integrations = self.c3_enabled_integration_names();
                                let status = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(
                                        self.client.c3_create_view(&name, &integrations)
                                    )
                                });
                                match status {
                                    Ok(_) => {
                                        self.status_msg = format!("Saved view: {name}");
                                        tokio::task::block_in_place(|| {
                                            tokio::runtime::Handle::current().block_on(self.c3_fetch_views())
                                        });
                                    }
                                    Err(e) => self.status_msg = format!("Error saving view: {e}"),
                                }
                                self.c3_view_save_name.clear();
                                self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                            }
                        }
                        KeyCode::Backspace => { self.c3_view_save_name.pop(); }
                        KeyCode::Char(c) => { self.c3_view_save_name.push(c); }
                        _ => {}
                    }
                }
                IntegrationPopupMode::ConfirmDelete => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(view) = self.c3_views.get(self.c3_view_selected.saturating_sub(1)) {
                                let id = view.id.clone();
                                let name = view.name.clone();
                                let status = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(
                                        self.client.c3_delete_view(&id)
                                    )
                                });
                                match status {
                                    Ok(_) => {
                                        self.status_msg = format!("Deleted view: {name}");
                                        tokio::task::block_in_place(|| {
                                            tokio::runtime::Handle::current().block_on(self.c3_fetch_views())
                                        });
                                        if self.c3_view_selected > self.c3_views.len() {
                                            self.c3_view_selected = self.c3_views.len();
                                        }
                                    }
                                    Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                                }
                            }
                            self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                        }
                        _ => {
                            self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                        }
                    }
                }
                IntegrationPopupMode::Views => {
                    // +1 for "Save Current" option at top
                    let list_len = self.c3_views.len() + 1;
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_integration_popup_mode = IntegrationPopupMode::Integrations;
                        }
                        KeyCode::Char('q') => self.c3_show_integration_popup = false,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if list_len > 0 {
                                self.c3_view_selected = (self.c3_view_selected + 1).min(list_len - 1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.c3_view_selected = self.c3_view_selected.saturating_sub(1);
                        }
                        KeyCode::Enter => {
                            if self.c3_view_selected == 0 {
                                // "Save Current" option
                                self.c3_view_save_name.clear();
                                self.c3_integration_popup_mode = IntegrationPopupMode::SaveInput;
                            } else {
                                // Load a view
                                let view_idx = self.c3_view_selected - 1;
                                if let Some(view) = self.c3_views.get(view_idx) {
                                    let integrations = view.integrations.clone();
                                    let name = view.name.clone();
                                    self.c3_active_view_id = Some(view.id.clone());
                                    self.c3_active_view_name = Some(name.clone());
                                    self.c3_apply_view(&integrations);
                                    self.status_msg = format!("Loaded view: {name}");
                                    self.c3_show_integration_popup = false;
                                }
                            }
                        }
                        KeyCode::Char('x') => {
                            // Delete selected view
                            if self.c3_view_selected > 0 {
                                let view_idx = self.c3_view_selected - 1;
                                if let Some(view) = self.c3_views.get(view_idx) {
                                    if view.editable {
                                        self.c3_integration_popup_mode = IntegrationPopupMode::ConfirmDelete;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                IntegrationPopupMode::Integrations => {
                    let filtered: Vec<usize> = self.c3_integrations.iter().enumerate()
                        .filter(|(_, int)| {
                            self.c3_integration_popup_filter.is_empty()
                            || int.name.to_lowercase().contains(&self.c3_integration_popup_filter.to_lowercase())
                        })
                        .map(|(i, _)| i)
                        .collect();

                    // When filtering mode is active, capture text input
                    if self.c3_integration_popup_filtering {
                        match key.code {
                            KeyCode::Esc => {
                                self.c3_integration_popup_filtering = false;
                                if self.c3_integration_popup_filter.is_empty() {
                                    // nothing to clear, close popup
                                }
                            }
                            KeyCode::Enter => {
                                self.c3_integration_popup_filtering = false;
                            }
                            KeyCode::Backspace => {
                                self.c3_integration_popup_filter.pop();
                                self.c3_integration_popup_selected = 0;
                            }
                            KeyCode::Char(c) => {
                                self.c3_integration_popup_filter.push(c);
                                self.c3_integration_popup_selected = 0;
                            }
                            _ => {}
                        }
                        return;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            if !self.c3_integration_popup_filter.is_empty() {
                                self.c3_integration_popup_filter.clear();
                                self.c3_integration_popup_selected = 0;
                            } else {
                                self.c3_show_integration_popup = false;
                            }
                        }
                        KeyCode::Char('q') => self.c3_show_integration_popup = false,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !filtered.is_empty() {
                                self.c3_integration_popup_selected = (self.c3_integration_popup_selected + 1).min(filtered.len() - 1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.c3_integration_popup_selected = self.c3_integration_popup_selected.saturating_sub(1);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if let Some(&idx) = filtered.get(self.c3_integration_popup_selected) {
                                let name = self.c3_integrations[idx].name.clone();
                                if self.c3_disabled_integrations.contains(&name) {
                                    self.c3_disabled_integrations.remove(&name);
                                } else {
                                    self.c3_disabled_integrations.insert(name);
                                }
                                self.c3_active_view_id = None;
                                self.c3_active_view_name = None;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.c3_integration_popup_filtering = true;
                        }
                        KeyCode::Char('a') => {
                            self.c3_disabled_integrations.clear();
                            self.c3_active_view_id = None;
                            self.c3_active_view_name = None;
                        }
                        KeyCode::Char('n') => {
                            for int in &self.c3_integrations {
                                self.c3_disabled_integrations.insert(int.name.clone());
                            }
                            self.c3_active_view_id = None;
                            self.c3_active_view_name = None;
                        }
                        KeyCode::Char('!') => {
                            let all_names: Vec<String> = self.c3_integrations.iter().map(|i| i.name.clone()).collect();
                            for name in all_names {
                                if self.c3_disabled_integrations.contains(&name) {
                                    self.c3_disabled_integrations.remove(&name);
                                } else {
                                    self.c3_disabled_integrations.insert(name);
                                }
                            }
                            self.c3_active_view_id = None;
                            self.c3_active_view_name = None;
                        }
                        KeyCode::Char('v') => {
                            // Switch to views mode, re-fetch views
                            self.c3_view_selected = 0;
                            self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(self.c3_fetch_views())
                            });
                        }
                        _ => {}
                    }
                }
            }
            return;
        }

        // C3 stats filter mode
        if self.c3_stats_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.c3_stats_filtering = false;
                    if self.c3_stats_filter.is_empty() {
                        // nothing to clear
                    }
                }
                KeyCode::Enter => {
                    self.c3_stats_filtering = false;
                }
                KeyCode::Backspace => {
                    self.c3_stats_filter.pop();
                }
                KeyCode::Char(c) => {
                    self.c3_stats_filter.push(c);
                    self.c3_stats_selected = 0;
                    self.c3_stats_table_state.select(Some(self.c3_stats_selected));
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::C3Stats && self.c3_stats_data.is_empty() {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_stats())
                    });
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::C3Stats && self.c3_stats_data.is_empty() {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_stats())
                    });
                }
            }
            KeyCode::Enter if self.active_tab == Tab::Search => {
                match self.c3_focus {
                    Cont3xtFocus::Results => {
                        self.c3_focus = Cont3xtFocus::Detail;
                        self.c3_detail_scroll = 0;
                        self.c3_detail_hscroll = 0;
                    }
                    Cont3xtFocus::Detail => {
                        self.c3_focus = Cont3xtFocus::Results;
                    }
                }
            }
            KeyCode::Esc if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail => {
                self.c3_focus = Cont3xtFocus::Results;
            }
            KeyCode::Char('/') | KeyCode::Char('E') if self.active_tab == Tab::Search => {
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('/') if self.active_tab == Tab::C3Stats => {
                self.c3_stats_filtering = true;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('R') if self.active_tab == Tab::Search => {
                self.c3_raw_view = !self.c3_raw_view;
                self.c3_detail_scroll = 0;
                self.c3_detail_hscroll = 0;
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) && self.active_tab == Tab::Search => {
                self.c3_no_cache = true;
                self.c3_request_search();
            }
            KeyCode::Char('r') if self.active_tab == Tab::Search => {
                self.c3_request_search();
            }
            KeyCode::Char('r') if self.active_tab == Tab::C3Stats => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_stats())
                });
            }
            KeyCode::Char('i') if self.active_tab == Tab::Search => {
                self.c3_show_integration_popup = true;
                self.c3_integration_popup_selected = 0;
                self.c3_integration_popup_filter.clear();
                self.c3_integration_popup_mode = IntegrationPopupMode::Integrations;
            }
            KeyCode::Char('I') if self.active_tab == Tab::Search => {
                self.c3_show_integration_popup = true;
                self.c3_view_selected = 0;
                self.c3_integration_popup_mode = IntegrationPopupMode::Views;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_views())
                });
            }
            KeyCode::Char('l') if self.active_tab == Tab::Search => {
                if self.c3_results.is_empty() {
                    self.status_msg = "Search for an indicator first".to_string();
                } else {
                    self.c3_link_popup_selected = 0;
                    self.c3_link_popup_filter.clear();
                    self.c3_link_popup_filtering = false;
                    self.c3_build_link_flat();
                    self.c3_show_link_popup = true;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            // Jump to next top-level indicator
                            if let Some(next) = self.c3_tree_roots.iter().find(|&&r| r > self.c3_selected) {
                                self.c3_selected = *next;
                            } else if !self.c3_tree_order.is_empty() {
                                self.c3_selected = self.c3_tree_order.len() - 1;
                            }
                            self.c3_detail_scroll = 0;
                            self.c3_detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_add(self.visible_rows as u16);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    let data = self.c3_stats_current_data();
                    let filtered_len = data.iter()
                        .filter(|item| self.c3_stats_filter.is_empty()
                            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                .to_lowercase().contains(&self.c3_stats_filter.to_lowercase()))
                        .count();
                    if filtered_len > 0 {
                        self.c3_stats_selected = (self.c3_stats_selected + self.visible_rows).min(filtered_len - 1);
                        self.c3_stats_table_state.select(Some(self.c3_stats_selected));
                    }
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            // Jump to previous top-level indicator
                            if let Some(prev) = self.c3_tree_roots.iter().rev().find(|&&r| r < self.c3_selected) {
                                self.c3_selected = *prev;
                            } else {
                                self.c3_selected = 0;
                            }
                            self.c3_detail_scroll = 0;
                            self.c3_detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_sub(self.visible_rows as u16);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    self.c3_stats_selected = self.c3_stats_selected.saturating_sub(self.visible_rows);
                    self.c3_stats_table_state.select(Some(self.c3_stats_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            if !self.c3_tree_order.is_empty() {
                                self.c3_selected = (self.c3_selected + 1).min(self.c3_tree_order.len() - 1);
                                self.c3_detail_scroll = 0;
                                self.c3_detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_scroll = self.c3_detail_scroll.saturating_add(1);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    let data = self.c3_stats_current_data();
                    let filtered_len = data.iter()
                        .filter(|item| self.c3_stats_filter.is_empty()
                            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                .to_lowercase().contains(&self.c3_stats_filter.to_lowercase()))
                        .count();
                    if filtered_len > 0 {
                        self.c3_stats_selected = (self.c3_stats_selected + 1).min(filtered_len - 1);
                        self.c3_stats_table_state.select(Some(self.c3_stats_selected));
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
                } else if self.active_tab == Tab::C3Stats {
                    self.c3_stats_selected = self.c3_stats_selected.saturating_sub(1);
                    self.c3_stats_table_state.select(Some(self.c3_stats_selected));
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
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            self.c3_selected = 0;
                            self.c3_detail_scroll = 0;
                            self.c3_detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_sub(4);
                        }
                    }
                }
            }
            KeyCode::Right => {
                if self.active_tab == Tab::Search {
                    match self.c3_focus {
                        Cont3xtFocus::Results => {
                            if !self.c3_tree_order.is_empty() {
                                self.c3_selected = self.c3_tree_order.len() - 1;
                                self.c3_detail_scroll = 0;
                                self.c3_detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.c3_detail_hscroll = self.c3_detail_hscroll.saturating_add(4);
                        }
                    }
                }
            }
            // C3 Stats tab keys
            KeyCode::Char('1') if self.active_tab == Tab::C3Stats => {
                if self.c3_stats_tab != C3StatsTab::Integrations {
                    self.c3_stats_tab = C3StatsTab::Integrations;
                    self.c3_stats_selected = 0;
                    self.c3_stats_table_state.select(Some(self.c3_stats_selected));
                }
            }
            KeyCode::Char('2') if self.active_tab == Tab::C3Stats => {
                if self.c3_stats_tab != C3StatsTab::ITypes {
                    self.c3_stats_tab = C3StatsTab::ITypes;
                    self.c3_stats_selected = 0;
                    self.c3_stats_table_state.select(Some(self.c3_stats_selected));
                }
            }
            KeyCode::Char('s') if self.active_tab == Tab::C3Stats => {
                let ncols = self.c3_stats_tab.columns().len();
                self.c3_stats_sort_col = (self.c3_stats_sort_col + 1) % ncols;
            }
            KeyCode::Char('S') if self.active_tab == Tab::C3Stats => {
                self.c3_stats_sort_desc = !self.c3_stats_sort_desc;
            }
            _ => {}
        }
    }
}
