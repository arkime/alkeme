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

        // Overview selector popup handler
        if self.c3_show_overview_popup {
            if self.c3_overview_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.c3_overview_popup_filtering = false;
                        self.c3_overview_popup_filter.clear();
                        self.c3_overview_popup_selected = 0;
                    }
                    KeyCode::Enter => {
                        self.c3_overview_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.c3_overview_popup_filter.pop();
                        self.c3_overview_popup_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.c3_overview_popup_filter.push(c);
                        self.c3_overview_popup_selected = 0;
                    }
                    _ => {}
                }
                return;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('o') => {
                    self.c3_show_overview_popup = false;
                }
                KeyCode::Char('/') => {
                    self.c3_overview_popup_filtering = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.c3_overview_popup_selected = self.c3_overview_popup_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(C3TreeItem::Indicator(itype, _)) = self.c3_tree_order.get(self.c3_selected) {
                        let itype_lower = itype.to_lowercase();
                        let filter_lower = self.c3_overview_popup_filter.to_lowercase();
                        let count = self.c3_overviews.iter()
                            .filter(|o| o.itype.to_lowercase() == itype_lower)
                            .filter(|o| filter_lower.is_empty() || o.name.to_lowercase().contains(&filter_lower))
                            .count();
                        if count > 0 {
                            self.c3_overview_popup_selected = (self.c3_overview_popup_selected + 1).min(count - 1);
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(ov) = self.c3_overview_filtered_get() {
                        let itype_lower = ov.itype.to_lowercase();
                        self.c3_selected_overviews.insert(itype_lower, ov.id.clone());
                        self.c3_detail_scroll = 0;
                    }
                    self.c3_show_overview_popup = false;
                }
                KeyCode::Char('d') => {
                    if let Some(ov) = self.c3_overview_filtered_get() {
                        let itype_lower = ov.itype.to_lowercase();
                        let ov_id = ov.id.clone();
                        let ov_name = ov.name.clone();
                        self.c3_selected_overviews.insert(itype_lower, ov_id);
                        self.c3_detail_scroll = 0;
                        match self.client.c3_save_selected_overviews(&self.c3_selected_overviews).await {
                            Ok(_) => {
                                self.status_msg = format!("Default overview set: {ov_name}");
                                self.c3_fetch_overviews().await;
                            }
                            Err(e) => self.status_msg = format!("Error saving default: {e}"),
                        }
                        self.c3_show_overview_popup = false;
                    }
                }
                KeyCode::Char('r') => {
                    self.c3_fetch_overviews().await;
                    self.status_msg = "Overviews refreshed".into();
                }
                KeyCode::Char('h') | KeyCode::Char('?') => {
                    self.show_help = true;
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
                        // macOS `open` percent-encodes the URL, so decode first to avoid double-encoding
                        #[cfg(target_os = "macos")]
                        let open_url = percent_decode(url);
                        #[cfg(not(target_os = "macos"))]
                        let open_url = url.clone();
                        #[cfg(target_os = "macos")]
                        { let _ = std::process::Command::new("open").arg(&open_url).spawn(); }
                        #[cfg(not(target_os = "macos"))]
                        { let _ = std::process::Command::new("xdg-open").arg(&open_url).spawn(); }
                        self.status_msg = format!("Opening: {url}");
                    }
                }
                KeyCode::Char('h') | KeyCode::Char('?') => {
                    self.show_help = true;
                }
                KeyCode::Char('r') => {
                    match self.client.c3_get_link_groups().await {
                        Ok(groups) => {
                            self.c3_link_groups = groups;
                            self.c3_build_link_flat();
                            self.c3_link_popup_selected = self.c3_link_popup_selected.min(self.c3_link_flat.len().saturating_sub(1));
                            self.status_msg = format!("Refreshed {} link groups", self.c3_link_groups.len());
                        }
                        Err(e) => {
                            self.status_msg = format!("Error refreshing link groups: {e}");
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Card definition popup handler
        if self.c3_show_card_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('C') | KeyCode::Char('q') => {
                    self.c3_show_card_popup = false;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.c3_card_popup_scroll = self.c3_card_popup_scroll.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.c3_card_popup_scroll = self.c3_card_popup_scroll.saturating_add(10);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.c3_card_popup_scroll = self.c3_card_popup_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.c3_card_popup_scroll = self.c3_card_popup_scroll.saturating_add(1);
                }
                KeyCode::Home => self.c3_card_popup_scroll = 0,
                KeyCode::End => self.c3_card_popup_scroll = u16::MAX,
                KeyCode::Char('s') | KeyCode::Char('w') => {
                    // Write card definition to /tmp file
                    let actual_idx = self.c3_tree_order.get(self.c3_selected).and_then(|t| t.result_idx());
                    if let Some(actual_idx) = actual_idx {
                        if let Some(result) = self.c3_results.get(actual_idx) {
                            let card = self.c3_integrations.iter()
                                .find(|i| i.name == result.name)
                                .and_then(|i| i.card.as_ref());
                            let text = if let Some(card) = card {
                                format!("{:#?}", card)
                            } else {
                                "No card definition found.".to_string()
                            };
                            let path = "/tmp/alkeme-card.txt";
                            match std::fs::write(path, &text) {
                                Ok(_) => self.status_msg = format!("Card written to {path}"),
                                Err(e) => self.status_msg = format!("Error writing card: {e}"),
                            }
                        }
                    }
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
                                        self.client.c3_create_view(&name, &integrations, &[], &[])
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

        // C3 history filter mode
        if self.c3_history_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.c3_history_filtering = false;
                }
                KeyCode::Enter => {
                    self.c3_history_filtering = false;
                }
                KeyCode::Backspace => {
                    self.c3_history_filter.pop();
                    self.c3_history_selected = 0;
                    self.c3_history_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.c3_history_filter.push(c);
                    self.c3_history_selected = 0;
                    self.c3_history_table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        // Tags editor popup
        if self.c3_show_tags_popup {
            match key.code {
                KeyCode::Esc => {
                    self.c3_show_tags_popup = false;
                }
                KeyCode::Enter => {
                    self.c3_tags = self.c3_tags_edit
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    self.c3_show_tags_popup = false;
                    if self.c3_tags.is_empty() {
                        self.status_msg = "Tags cleared".to_string();
                    } else {
                        self.status_msg = format!("Tags set: {}", self.c3_tags.join(", "));
                    }
                }
                KeyCode::Backspace => {
                    self.c3_tags_edit.pop();
                }
                KeyCode::Char(c) => {
                    self.c3_tags_edit.push(c);
                }
                _ => {}
            }
            return;
        }

        // Date range editor popup
        if self.c3_show_date_popup {
            match key.code {
                KeyCode::Esc => {
                    self.c3_show_date_popup = false;
                }
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    self.c3_date_field = 1 - self.c3_date_field;
                }
                KeyCode::Enter => {
                    let start_parsed = parse_date_input(&self.c3_date_start_edit);
                    let stop_parsed = parse_date_input(&self.c3_date_stop_edit);
                    if let (Some(s), Some(e)) = (start_parsed, stop_parsed) {
                        self.c3_start_date = s;
                        self.c3_stop_date = e;
                        self.c3_show_date_popup = false;
                        let days = (e - s).num_days();
                        self.status_msg = format!("Date range set: {} days", days);
                    } else {
                        self.status_msg = "Invalid date format. Use: now, -5h, -7d, -1w, -3M, or YYYY-MM-DD".to_string();
                    }
                }
                KeyCode::Backspace => {
                    if self.c3_date_field == 0 {
                        self.c3_date_start_edit.pop();
                    } else {
                        self.c3_date_stop_edit.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if self.c3_date_field == 0 {
                        self.c3_date_start_edit.push(c);
                    } else {
                        self.c3_date_stop_edit.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        // JSON save filename prompt
        if self.c3_save_json_prompt.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.c3_save_json_prompt = None;
                }
                KeyCode::Enter => {
                    if let Some(filename) = self.c3_save_json_prompt.take() {
                        if filename.is_empty() {
                            self.status_msg = "No filename provided".to_string();
                        } else {
                            self.c3_save_json(&filename);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(ref mut f) = self.c3_save_json_prompt {
                        f.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut f) = self.c3_save_json_prompt {
                        f.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        // Settings confirm dialog
        if let Some((action, _msg)) = &self.c3_settings_confirm {
            let action = action.clone();
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(id) = action.strip_prefix("delete_view:") {
                        let id = id.to_string();
                        let status = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                self.client.c3_delete_view(&id)
                            )
                        });
                        match status {
                            Ok(_) => {
                                self.status_msg = "View deleted".to_string();
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_settings_views().await;
                                        self.c3_fetch_views().await;
                                    })
                                });
                            }
                            Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                        }
                    } else if let Some(id) = action.strip_prefix("delete_link_group:") {
                        let id = id.to_string();
                        let status = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                self.client.c3_delete_link_group(&id)
                            )
                        });
                        match status {
                            Ok(_) => {
                                self.status_msg = "Link group deleted".to_string();
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_link_groups_settings().await;
                                        self.c3_fetch_link_groups().await;
                                    })
                                });
                            }
                            Err(e) => self.status_msg = format!("Error deleting link group: {e}"),
                        }
                    }
                    self.c3_settings_confirm = None;
                }
                _ => {
                    self.c3_settings_confirm = None;
                }
            }
            return;
        }

        // Role selection sub-popup within view editor
        if self.c3_role_popup_open {
            if self.c3_role_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.c3_role_popup_filtering = false;
                    }
                    KeyCode::Enter => {
                        self.c3_role_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.c3_role_popup_filter.pop();
                        self.c3_role_popup_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.c3_role_popup_filter.push(c);
                        self.c3_role_popup_selected = 0;
                    }
                    _ => {}
                }
                return;
            }
            let filtered = self.c3_role_popup_filtered_roles();
            match key.code {
                KeyCode::Esc => {
                    self.c3_role_popup_open = false;
                    self.c3_role_popup_filter.clear();
                }
                KeyCode::Char('/') => {
                    self.c3_role_popup_filtering = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.c3_role_popup_selected > 0 {
                        self.c3_role_popup_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.c3_role_popup_selected + 1 < filtered.len() {
                        self.c3_role_popup_selected += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(&idx) = filtered.get(self.c3_role_popup_selected) {
                        let roles = if self.c3_role_popup_for_edit {
                            &mut self.c3_view_editor_edit_roles
                        } else {
                            &mut self.c3_view_editor_view_roles
                        };
                        roles[idx].1 = !roles[idx].1;
                    }
                }
                KeyCode::Char('a') => {
                    let roles = if self.c3_role_popup_for_edit {
                        &mut self.c3_view_editor_edit_roles
                    } else {
                        &mut self.c3_view_editor_view_roles
                    };
                    for r in roles.iter_mut() { r.1 = true; }
                }
                KeyCode::Char('n') => {
                    let roles = if self.c3_role_popup_for_edit {
                        &mut self.c3_view_editor_edit_roles
                    } else {
                        &mut self.c3_view_editor_view_roles
                    };
                    for r in roles.iter_mut() { r.1 = false; }
                }
                _ => {}
            }
            return;
        }

        // View editor
        if self.c3_view_editor_open {
            // Integration filter mode within editor
            if self.c3_view_editor_integration_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.c3_view_editor_integration_filtering = false;
                    }
                    KeyCode::Enter => {
                        self.c3_view_editor_integration_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.c3_view_editor_integration_filter.pop();
                        self.c3_view_editor_integration_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.c3_view_editor_integration_filter.push(c);
                        self.c3_view_editor_integration_selected = 0;
                    }
                    _ => {}
                }
                return;
            }

            match self.c3_view_editor_field {
                C3ViewEditorField::Name => {
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.next();
                            self.c3_view_editor_integration_selected = 0;
                        }
                        KeyCode::BackTab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.prev();
                        }
                        KeyCode::Left => {
                            if self.c3_view_editor_name_cursor > 0 {
                                self.c3_view_editor_name_cursor -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if self.c3_view_editor_name_cursor < self.c3_view_editor_name.len() {
                                self.c3_view_editor_name_cursor += 1;
                            }
                        }
                        KeyCode::Home => self.c3_view_editor_name_cursor = 0,
                        KeyCode::End => self.c3_view_editor_name_cursor = self.c3_view_editor_name.len(),
                        KeyCode::Backspace => {
                            if self.c3_view_editor_name_cursor > 0 {
                                self.c3_view_editor_name_cursor -= 1;
                                self.c3_view_editor_name.remove(self.c3_view_editor_name_cursor);
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_save_view_editor();
                        }
                        KeyCode::Char(c) => {
                            self.c3_view_editor_name.insert(self.c3_view_editor_name_cursor, c);
                            self.c3_view_editor_name_cursor += 1;
                        }
                        _ => {}
                    }
                }
                C3ViewEditorField::Integrations => {
                    let filtered = self.c3_view_editor_filtered_integrations();
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.prev();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.c3_view_editor_integration_selected > 0 {
                                self.c3_view_editor_integration_selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.c3_view_editor_integration_selected + 1 < filtered.len() {
                                self.c3_view_editor_integration_selected += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(&idx) = filtered.get(self.c3_view_editor_integration_selected) {
                                self.c3_view_editor_integrations[idx].1 = !self.c3_view_editor_integrations[idx].1;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.c3_view_editor_integration_filtering = true;
                        }
                        KeyCode::Char('a') => {
                            for i in &mut self.c3_view_editor_integrations { i.1 = true; }
                        }
                        KeyCode::Char('n') => {
                            for i in &mut self.c3_view_editor_integrations { i.1 = false; }
                        }
                        KeyCode::Char('!') => {
                            for i in &mut self.c3_view_editor_integrations { i.1 = !i.1; }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_save_view_editor();
                        }
                        _ => {}
                    }
                }
                C3ViewEditorField::ViewRoles => {
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.prev();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.c3_role_popup_open = true;
                            self.c3_role_popup_for_edit = false;
                            self.c3_role_popup_selected = 0;
                            self.c3_role_popup_filter.clear();
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_save_view_editor();
                        }
                        _ => {}
                    }
                }
                C3ViewEditorField::EditRoles => {
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.c3_view_editor_field = self.c3_view_editor_field.prev();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.c3_role_popup_open = true;
                            self.c3_role_popup_for_edit = true;
                            self.c3_role_popup_selected = 0;
                            self.c3_role_popup_filter.clear();
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_save_view_editor();
                        }
                        _ => {}
                    }
                }
            }
            return;
        }

        // Integration config editor
        if self.c3_int_editor_open {
            match key.code {
                KeyCode::Esc => {
                    self.c3_int_editor_open = false;
                }
                KeyCode::Up | KeyCode::Char('k') if !self.c3_int_editor_values.is_empty() => {
                    if self.c3_int_editor_selected > 0 {
                        self.c3_int_editor_selected -= 1;
                        let val = &self.c3_int_editor_values[self.c3_int_editor_selected].1;
                        self.c3_int_editor_cursor = val.len();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') if !self.c3_int_editor_values.is_empty() => {
                    if self.c3_int_editor_selected + 1 < self.c3_int_editor_values.len() {
                        self.c3_int_editor_selected += 1;
                        let val = &self.c3_int_editor_values[self.c3_int_editor_selected].1;
                        self.c3_int_editor_cursor = val.len();
                    }
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Copy editor values back into settings
                    let idx = self.c3_int_editor_idx;
                    if idx < self.c3_int_settings.len() {
                        for (field_name, value, _, _, _, _) in &self.c3_int_editor_values {
                            self.c3_int_settings[idx].values.insert(field_name.clone(), value.clone());
                        }
                    }
                    self.c3_int_editor_open = false;
                    // Save all settings
                    let payload = self.c3_build_int_settings_payload();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            match self.client.c3_put_integration_settings(&payload).await {
                                Ok(_) => {
                                    self.status_msg = "Integration settings saved".to_string();
                                    self.c3_int_settings_dirty = false;
                                    self.c3_fetch_integration_settings().await;
                                }
                                Err(e) => {
                                    self.status_msg = format!("Error saving settings: {e}");
                                }
                            }
                        })
                    });
                }
                KeyCode::Char('p') => {
                    self.c3_int_editor_show_password = !self.c3_int_editor_show_password;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(entry) = self.c3_int_editor_values.get_mut(self.c3_int_editor_selected) {
                        if entry.3 { // is_boolean
                            entry.1 = if entry.1 == "true" { "false".to_string() } else { "true".to_string() };
                            self.c3_int_settings_dirty = true;
                        }
                    }
                }
                _ => {
                    // Text input for non-boolean fields
                    if let Some(entry) = self.c3_int_editor_values.get(self.c3_int_editor_selected) {
                        if !entry.3 { // not boolean
                            let locked = self.c3_int_editor_idx < self.c3_int_settings.len() && self.c3_int_settings[self.c3_int_editor_idx].locked;
                            if !locked {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if let Some(entry) = self.c3_int_editor_values.get_mut(self.c3_int_editor_selected) {
                                            entry.1.insert(self.c3_int_editor_cursor, c);
                                            self.c3_int_editor_cursor += 1;
                                            self.c3_int_settings_dirty = true;
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        if self.c3_int_editor_cursor > 0 {
                                            if let Some(entry) = self.c3_int_editor_values.get_mut(self.c3_int_editor_selected) {
                                                entry.1.remove(self.c3_int_editor_cursor - 1);
                                                self.c3_int_editor_cursor -= 1;
                                                self.c3_int_settings_dirty = true;
                                            }
                                        }
                                    }
                                    KeyCode::Delete => {
                                        if let Some(entry) = self.c3_int_editor_values.get_mut(self.c3_int_editor_selected) {
                                            if self.c3_int_editor_cursor < entry.1.len() {
                                                entry.1.remove(self.c3_int_editor_cursor);
                                                self.c3_int_settings_dirty = true;
                                            }
                                        }
                                    }
                                    KeyCode::Left => {
                                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            // Word jump left
                                            if let Some(entry) = self.c3_int_editor_values.get(self.c3_int_editor_selected) {
                                                let bytes = entry.1.as_bytes();
                                                let mut pos = self.c3_int_editor_cursor;
                                                while pos > 0 && bytes.get(pos - 1) == Some(&b' ') { pos -= 1; }
                                                while pos > 0 && bytes.get(pos - 1) != Some(&b' ') { pos -= 1; }
                                                self.c3_int_editor_cursor = pos;
                                            }
                                        } else {
                                            self.c3_int_editor_cursor = self.c3_int_editor_cursor.saturating_sub(1);
                                        }
                                    }
                                    KeyCode::Right => {
                                        if let Some(entry) = self.c3_int_editor_values.get(self.c3_int_editor_selected) {
                                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                // Word jump right
                                                let bytes = entry.1.as_bytes();
                                                let len = entry.1.len();
                                                let mut pos = self.c3_int_editor_cursor;
                                                while pos < len && bytes.get(pos) != Some(&b' ') { pos += 1; }
                                                while pos < len && bytes.get(pos) == Some(&b' ') { pos += 1; }
                                                self.c3_int_editor_cursor = pos;
                                            } else if self.c3_int_editor_cursor < entry.1.len() {
                                                self.c3_int_editor_cursor += 1;
                                            }
                                        }
                                    }
                                    KeyCode::Home => {
                                        self.c3_int_editor_cursor = 0;
                                    }
                                    KeyCode::End => {
                                        if let Some(entry) = self.c3_int_editor_values.get(self.c3_int_editor_selected) {
                                            self.c3_int_editor_cursor = entry.1.len();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        // Link group settings editor intercept
        if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
            match self.c3_lg_level {
                C3LinkGroupLevel::LinkEditor => {
                    let all_itypes = ["domain", "ip", "url", "email", "hash", "phone", "text"];
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_lg_level = C3LinkGroupLevel::LinkList;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Apply edits back to the group's link
                            let gi = self.c3_lg_editing_group_idx;
                            let li = self.c3_lg_editor_link_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                if let Some(link) = group.links.get_mut(li) {
                                    *link = self.c3_lg_editor_link.clone();
                                }
                            }
                            self.c3_lg_level = C3LinkGroupLevel::LinkList;
                        }
                        KeyCode::Up | KeyCode::Char('k') if self.c3_lg_editor_field == C3LinkEditorField::Itypes => {
                            if self.c3_lg_editor_itype_selected > 0 {
                                self.c3_lg_editor_itype_selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') if self.c3_lg_editor_field == C3LinkEditorField::Itypes => {
                            if self.c3_lg_editor_itype_selected + 1 < all_itypes.len() {
                                self.c3_lg_editor_itype_selected += 1;
                            }
                        }
                        KeyCode::Char(' ') if self.c3_lg_editor_field == C3LinkEditorField::Itypes => {
                            let itype = all_itypes[self.c3_lg_editor_itype_selected].to_string();
                            if let Some(pos) = self.c3_lg_editor_link.itypes.iter().position(|t| t == &itype) {
                                self.c3_lg_editor_link.itypes.remove(pos);
                            } else {
                                self.c3_lg_editor_link.itypes.push(itype);
                            }
                        }
                        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                            let fields = C3LinkEditorField::all();
                            if let Some(pos) = fields.iter().position(|f| *f == self.c3_lg_editor_field) {
                                self.c3_lg_editor_field = fields[(pos + 1) % fields.len()];
                                self.c3_lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                self.c3_lg_editor_itype_selected = 0;
                            }
                        }
                        KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                            let fields = C3LinkEditorField::all();
                            if let Some(pos) = fields.iter().position(|f| *f == self.c3_lg_editor_field) {
                                self.c3_lg_editor_field = fields[(pos + fields.len() - 1) % fields.len()];
                                self.c3_lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                self.c3_lg_editor_itype_selected = 0;
                            }
                        }
                        _ if self.c3_lg_editor_field != C3LinkEditorField::Itypes => {
                            // Text input for non-itypes fields
                            match key.code {
                                KeyCode::Char(c) => {
                                    let pos = self.c3_lg_editor_cursor;
                                    self.c3_lg_editor_field_value_mut().insert(pos, c);
                                    self.c3_lg_editor_cursor += 1;
                                }
                                KeyCode::Backspace => {
                                    if self.c3_lg_editor_cursor > 0 {
                                        self.c3_lg_editor_cursor -= 1;
                                        let pos = self.c3_lg_editor_cursor;
                                        self.c3_lg_editor_field_value_mut().remove(pos);
                                    }
                                }
                                KeyCode::Delete => {
                                    let len = self.c3_lg_editor_field_value().len();
                                    let pos = self.c3_lg_editor_cursor;
                                    if pos < len {
                                        self.c3_lg_editor_field_value_mut().remove(pos);
                                    }
                                }
                                KeyCode::Left => {
                                    self.c3_lg_editor_cursor = self.c3_lg_editor_cursor.saturating_sub(1);
                                }
                                KeyCode::Right => {
                                    let len = self.c3_lg_editor_field_value().len();
                                    if self.c3_lg_editor_cursor < len {
                                        self.c3_lg_editor_cursor += 1;
                                    }
                                }
                                KeyCode::Home => self.c3_lg_editor_cursor = 0,
                                KeyCode::End => {
                                    self.c3_lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    return;
                }
                C3LinkGroupLevel::LinkList => {
                    // Link list filter mode
                    if self.c3_lg_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.c3_lg_filtering = false;
                            }
                            KeyCode::Backspace => {
                                self.c3_lg_filter.pop();
                            }
                            KeyCode::Char(c) => {
                                self.c3_lg_filter.push(c);
                            }
                            _ => {}
                        }
                        return;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.c3_lg_level = C3LinkGroupLevel::GroupList;
                        }
                        KeyCode::Enter => {
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get(gi) {
                                if let Some(link) = group.links.get(self.c3_lg_links_selected) {
                                    if !link.is_separator() {
                                        self.c3_lg_editor_link = link.clone();
                                        self.c3_lg_editor_link_idx = self.c3_lg_links_selected;
                                        self.c3_lg_editor_field = C3LinkEditorField::Name;
                                        self.c3_lg_editor_cursor = link.name.len();
                                        self.c3_lg_editor_itype_selected = 0;
                                        self.c3_lg_level = C3LinkGroupLevel::LinkEditor;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                group.links.push(crate::api::Cont3xtLink {
                                    name: "New Link".to_string(),
                                    url: String::new(),
                                    itypes: vec!["domain".to_string(), "ip".to_string(), "url".to_string()],
                                    info: String::new(),
                                    color: String::new(),
                                    external_doc_name: String::new(),
                                    external_doc_url: String::new(),
                                });
                                self.c3_lg_links_selected = group.links.len() - 1;
                                self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                            }
                        }
                        KeyCode::Char('a') => {
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                group.links.push(crate::api::Cont3xtLink::new_separator());
                                self.c3_lg_links_selected = group.links.len() - 1;
                                self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('x') => {
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                if self.c3_lg_links_selected < group.links.len() {
                                    group.links.remove(self.c3_lg_links_selected);
                                    if self.c3_lg_links_selected >= group.links.len() && !group.links.is_empty() {
                                        self.c3_lg_links_selected = group.links.len() - 1;
                                    }
                                    self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                                }
                            }
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            // Reorder: swap with previous
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                if self.c3_lg_links_selected > 0 {
                                    group.links.swap(self.c3_lg_links_selected, self.c3_lg_links_selected - 1);
                                    self.c3_lg_links_selected -= 1;
                                    self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                                }
                            }
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            // Reorder: swap with next
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get_mut(gi) {
                                if self.c3_lg_links_selected + 1 < group.links.len() {
                                    group.links.swap(self.c3_lg_links_selected, self.c3_lg_links_selected + 1);
                                    self.c3_lg_links_selected += 1;
                                    self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                                }
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Save the group to server
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get(gi) {
                                let group_json = self.c3_lg_build_group_json(group);
                                let group_id = group.id.clone();
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        match self.client.c3_update_link_group(&group_id, &group_json).await {
                                            Ok(_) => {
                                                self.status_msg = "Link group saved".to_string();
                                                self.c3_fetch_link_groups_settings().await;
                                                self.c3_fetch_link_groups().await;
                                            }
                                            Err(e) => {
                                                self.status_msg = format!("Error saving link group: {e}");
                                            }
                                        }
                                    })
                                });
                                // Stay in link list, re-select the group we were editing
                                let new_idx = self.c3_lg_groups.iter().position(|g| g.id == group_id).unwrap_or(0);
                                self.c3_lg_editing_group_idx = new_idx;
                                self.c3_lg_links_selected = 0;
                                self.c3_lg_links_table_state.select(Some(0));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let gi = self.c3_lg_editing_group_idx;
                            if let Some(group) = self.c3_lg_groups.get(gi) {
                                if self.c3_lg_links_selected + 1 < group.links.len() {
                                    self.c3_lg_links_selected += 1;
                                    self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.c3_lg_links_selected > 0 {
                                self.c3_lg_links_selected -= 1;
                                self.c3_lg_links_table_state.select(Some(self.c3_lg_links_selected));
                            }
                        }
                        _ => {}
                    }
                    return;
                }
                C3LinkGroupLevel::GroupList => {
                    // Group list filter mode
                    if self.c3_lg_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.c3_lg_filtering = false;
                            }
                            KeyCode::Backspace => {
                                self.c3_lg_filter.pop();
                                self.c3_lg_selected = 0;
                                self.c3_lg_table_state.select(Some(0));
                            }
                            KeyCode::Char(c) => {
                                self.c3_lg_filter.push(c);
                                self.c3_lg_selected = 0;
                                self.c3_lg_table_state.select(Some(0));
                            }
                            _ => {}
                        }
                        return;
                    }
                    // handled below in the main match
                }
            }
        }

        // Integration settings filter
        if self.c3_int_settings_filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.c3_int_settings_filtering = false;
                }
                KeyCode::Backspace => {
                    self.c3_int_settings_filter.pop();
                    self.c3_int_settings_selected = 0;
                    self.c3_int_settings_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.c3_int_settings_filter.push(c);
                    self.c3_int_settings_selected = 0;
                    self.c3_int_settings_table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        // Settings views filter
        if self.c3_settings_views_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.c3_settings_views_filtering = false;
                }
                KeyCode::Enter => {
                    self.c3_settings_views_filtering = false;
                }
                KeyCode::Backspace => {
                    self.c3_settings_views_filter.pop();
                    self.c3_settings_views_selected = 0;
                    self.c3_settings_views_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.c3_settings_views_filter.push(c);
                    self.c3_settings_views_selected = 0;
                    self.c3_settings_views_table_state.select(Some(0));
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
                if self.active_tab == Tab::History && !self.c3_history_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
                }
                if self.active_tab == Tab::Settings && !self.c3_settings_views_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            self.c3_fetch_settings_views().await;
                            self.c3_fetch_roles().await;
                        })
                    });
                }
                if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations && !self.c3_int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
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
                if self.active_tab == Tab::History && !self.c3_history_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
                }
                if self.active_tab == Tab::Settings && !self.c3_settings_views_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            self.c3_fetch_settings_views().await;
                            self.c3_fetch_roles().await;
                        })
                    });
                }
                if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations && !self.c3_int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
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
            KeyCode::Char('/') if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('/') | KeyCode::Char('E') if self.active_tab == Tab::Search => {
                self.enter_expression_mode();
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
            KeyCode::Char('C') if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail => {
                // Show card popup for results, overview definition for indicators
                self.c3_show_card_popup = !self.c3_show_card_popup;
                self.c3_card_popup_scroll = 0;
            }
            KeyCode::Char('o') if self.active_tab == Tab::Search => {
                // Overview selector — only when on an indicator
                if let Some(C3TreeItem::Indicator(itype, _)) = self.c3_tree_order.get(self.c3_selected) {
                    let itype_lower = itype.to_lowercase();
                    let mut matching: Vec<&crate::api::Cont3xtOverview> = self.c3_overviews.iter()
                        .filter(|o| o.itype.to_lowercase() == itype_lower)
                        .collect();
                    matching.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    if !matching.is_empty() {
                        let current_id = self.c3_selected_overviews.get(&itype_lower);
                        self.c3_overview_popup_selected = matching.iter().position(|o| {
                            Some(&o.id) == current_id
                        }).or_else(|| matching.iter().position(|o| o.is_default))
                            .unwrap_or(0);
                        self.c3_overview_popup_filter.clear();
                        self.c3_overview_popup_filtering = false;
                        self.c3_show_overview_popup = true;
                    } else {
                        self.status_msg = format!("No overviews for type '{}'", itype);
                    }
                }
            }
            KeyCode::Char('J') if self.active_tab == Tab::Search => {
                if self.c3_results.is_empty() {
                    self.status_msg = "No results to save".to_string();
                } else {
                    let default_name = format!("{}.json", self.expression.replace(['/', '\\', ' '], "_"));
                    self.c3_save_json_prompt = Some(default_name);
                }
            }
            KeyCode::Char('t') if self.active_tab == Tab::Search => {
                self.c3_tags_edit = self.c3_tags.join(", ");
                self.c3_show_tags_popup = true;
            }
            KeyCode::Char('d') if self.active_tab == Tab::Search => {
                self.c3_date_field = 0;
                self.c3_show_date_popup = true;
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
            KeyCode::Char('I') | KeyCode::Char('v') if self.active_tab == Tab::Search => {
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
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Results {
                    if let Some(name) = self.c3_current_integration_name() {
                        for i in (self.c3_selected + 1)..self.c3_tree_order.len() {
                            if let Some(idx) = self.c3_tree_order[i].result_idx() {
                                if self.c3_results.get(idx).map(|r| r.name.as_str()) == Some(&name) {
                                    self.c3_selected = i;
                                    self.c3_detail_scroll = 0;
                                    self.c3_detail_hscroll = 0;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Results {
                    if let Some(name) = self.c3_current_integration_name() {
                        for i in (0..self.c3_selected).rev() {
                            if let Some(idx) = self.c3_tree_order[i].result_idx() {
                                if self.c3_results.get(idx).map(|r| r.name.as_str()) == Some(&name) {
                                    self.c3_selected = i;
                                    self.c3_detail_scroll = 0;
                                    self.c3_detail_hscroll = 0;
                                    break;
                                }
                            }
                        }
                    }
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
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.c3_history_selected = (self.c3_history_selected + self.visible_rows).min(len - 1);
                        self.c3_history_table_state.select(Some(self.c3_history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views {
                    let filtered = self.c3_settings_filtered_views();
                    if !filtered.is_empty() {
                        self.c3_settings_views_selected = (self.c3_settings_views_selected + self.visible_rows).min(filtered.len() - 1);
                        self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if !filtered.is_empty() {
                        self.c3_int_settings_selected = (self.c3_int_settings_selected + self.visible_rows).min(filtered.len() - 1);
                        self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if !filtered.is_empty() {
                        self.c3_lg_selected = (self.c3_lg_selected + self.visible_rows).min(filtered.len() - 1);
                        self.c3_lg_table_state.select(Some(self.c3_lg_selected));
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
                } else if self.active_tab == Tab::History {
                    self.c3_history_selected = self.c3_history_selected.saturating_sub(self.visible_rows);
                    self.c3_history_table_state.select(Some(self.c3_history_selected));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views {
                    self.c3_settings_views_selected = self.c3_settings_views_selected.saturating_sub(self.visible_rows);
                    self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    self.c3_int_settings_selected = self.c3_int_settings_selected.saturating_sub(self.visible_rows);
                    self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    self.c3_lg_selected = self.c3_lg_selected.saturating_sub(self.visible_rows);
                    self.c3_lg_table_state.select(Some(self.c3_lg_selected));
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
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.c3_history_selected = (self.c3_history_selected + 1).min(len - 1);
                        self.c3_history_table_state.select(Some(self.c3_history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views {
                    let filtered = self.c3_settings_filtered_views();
                    if self.c3_settings_views_selected + 1 < filtered.len() {
                        self.c3_settings_views_selected += 1;
                        self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if self.c3_int_settings_selected + 1 < filtered.len() {
                        self.c3_int_settings_selected += 1;
                        self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if self.c3_lg_selected + 1 < filtered.len() {
                        self.c3_lg_selected += 1;
                        self.c3_lg_table_state.select(Some(self.c3_lg_selected));
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
                } else if self.active_tab == Tab::History {
                    self.c3_history_selected = self.c3_history_selected.saturating_sub(1);
                    self.c3_history_table_state.select(Some(self.c3_history_selected));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views {
                    if self.c3_settings_views_selected > 0 {
                        self.c3_settings_views_selected -= 1;
                        self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    if self.c3_int_settings_selected > 0 {
                        self.c3_int_settings_selected -= 1;
                        self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    if self.c3_lg_selected > 0 {
                        self.c3_lg_selected -= 1;
                        self.c3_lg_table_state.select(Some(self.c3_lg_selected));
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
                } else if self.active_tab == Tab::History {
                    self.c3_history_selected = 0;
                    self.c3_history_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    self.c3_int_settings_selected = 0;
                    self.c3_int_settings_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    self.c3_lg_selected = 0;
                    self.c3_lg_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings {
                    self.c3_settings_views_selected = 0;
                    self.c3_settings_views_table_state.select(Some(0));
                }
            }
            KeyCode::End => {
                if self.active_tab == Tab::Search && self.c3_focus == Cont3xtFocus::Detail {
                    self.c3_detail_scroll = u16::MAX;
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.c3_history_selected = len - 1;
                        self.c3_history_table_state.select(Some(self.c3_history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if !filtered.is_empty() {
                        self.c3_int_settings_selected = filtered.len() - 1;
                        self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if !filtered.is_empty() {
                        self.c3_lg_selected = filtered.len() - 1;
                        self.c3_lg_table_state.select(Some(self.c3_lg_selected));
                    }
                } else if self.active_tab == Tab::Settings {
                    let filtered = self.c3_settings_filtered_views();
                    if !filtered.is_empty() {
                        self.c3_settings_views_selected = filtered.len() - 1;
                        self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
                    }
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
                } else if self.active_tab == Tab::History && self.c3_history_page > 1 {
                    self.c3_history_page -= 1;
                    self.c3_history_selected = 0;
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
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
                } else if self.active_tab == Tab::History {
                    let total_pages = (self.c3_history_total + 99) / 100;
                    if self.c3_history_page < total_pages {
                        self.c3_history_page += 1;
                        self.c3_history_selected = 0;
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                        });
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
            // History tab keys
            KeyCode::Char('/') if self.active_tab == Tab::History => {
                self.c3_history_filtering = true;
            }
            KeyCode::Char('s') if self.active_tab == Tab::History => {
                let sortable_cols: Vec<usize> = C3_HISTORY_COLUMNS.iter().enumerate()
                    .filter(|(_, c)| c.3).map(|(i, _)| i).collect();
                if let Some(pos) = sortable_cols.iter().position(|&i| i == self.c3_history_sort_col) {
                    self.c3_history_sort_col = sortable_cols[(pos + 1) % sortable_cols.len()];
                } else if !sortable_cols.is_empty() {
                    self.c3_history_sort_col = sortable_cols[0];
                }
                self.c3_history_page = 1;
                self.c3_history_selected = 0;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                });
            }
            KeyCode::Char('S') if self.active_tab == Tab::History => {
                self.c3_history_sort_desc = !self.c3_history_sort_desc;
                self.c3_history_page = 1;
                self.c3_history_selected = 0;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                });
            }
            KeyCode::Char('r') if self.active_tab == Tab::History => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                });
            }
            KeyCode::Char('d') if self.active_tab == Tab::History => {
                if let Some(item) = self.c3_history_data.get(self.c3_history_selected) {
                    if let Some(id) = item.get("_id").and_then(|v| v.as_str()) {
                        let id = id.to_string();
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(self.c3_delete_history(&id))
                        });
                    }
                }
            }
            KeyCode::Enter if self.active_tab == Tab::History => {
                // Re-run the search from the selected history entry
                if let Some(item) = self.c3_history_data.get(self.c3_history_selected) {
                    let indicator = item.get("indicator").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if !indicator.is_empty() {
                        self.expression = indicator;
                        self.active_tab = Tab::Search;
                        self.c3_focus = Cont3xtFocus::Results;
                        self.c3_request_search();
                    }
                }
            }
            // Settings tab keys
            KeyCode::Char('1') if self.active_tab == Tab::Settings => {
                self.c3_settings_tab = C3SettingsTab::Views;
            }
            KeyCode::Char('2') if self.active_tab == Tab::Settings => {
                self.c3_settings_tab = C3SettingsTab::Integrations;
                if !self.c3_int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                    });
                }
            }
            KeyCode::Char('3') if self.active_tab == Tab::Settings => {
                self.c3_settings_tab = C3SettingsTab::Overviews;
            }
            KeyCode::Char('4') if self.active_tab == Tab::Settings => {
                self.c3_settings_tab = C3SettingsTab::LinkGroups;
                if !self.c3_lg_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_link_groups_settings())
                    });
                }
            }
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.c3_fetch_settings_views().await;
                        self.c3_fetch_roles().await;
                    })
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                self.c3_settings_views_sort = (self.c3_settings_views_sort + 1) % 5;
                self.c3_settings_views_selected = 0;
                self.c3_settings_views_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                self.c3_settings_views_sort_desc = !self.c3_settings_views_sort_desc;
                self.c3_settings_views_selected = 0;
                self.c3_settings_views_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                self.c3_settings_views_filtering = true;
            }
            KeyCode::Char('n') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                self.c3_open_new_view_editor();
            }
            KeyCode::Enter | KeyCode::Char('e') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                let filtered = self.c3_settings_filtered_views();
                if let Some(&idx) = filtered.get(self.c3_settings_views_selected) {
                    let view = self.c3_settings_views[idx].clone();
                    if view.editable {
                        self.c3_open_edit_view_editor(&view);
                    } else {
                        self.status_msg = "View is not editable".to_string();
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Views => {
                let filtered = self.c3_settings_filtered_views();
                if let Some(&idx) = filtered.get(self.c3_settings_views_selected) {
                    let view = &self.c3_settings_views[idx];
                    if view.editable {
                        self.c3_settings_confirm = Some((
                            format!("delete_view:{}", view.id),
                            format!("Delete view \"{}\"?", view.name),
                        ));
                    } else {
                        self.status_msg = "View is not editable".to_string();
                    }
                }
            }
            // Integration settings keys
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.c3_int_settings_sort = (self.c3_int_settings_sort + 1) % 2;
                self.c3_int_settings_selected = 0;
                self.c3_int_settings_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations => {
                self.c3_int_settings_sort_desc = !self.c3_int_settings_sort_desc;
                self.c3_int_settings_selected = 0;
                self.c3_int_settings_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations => {
                self.c3_int_settings_filtering = true;
            }
            KeyCode::Char('d') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations => {
                let filtered = self.c3_int_settings_filtered();
                if let Some(&idx) = filtered.get(self.c3_int_settings_selected) {
                    self.c3_int_settings[idx].disabled = !self.c3_int_settings[idx].disabled;
                    self.c3_int_settings_dirty = true;
                }
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations && key.modifiers.contains(KeyModifiers::CONTROL) => {
                let payload = self.c3_build_int_settings_payload();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match self.client.c3_put_integration_settings(&payload).await {
                            Ok(_) => {
                                self.status_msg = "Integration settings saved".to_string();
                                self.c3_int_settings_dirty = false;
                                self.c3_fetch_integration_settings().await;
                            }
                            Err(e) => {
                                self.status_msg = format!("Error saving settings: {e}");
                            }
                        }
                    })
                });
            }
            KeyCode::Enter | KeyCode::Char('e') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::Integrations => {
                let filtered = self.c3_int_settings_filtered();
                if let Some(&idx) = filtered.get(self.c3_int_settings_selected) {
                    let int = &self.c3_int_settings[idx];
                    let values: Vec<(String, String, bool, bool, bool, String)> = int.fields.iter().map(|f| {
                        let val = int.values.get(&f.name).cloned().unwrap_or_default();
                        (f.name.clone(), val, f.password, f.is_boolean, f.required, f.help.clone())
                    }).collect();
                    self.c3_int_editor_open = true;
                    self.c3_int_editor_idx = idx;
                    self.c3_int_editor_values = values;
                    self.c3_int_editor_selected = 0;
                    self.c3_int_editor_cursor = self.c3_int_editor_values.first().map(|v| v.1.len()).unwrap_or(0);
                    self.c3_int_editor_show_password = false;
                }
            }
            // Link group settings keys (GroupList level)
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_link_groups_settings())
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                self.c3_lg_sort_col = (self.c3_lg_sort_col + 1) % 4;
                self.c3_lg_selected = 0;
                self.c3_lg_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                self.c3_lg_sort_desc = !self.c3_lg_sort_desc;
                self.c3_lg_selected = 0;
                self.c3_lg_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                self.c3_lg_filtering = true;
            }
            KeyCode::Char('n') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                // Create a new empty group
                let body = serde_json::json!({
                    "name": "New Group",
                    "links": [],
                    "viewRoles": [],
                    "editRoles": [],
                });
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match self.client.c3_create_link_group(&body).await {
                            Ok(_) => {
                                self.status_msg = "Link group created".to_string();
                                self.c3_fetch_link_groups_settings().await;
                                self.c3_fetch_link_groups().await;
                            }
                            Err(e) => {
                                self.status_msg = format!("Error creating link group: {e}");
                            }
                        }
                    })
                });
            }
            KeyCode::Enter if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                let filtered = self.c3_lg_filtered_groups();
                if let Some(&idx) = filtered.get(self.c3_lg_selected) {
                    let group = &self.c3_lg_groups[idx];
                    if group.editable {
                        self.c3_lg_editing_group_idx = idx;
                        self.c3_lg_links_selected = 0;
                        self.c3_lg_links_table_state.select(Some(0));
                        self.c3_lg_level = C3LinkGroupLevel::LinkList;
                    } else {
                        self.status_msg = "Link group is not editable".to_string();
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') if self.active_tab == Tab::Settings && self.c3_settings_tab == C3SettingsTab::LinkGroups => {
                let filtered = self.c3_lg_filtered_groups();
                if let Some(&idx) = filtered.get(self.c3_lg_selected) {
                    let group = &self.c3_lg_groups[idx];
                    if group.editable {
                        self.c3_settings_confirm = Some((
                            format!("delete_link_group:{}", group.id),
                            format!("Delete link group \"{}\"?", group.name),
                        ));
                    } else {
                        self.status_msg = "Link group is not editable".to_string();
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn c3_settings_filtered_views(&self) -> Vec<usize> {
        let filter = self.c3_settings_views_filter.to_lowercase();
        let mut indices: Vec<usize> = self.c3_settings_views.iter().enumerate()
            .filter(|(_, v)| filter.is_empty() || v.name.to_lowercase().contains(&filter) || v.creator.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let views = &self.c3_settings_views;
        let col = self.c3_settings_views_sort;
        let desc = self.c3_settings_views_sort_desc;
        indices.sort_by(|&a, &b| {
            let cmp = match col {
                0 => views[a].name.to_lowercase().cmp(&views[b].name.to_lowercase()),
                1 => views[a].creator.to_lowercase().cmp(&views[b].creator.to_lowercase()),
                2 => views[a].integrations.len().cmp(&views[b].integrations.len()),
                3 => views[a].view_roles.join(",").cmp(&views[b].view_roles.join(",")),
                _ => views[a].edit_roles.join(",").cmp(&views[b].edit_roles.join(",")),
            };
            if desc { cmp.reverse() } else { cmp }
        });
        indices
    }

    fn c3_save_view_editor(&mut self) {
        let name = self.c3_view_editor_name.trim().to_string();
        if name.is_empty() {
            self.status_msg = "View name cannot be empty".to_string();
            return;
        }
        let integrations: Vec<String> = self.c3_view_editor_integrations.iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(name, _)| name.clone())
            .collect();
        let view_roles: Vec<String> = self.c3_view_editor_view_roles.iter()
            .filter(|(_, sel)| *sel)
            .map(|(r, _)| r.clone())
            .collect();
        let edit_roles: Vec<String> = self.c3_view_editor_edit_roles.iter()
            .filter(|(_, sel)| *sel)
            .map(|(r, _)| r.clone())
            .collect();

        let result = if let Some(id) = &self.c3_view_editor_id {
            let id = id.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    self.client.c3_update_view(&id, &name, &integrations, &view_roles, &edit_roles)
                )
            })
        } else {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    self.client.c3_create_view(&name, &integrations, &view_roles, &edit_roles)
                )
            })
        };

        match result {
            Ok(_) => {
                let action = if self.c3_view_editor_id.is_some() { "Updated" } else { "Created" };
                self.status_msg = format!("{action} view: {name}");
                self.c3_view_editor_open = false;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.c3_fetch_settings_views().await;
                        self.c3_fetch_views().await;
                    })
                });
            }
            Err(e) => self.status_msg = format!("Error saving view: {e}"),
        }
    }
}
