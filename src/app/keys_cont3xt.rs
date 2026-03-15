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
        if self.cont3xt.show_overview_popup {
            if self.cont3xt.overview_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.cont3xt.overview_popup_filtering = false;
                        self.cont3xt.overview_popup_filter.clear();
                        self.cont3xt.overview_popup_selected = 0;
                    }
                    KeyCode::Enter => {
                        self.cont3xt.overview_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.cont3xt.overview_popup_filter.pop();
                        self.cont3xt.overview_popup_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.cont3xt.overview_popup_filter.push(c);
                        self.cont3xt.overview_popup_selected = 0;
                    }
                    _ => {}
                }
                return;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('o') => {
                    self.cont3xt.show_overview_popup = false;
                }
                KeyCode::Char('/') => {
                    self.cont3xt.overview_popup_filtering = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.cont3xt.overview_popup_selected = self.cont3xt.overview_popup_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(C3TreeItem::Indicator(itype, _)) = self.cont3xt.tree_order.get(self.cont3xt.selected) {
                        let itype_lower = itype.to_lowercase();
                        let filter_lower = self.cont3xt.overview_popup_filter.to_lowercase();
                        let count = self.cont3xt.overviews.iter()
                            .filter(|o| o.itype.to_lowercase() == itype_lower)
                            .filter(|o| filter_lower.is_empty() || o.name.to_lowercase().contains(&filter_lower))
                            .count();
                        if count > 0 {
                            self.cont3xt.overview_popup_selected = (self.cont3xt.overview_popup_selected + 1).min(count - 1);
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(ov) = self.c3_overview_filtered_get() {
                        let itype_lower = ov.itype.to_lowercase();
                        self.cont3xt.selected_overviews.insert(itype_lower, ov.id.clone());
                        self.cont3xt.detail_scroll = 0;
                    }
                    self.cont3xt.show_overview_popup = false;
                }
                KeyCode::Char('d') => {
                    if let Some(ov) = self.c3_overview_filtered_get() {
                        let itype_lower = ov.itype.to_lowercase();
                        let ov_id = ov.id.clone();
                        let ov_name = ov.name.clone();
                        self.cont3xt.selected_overviews.insert(itype_lower, ov_id);
                        self.cont3xt.detail_scroll = 0;
                        match self.client.c3_save_selected_overviews(&self.cont3xt.selected_overviews).await {
                            Ok(_) => {
                                self.status_msg = format!("Default overview set: {ov_name}");
                                self.c3_fetch_overviews().await;
                            }
                            Err(e) => self.status_msg = format!("Error saving default: {e}"),
                        }
                        self.cont3xt.show_overview_popup = false;
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
        if self.cont3xt.show_link_popup {
            if self.cont3xt.link_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.cont3xt.link_popup_filtering = false;
                        self.cont3xt.link_popup_filter.clear();
                        self.c3_build_link_flat();
                    }
                    KeyCode::Enter => {
                        self.cont3xt.link_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.cont3xt.link_popup_filter.pop();
                        self.c3_build_link_flat();
                    }
                    KeyCode::Char(c) => {
                        self.cont3xt.link_popup_filter.push(c);
                        self.c3_build_link_flat();
                    }
                    _ => {}
                }
                return;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => {
                    self.cont3xt.show_link_popup = false;
                }
                KeyCode::Char('/') => {
                    self.cont3xt.link_popup_filtering = true;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.cont3xt.link_popup_selected = self.cont3xt.link_popup_selected.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if !self.cont3xt.link_flat.is_empty() {
                        self.cont3xt.link_popup_selected = (self.cont3xt.link_popup_selected + 10).min(self.cont3xt.link_flat.len() - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.cont3xt.link_popup_selected = self.cont3xt.link_popup_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.cont3xt.link_flat.is_empty() {
                        self.cont3xt.link_popup_selected = (self.cont3xt.link_popup_selected + 1).min(self.cont3xt.link_flat.len() - 1);
                    }
                }
                KeyCode::Enter => {
                    if let Some((_, _, url, _, _)) = self.cont3xt.link_flat.get(self.cont3xt.link_popup_selected) {
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
                            self.cont3xt.link_groups = groups;
                            self.c3_build_link_flat();
                            self.cont3xt.link_popup_selected = self.cont3xt.link_popup_selected.min(self.cont3xt.link_flat.len().saturating_sub(1));
                            self.status_msg = format!("Refreshed {} link groups", self.cont3xt.link_groups.len());
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
        if self.cont3xt.show_card_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('C') | KeyCode::Char('q') => {
                    self.cont3xt.show_card_popup = false;
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.cont3xt.card_popup_scroll = self.cont3xt.card_popup_scroll.saturating_sub(10);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.cont3xt.card_popup_scroll = self.cont3xt.card_popup_scroll.saturating_add(10);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.cont3xt.card_popup_scroll = self.cont3xt.card_popup_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.cont3xt.card_popup_scroll = self.cont3xt.card_popup_scroll.saturating_add(1);
                }
                KeyCode::Home => self.cont3xt.card_popup_scroll = 0,
                KeyCode::End => self.cont3xt.card_popup_scroll = u16::MAX,
                KeyCode::Char('s') | KeyCode::Char('w') => {
                    // Write card definition to /tmp file
                    let actual_idx = self.cont3xt.tree_order.get(self.cont3xt.selected).and_then(|t| t.result_idx());
                    if let Some(actual_idx) = actual_idx {
                        if let Some(result) = self.cont3xt.results.get(actual_idx) {
                            let card = self.cont3xt.integrations.iter()
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
        if self.cont3xt.show_integration_popup {
            match self.cont3xt.integration_popup_mode {
                IntegrationPopupMode::SaveInput => {
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
                        }
                        KeyCode::Enter => {
                            if !self.cont3xt.view_save_name.is_empty() {
                                let name = self.cont3xt.view_save_name.clone();
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
                                self.cont3xt.view_save_name.clear();
                                self.cont3xt.view_save_cursor = 0;
                                self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
                            }
                        }
                        _ => {
                            handle_text_input_key(key.code, &mut self.cont3xt.view_save_name, &mut self.cont3xt.view_save_cursor);
                        }
                    }
                }
                IntegrationPopupMode::ConfirmDelete => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(view) = self.cont3xt.views.get(self.cont3xt.view_selected.saturating_sub(1)) {
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
                                        if self.cont3xt.view_selected > self.cont3xt.views.len() {
                                            self.cont3xt.view_selected = self.cont3xt.views.len();
                                        }
                                    }
                                    Err(e) => self.status_msg = format!("Error deleting view: {e}"),
                                }
                            }
                            self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
                        }
                        _ => {
                            self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
                        }
                    }
                }
                IntegrationPopupMode::Views => {
                    // +1 for "Save Current" option at top
                    let list_len = self.cont3xt.views.len() + 1;
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.integration_popup_mode = IntegrationPopupMode::Integrations;
                        }
                        KeyCode::Char('q') => self.cont3xt.show_integration_popup = false,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if list_len > 0 {
                                self.cont3xt.view_selected = (self.cont3xt.view_selected + 1).min(list_len - 1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.cont3xt.view_selected = self.cont3xt.view_selected.saturating_sub(1);
                        }
                        KeyCode::Enter => {
                            if self.cont3xt.view_selected == 0 {
                                // "Save Current" option
                                self.cont3xt.view_save_name.clear();
                                self.cont3xt.view_save_cursor = 0;
                                self.cont3xt.integration_popup_mode = IntegrationPopupMode::SaveInput;
                            } else {
                                // Load a view
                                let view_idx = self.cont3xt.view_selected - 1;
                                if let Some(view) = self.cont3xt.views.get(view_idx) {
                                    let integrations = view.integrations.clone();
                                    let name = view.name.clone();
                                    self.cont3xt.active_view_id = Some(view.id.clone());
                                    self.cont3xt.active_view_name = Some(name.clone());
                                    self.c3_apply_view(&integrations);
                                    self.status_msg = format!("Loaded view: {name}");
                                    self.cont3xt.show_integration_popup = false;
                                }
                            }
                        }
                        KeyCode::Char('x') => {
                            // Delete selected view
                            if self.cont3xt.view_selected > 0 {
                                let view_idx = self.cont3xt.view_selected - 1;
                                if let Some(view) = self.cont3xt.views.get(view_idx) {
                                    if view.editable {
                                        self.cont3xt.integration_popup_mode = IntegrationPopupMode::ConfirmDelete;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                IntegrationPopupMode::Integrations => {
                    let filtered: Vec<usize> = self.cont3xt.integrations.iter().enumerate()
                        .filter(|(_, int)| {
                            self.cont3xt.integration_popup_filter.is_empty()
                            || int.name.to_lowercase().contains(&self.cont3xt.integration_popup_filter.to_lowercase())
                        })
                        .map(|(i, _)| i)
                        .collect();

                    // When filtering mode is active, capture text input
                    if self.cont3xt.integration_popup_filtering {
                        match key.code {
                            KeyCode::Esc => {
                                self.cont3xt.integration_popup_filtering = false;
                                if self.cont3xt.integration_popup_filter.is_empty() {
                                    // nothing to clear, close popup
                                }
                            }
                            KeyCode::Enter => {
                                self.cont3xt.integration_popup_filtering = false;
                            }
                            KeyCode::Backspace => {
                                self.cont3xt.integration_popup_filter.pop();
                                self.cont3xt.integration_popup_selected = 0;
                            }
                            KeyCode::Char(c) => {
                                self.cont3xt.integration_popup_filter.push(c);
                                self.cont3xt.integration_popup_selected = 0;
                            }
                            _ => {}
                        }
                        return;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            if !self.cont3xt.integration_popup_filter.is_empty() {
                                self.cont3xt.integration_popup_filter.clear();
                                self.cont3xt.integration_popup_selected = 0;
                            } else {
                                self.cont3xt.show_integration_popup = false;
                            }
                        }
                        KeyCode::Char('q') => self.cont3xt.show_integration_popup = false,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !filtered.is_empty() {
                                self.cont3xt.integration_popup_selected = (self.cont3xt.integration_popup_selected + 1).min(filtered.len() - 1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.cont3xt.integration_popup_selected = self.cont3xt.integration_popup_selected.saturating_sub(1);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if let Some(&idx) = filtered.get(self.cont3xt.integration_popup_selected) {
                                let name = self.cont3xt.integrations[idx].name.clone();
                                if self.cont3xt.disabled_integrations.contains(&name) {
                                    self.cont3xt.disabled_integrations.remove(&name);
                                } else {
                                    self.cont3xt.disabled_integrations.insert(name);
                                }
                                self.cont3xt.active_view_id = None;
                                self.cont3xt.active_view_name = None;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.cont3xt.integration_popup_filtering = true;
                        }
                        KeyCode::Char('a') => {
                            self.cont3xt.disabled_integrations.clear();
                            self.cont3xt.active_view_id = None;
                            self.cont3xt.active_view_name = None;
                        }
                        KeyCode::Char('n') => {
                            for int in &self.cont3xt.integrations {
                                self.cont3xt.disabled_integrations.insert(int.name.clone());
                            }
                            self.cont3xt.active_view_id = None;
                            self.cont3xt.active_view_name = None;
                        }
                        KeyCode::Char('!') => {
                            let all_names: Vec<String> = self.cont3xt.integrations.iter().map(|i| i.name.clone()).collect();
                            for name in all_names {
                                if self.cont3xt.disabled_integrations.contains(&name) {
                                    self.cont3xt.disabled_integrations.remove(&name);
                                } else {
                                    self.cont3xt.disabled_integrations.insert(name);
                                }
                            }
                            self.cont3xt.active_view_id = None;
                            self.cont3xt.active_view_name = None;
                        }
                        KeyCode::Char('v') => {
                            // Switch to views mode, re-fetch views
                            self.cont3xt.view_selected = 0;
                            self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
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
        if self.cont3xt.stats_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.stats_filtering = false;
                    if self.cont3xt.stats_filter.is_empty() {
                        // nothing to clear
                    }
                }
                KeyCode::Enter => {
                    self.cont3xt.stats_filtering = false;
                }
                KeyCode::Backspace => {
                    self.cont3xt.stats_filter.pop();
                }
                KeyCode::Char(c) => {
                    self.cont3xt.stats_filter.push(c);
                    self.cont3xt.stats_selected = 0;
                    self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                }
                _ => {}
            }
            return;
        }

        // C3 history filter mode
        if self.cont3xt.history_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.history_filtering = false;
                }
                KeyCode::Enter => {
                    self.cont3xt.history_filtering = false;
                }
                KeyCode::Backspace => {
                    self.cont3xt.history_filter.pop();
                    self.cont3xt.history_selected = 0;
                    self.cont3xt.history_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.cont3xt.history_filter.push(c);
                    self.cont3xt.history_selected = 0;
                    self.cont3xt.history_table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        // Tags editor popup
        if self.cont3xt.show_tags_popup {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.show_tags_popup = false;
                }
                KeyCode::Enter => {
                    self.cont3xt.tags = self.cont3xt.tags_edit
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    self.cont3xt.show_tags_popup = false;
                    if self.cont3xt.tags.is_empty() {
                        self.status_msg = "Tags cleared".to_string();
                    } else {
                        self.status_msg = format!("Tags set: {}", self.cont3xt.tags.join(", "));
                    }
                }
                KeyCode::Backspace => {
                    self.cont3xt.tags_edit.pop();
                }
                KeyCode::Char(c) => {
                    self.cont3xt.tags_edit.push(c);
                }
                _ => {}
            }
            return;
        }

        // Date range editor popup
        if self.cont3xt.show_date_popup {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.show_date_popup = false;
                }
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    self.cont3xt.date_field = 1 - self.cont3xt.date_field;
                }
                KeyCode::Enter => {
                    let start_parsed = parse_date_input(&self.cont3xt.date_start_edit);
                    let stop_parsed = parse_date_input(&self.cont3xt.date_stop_edit);
                    if let (Some(s), Some(e)) = (start_parsed, stop_parsed) {
                        self.cont3xt.start_date = s;
                        self.cont3xt.stop_date = e;
                        self.cont3xt.show_date_popup = false;
                        let days = (e - s).num_days();
                        self.status_msg = format!("Date range set: {} days", days);
                    } else {
                        self.status_msg = "Invalid date format. Use: now, -5h, -7d, -1w, -3M, or YYYY-MM-DD".to_string();
                    }
                }
                KeyCode::Backspace => {
                    if self.cont3xt.date_field == 0 {
                        self.cont3xt.date_start_edit.pop();
                    } else {
                        self.cont3xt.date_stop_edit.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if self.cont3xt.date_field == 0 {
                        self.cont3xt.date_start_edit.push(c);
                    } else {
                        self.cont3xt.date_stop_edit.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        // JSON save filename prompt
        if self.cont3xt.save_json_prompt.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.save_json_prompt = None;
                }
                KeyCode::Enter => {
                    if let Some(filename) = self.cont3xt.save_json_prompt.take() {
                        if filename.is_empty() {
                            self.status_msg = "No filename provided".to_string();
                        } else {
                            self.c3_save_json(&filename);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(ref mut f) = self.cont3xt.save_json_prompt {
                        f.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut f) = self.cont3xt.save_json_prompt {
                        f.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        // Settings confirm dialog
        if let Some((action, _msg)) = &self.cont3xt.settings_confirm {
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
                    self.cont3xt.settings_confirm = None;
                }
                _ => {
                    self.cont3xt.settings_confirm = None;
                }
            }
            return;
        }

        // Role selection sub-popup within view editor
        // (Skip when on Settings link groups/overviews tabs which have their own role popup handlers)
        if self.cont3xt.role_popup_open && !(self.active_tab == Tab::Settings
            && matches!(self.cont3xt.settings_tab, C3SettingsTab::LinkGroups | C3SettingsTab::Overviews)) {
            if self.cont3xt.role_popup_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.cont3xt.role_popup_filtering = false;
                    }
                    KeyCode::Enter | KeyCode::Down => {
                        self.cont3xt.role_popup_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.cont3xt.role_popup_filter.pop();
                        self.cont3xt.role_popup_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.cont3xt.role_popup_filter.push(c);
                        self.cont3xt.role_popup_selected = 0;
                    }
                    _ => {}
                }
                return;
            }
            let filtered = self.c3_role_popup_filtered_roles();
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.role_popup_open = false;
                    self.cont3xt.role_popup_filter.clear();
                }
                KeyCode::Char('/') => {
                    self.cont3xt.role_popup_filtering = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.cont3xt.role_popup_selected > 0 {
                        self.cont3xt.role_popup_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.cont3xt.role_popup_selected + 1 < filtered.len() {
                        self.cont3xt.role_popup_selected += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(&idx) = filtered.get(self.cont3xt.role_popup_selected) {
                        let roles = if self.cont3xt.role_popup_for_edit {
                            &mut self.cont3xt.view_editor_edit_roles
                        } else {
                            &mut self.cont3xt.view_editor_view_roles
                        };
                        roles[idx].1 = !roles[idx].1;
                    }
                }
                KeyCode::Char('a') => {
                    let roles = if self.cont3xt.role_popup_for_edit {
                        &mut self.cont3xt.view_editor_edit_roles
                    } else {
                        &mut self.cont3xt.view_editor_view_roles
                    };
                    for r in roles.iter_mut() { r.1 = true; }
                }
                KeyCode::Char('n') => {
                    let roles = if self.cont3xt.role_popup_for_edit {
                        &mut self.cont3xt.view_editor_edit_roles
                    } else {
                        &mut self.cont3xt.view_editor_view_roles
                    };
                    for r in roles.iter_mut() { r.1 = false; }
                }
                _ => {}
            }
            return;
        }

        // View editor (skip when on Settings link groups/overviews which have their own handlers)
        if self.cont3xt.view_editor_open && !(self.active_tab == Tab::Settings
            && matches!(self.cont3xt.settings_tab, C3SettingsTab::LinkGroups | C3SettingsTab::Overviews)) {
            // Integration filter mode within editor
            if self.cont3xt.view_editor_integration_filtering {
                match key.code {
                    KeyCode::Esc => {
                        self.cont3xt.view_editor_integration_filtering = false;
                    }
                    KeyCode::Enter => {
                        self.cont3xt.view_editor_integration_filtering = false;
                    }
                    KeyCode::Backspace => {
                        self.cont3xt.view_editor_integration_filter.pop();
                        self.cont3xt.view_editor_integration_selected = 0;
                    }
                    KeyCode::Char(c) => {
                        self.cont3xt.view_editor_integration_filter.push(c);
                        self.cont3xt.view_editor_integration_selected = 0;
                    }
                    _ => {}
                }
                return;
            }

            match self.cont3xt.view_editor_field {
                C3ViewEditorField::Name => {
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.next();
                            self.cont3xt.view_editor_integration_selected = 0;
                        }
                        KeyCode::BackTab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.prev();
                        }
                        KeyCode::Left => {
                            if self.cont3xt.view_editor_name_cursor > 0 {
                                self.cont3xt.view_editor_name_cursor -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if self.cont3xt.view_editor_name_cursor < self.cont3xt.view_editor_name.len() {
                                self.cont3xt.view_editor_name_cursor += 1;
                            }
                        }
                        KeyCode::Home => self.cont3xt.view_editor_name_cursor = 0,
                        KeyCode::End => self.cont3xt.view_editor_name_cursor = self.cont3xt.view_editor_name.len(),
                        KeyCode::Backspace => {
                            if self.cont3xt.view_editor_name_cursor > 0 {
                                self.cont3xt.view_editor_name_cursor -= 1;
                                self.cont3xt.view_editor_name.remove(self.cont3xt.view_editor_name_cursor);
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_save_view_editor();
                        }
                        KeyCode::Char(c) => {
                            self.cont3xt.view_editor_name.insert(self.cont3xt.view_editor_name_cursor, c);
                            self.cont3xt.view_editor_name_cursor += 1;
                        }
                        _ => {}
                    }
                }
                C3ViewEditorField::Integrations => {
                    let filtered = self.c3_view_editor_filtered_integrations();
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.prev();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.cont3xt.view_editor_integration_selected > 0 {
                                self.cont3xt.view_editor_integration_selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.cont3xt.view_editor_integration_selected + 1 < filtered.len() {
                                self.cont3xt.view_editor_integration_selected += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(&idx) = filtered.get(self.cont3xt.view_editor_integration_selected) {
                                self.cont3xt.view_editor_integrations[idx].1 = !self.cont3xt.view_editor_integrations[idx].1;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.cont3xt.view_editor_integration_filtering = true;
                        }
                        KeyCode::Char('a') => {
                            for i in &mut self.cont3xt.view_editor_integrations { i.1 = true; }
                        }
                        KeyCode::Char('n') => {
                            for i in &mut self.cont3xt.view_editor_integrations { i.1 = false; }
                        }
                        KeyCode::Char('!') => {
                            for i in &mut self.cont3xt.view_editor_integrations { i.1 = !i.1; }
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
                            self.cont3xt.view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.prev();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if self.cont3xt.all_roles.is_empty() {
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_roles().await;
                                    })
                                });
                                self.c3_rebuild_view_editor_roles();
                            }
                            self.cont3xt.role_popup_open = true;
                            self.cont3xt.role_popup_for_edit = false;
                            self.cont3xt.role_popup_selected = 0;
                            self.cont3xt.role_popup_filter.clear();
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
                            self.cont3xt.view_editor_open = false;
                        }
                        KeyCode::Tab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.next();
                        }
                        KeyCode::BackTab => {
                            self.cont3xt.view_editor_field = self.cont3xt.view_editor_field.prev();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if self.cont3xt.all_roles.is_empty() {
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_roles().await;
                                    })
                                });
                                self.c3_rebuild_view_editor_roles();
                            }
                            self.cont3xt.role_popup_open = true;
                            self.cont3xt.role_popup_for_edit = true;
                            self.cont3xt.role_popup_selected = 0;
                            self.cont3xt.role_popup_filter.clear();
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
        if self.cont3xt.int_editor_open {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.int_editor_open = false;
                }
                KeyCode::Up | KeyCode::Char('k') if !self.cont3xt.int_editor_values.is_empty() => {
                    if self.cont3xt.int_editor_selected > 0 {
                        self.cont3xt.int_editor_selected -= 1;
                        let val = &self.cont3xt.int_editor_values[self.cont3xt.int_editor_selected].1;
                        self.cont3xt.int_editor_cursor = val.len();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') if !self.cont3xt.int_editor_values.is_empty() => {
                    if self.cont3xt.int_editor_selected + 1 < self.cont3xt.int_editor_values.len() {
                        self.cont3xt.int_editor_selected += 1;
                        let val = &self.cont3xt.int_editor_values[self.cont3xt.int_editor_selected].1;
                        self.cont3xt.int_editor_cursor = val.len();
                    }
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Copy editor values back into settings
                    let idx = self.cont3xt.int_editor_idx;
                    if idx < self.cont3xt.int_settings.len() {
                        for (field_name, value, _, _, _, _) in &self.cont3xt.int_editor_values {
                            self.cont3xt.int_settings[idx].values.insert(field_name.clone(), value.clone());
                        }
                    }
                    self.cont3xt.int_editor_open = false;
                    // Save all settings
                    let payload = self.c3_build_int_settings_payload();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            match self.client.c3_put_integration_settings(&payload).await {
                                Ok(_) => {
                                    self.status_msg = "Integration settings saved".to_string();
                                    self.cont3xt.int_settings_dirty = false;
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
                    self.cont3xt.int_editor_show_password = !self.cont3xt.int_editor_show_password;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(entry) = self.cont3xt.int_editor_values.get_mut(self.cont3xt.int_editor_selected) {
                        if entry.3 { // is_boolean
                            entry.1 = if entry.1 == "true" { "false".to_string() } else { "true".to_string() };
                            self.cont3xt.int_settings_dirty = true;
                        }
                    }
                }
                _ => {
                    // Text input for non-boolean fields
                    if let Some(entry) = self.cont3xt.int_editor_values.get(self.cont3xt.int_editor_selected) {
                        if !entry.3 { // not boolean
                            let locked = self.cont3xt.int_editor_idx < self.cont3xt.int_settings.len() && self.cont3xt.int_settings[self.cont3xt.int_editor_idx].locked;
                            if !locked {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if let Some(entry) = self.cont3xt.int_editor_values.get_mut(self.cont3xt.int_editor_selected) {
                                            entry.1.insert(self.cont3xt.int_editor_cursor, c);
                                            self.cont3xt.int_editor_cursor += 1;
                                            self.cont3xt.int_settings_dirty = true;
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        if self.cont3xt.int_editor_cursor > 0 {
                                            if let Some(entry) = self.cont3xt.int_editor_values.get_mut(self.cont3xt.int_editor_selected) {
                                                entry.1.remove(self.cont3xt.int_editor_cursor - 1);
                                                self.cont3xt.int_editor_cursor -= 1;
                                                self.cont3xt.int_settings_dirty = true;
                                            }
                                        }
                                    }
                                    KeyCode::Delete => {
                                        if let Some(entry) = self.cont3xt.int_editor_values.get_mut(self.cont3xt.int_editor_selected) {
                                            if self.cont3xt.int_editor_cursor < entry.1.len() {
                                                entry.1.remove(self.cont3xt.int_editor_cursor);
                                                self.cont3xt.int_settings_dirty = true;
                                            }
                                        }
                                    }
                                    KeyCode::Left => {
                                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            // Word jump left
                                            if let Some(entry) = self.cont3xt.int_editor_values.get(self.cont3xt.int_editor_selected) {
                                                let bytes = entry.1.as_bytes();
                                                let mut pos = self.cont3xt.int_editor_cursor;
                                                while pos > 0 && bytes.get(pos - 1) == Some(&b' ') { pos -= 1; }
                                                while pos > 0 && bytes.get(pos - 1) != Some(&b' ') { pos -= 1; }
                                                self.cont3xt.int_editor_cursor = pos;
                                            }
                                        } else {
                                            self.cont3xt.int_editor_cursor = self.cont3xt.int_editor_cursor.saturating_sub(1);
                                        }
                                    }
                                    KeyCode::Right => {
                                        if let Some(entry) = self.cont3xt.int_editor_values.get(self.cont3xt.int_editor_selected) {
                                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                // Word jump right
                                                let bytes = entry.1.as_bytes();
                                                let len = entry.1.len();
                                                let mut pos = self.cont3xt.int_editor_cursor;
                                                while pos < len && bytes.get(pos) != Some(&b' ') { pos += 1; }
                                                while pos < len && bytes.get(pos) == Some(&b' ') { pos += 1; }
                                                self.cont3xt.int_editor_cursor = pos;
                                            } else if self.cont3xt.int_editor_cursor < entry.1.len() {
                                                self.cont3xt.int_editor_cursor += 1;
                                            }
                                        }
                                    }
                                    KeyCode::Home => {
                                        self.cont3xt.int_editor_cursor = 0;
                                    }
                                    KeyCode::End => {
                                        if let Some(entry) = self.cont3xt.int_editor_values.get(self.cont3xt.int_editor_selected) {
                                            self.cont3xt.int_editor_cursor = entry.1.len();
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

        // Link group backup filename prompt
        if self.cont3xt.backup_prompt.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.backup_prompt = None;
                }
                KeyCode::Enter => {
                    if let Some(filename) = self.cont3xt.backup_prompt.take() {
                        if filename.is_empty() {
                            self.status_msg = "No filename provided".to_string();
                        } else {
                            self.c3_save_backup(&filename, self.cont3xt.backup_kind);
                        }
                    }
                }
                _ => {
                    if let Some(ref mut f) = self.cont3xt.backup_prompt {
                        handle_text_input_key(key.code, f, &mut self.cont3xt.backup_cursor);
                    }
                }
            }
            return;
        }


        // Link group settings editor intercept
        if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
            if self.handle_c3_lg_settings_key(key) {
                return;
            }
        }

        // Overview settings editor intercept
        if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
            if self.handle_c3_ov_settings_key(key) {
                return;
            }
        }


        // Integration settings filter
        if self.cont3xt.int_settings_filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.cont3xt.int_settings_filtering = false;
                }
                KeyCode::Backspace => {
                    self.cont3xt.int_settings_filter.pop();
                    self.cont3xt.int_settings_selected = 0;
                    self.cont3xt.int_settings_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.cont3xt.int_settings_filter.push(c);
                    self.cont3xt.int_settings_selected = 0;
                    self.cont3xt.int_settings_table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        // Settings views filter
        if self.cont3xt.settings_views_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.cont3xt.settings_views_filtering = false;
                }
                KeyCode::Enter => {
                    self.cont3xt.settings_views_filtering = false;
                }
                KeyCode::Backspace => {
                    self.cont3xt.settings_views_filter.pop();
                    self.cont3xt.settings_views_selected = 0;
                    self.cont3xt.settings_views_table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    self.cont3xt.settings_views_filter.push(c);
                    self.cont3xt.settings_views_selected = 0;
                    self.cont3xt.settings_views_table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                if self.active_tab == Tab::C3Stats && self.cont3xt.stats_data.is_empty() {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_stats())
                    });
                }
                if self.active_tab == Tab::History && !self.cont3xt.history_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
                }
                if self.active_tab == Tab::Settings && !self.cont3xt.settings_views_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            self.c3_fetch_settings_views().await;
                            self.c3_fetch_roles().await;
                        })
                    });
                }
                if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations && !self.cont3xt.int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                    });
                }
            }
            KeyCode::BackTab => {
                self.prev_tab();
                if self.active_tab == Tab::C3Stats && self.cont3xt.stats_data.is_empty() {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_stats())
                    });
                }
                if self.active_tab == Tab::History && !self.cont3xt.history_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
                }
                if self.active_tab == Tab::Settings && !self.cont3xt.settings_views_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            self.c3_fetch_settings_views().await;
                            self.c3_fetch_roles().await;
                        })
                    });
                }
                if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations && !self.cont3xt.int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                    });
                }
            }
            KeyCode::Enter if self.active_tab == Tab::Search => {
                match self.cont3xt.focus {
                    Cont3xtFocus::Results => {
                        self.cont3xt.focus = Cont3xtFocus::Detail;
                        self.cont3xt.detail_scroll = 0;
                        self.cont3xt.detail_hscroll = 0;
                    }
                    Cont3xtFocus::Detail => {
                        self.cont3xt.focus = Cont3xtFocus::Results;
                    }
                }
            }
            KeyCode::Esc if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail => {
                self.cont3xt.focus = Cont3xtFocus::Results;
            }
            KeyCode::Char('/') if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail => {
                self.input_mode = InputMode::DetailFilter;
            }
            KeyCode::Char('/') | KeyCode::Char('E') if self.active_tab == Tab::Search => {
                self.enter_expression_mode();
            }
            KeyCode::Char('/') if self.active_tab == Tab::C3Stats => {
                self.cont3xt.stats_filtering = true;
            }
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('R') if self.active_tab == Tab::Search => {
                self.cont3xt.raw_view = !self.cont3xt.raw_view;
                self.cont3xt.detail_scroll = 0;
                self.cont3xt.detail_hscroll = 0;
            }
            KeyCode::Char('C') if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail => {
                // Show card popup for results, overview definition for indicators
                self.cont3xt.show_card_popup = !self.cont3xt.show_card_popup;
                self.cont3xt.card_popup_scroll = 0;
            }
            KeyCode::Char('o') if self.active_tab == Tab::Search => {
                // Overview selector — only when on an indicator
                if let Some(C3TreeItem::Indicator(itype, _)) = self.cont3xt.tree_order.get(self.cont3xt.selected) {
                    let itype_lower = itype.to_lowercase();
                    let mut matching: Vec<&crate::api::Cont3xtOverview> = self.cont3xt.overviews.iter()
                        .filter(|o| o.itype.to_lowercase() == itype_lower)
                        .collect();
                    matching.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    if !matching.is_empty() {
                        let current_id = self.cont3xt.selected_overviews.get(&itype_lower);
                        self.cont3xt.overview_popup_selected = matching.iter().position(|o| {
                            Some(&o.id) == current_id
                        }).or_else(|| matching.iter().position(|o| o.is_default))
                            .unwrap_or(0);
                        self.cont3xt.overview_popup_filter.clear();
                        self.cont3xt.overview_popup_filtering = false;
                        self.cont3xt.show_overview_popup = true;
                    } else {
                        self.status_msg = format!("No overviews for type '{}'", itype);
                    }
                }
            }
            KeyCode::Char('J') if self.active_tab == Tab::Search => {
                if self.cont3xt.results.is_empty() {
                    self.status_msg = "No results to save".to_string();
                } else {
                    let default_name = format!("{}.json", self.expression.replace(['/', '\\', ' '], "_"));
                    self.cont3xt.save_json_prompt = Some(default_name);
                }
            }
            KeyCode::Char('t') if self.active_tab == Tab::Search => {
                self.cont3xt.tags_edit = self.cont3xt.tags.join(", ");
                self.cont3xt.show_tags_popup = true;
            }
            KeyCode::Char('d') if self.active_tab == Tab::Search => {
                self.cont3xt.date_field = 0;
                self.cont3xt.show_date_popup = true;
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) && self.active_tab == Tab::Search => {
                self.cont3xt.no_cache = true;
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
                self.cont3xt.show_integration_popup = true;
                self.cont3xt.integration_popup_selected = 0;
                self.cont3xt.integration_popup_filter.clear();
                self.cont3xt.integration_popup_mode = IntegrationPopupMode::Integrations;
            }
            KeyCode::Char('I') | KeyCode::Char('v') if self.active_tab == Tab::Search => {
                self.cont3xt.show_integration_popup = true;
                self.cont3xt.view_selected = 0;
                self.cont3xt.integration_popup_mode = IntegrationPopupMode::Views;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_views())
                });
            }
            KeyCode::Char('l') if self.active_tab == Tab::Search => {
                if self.cont3xt.results.is_empty() {
                    self.status_msg = "Search for an indicator first".to_string();
                } else {
                    self.cont3xt.link_popup_selected = 0;
                    self.cont3xt.link_popup_filter.clear();
                    self.cont3xt.link_popup_filtering = false;
                    self.c3_build_link_flat();
                    self.cont3xt.show_link_popup = true;
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Results {
                    if let Some(name) = self.c3_current_integration_name() {
                        for i in (self.cont3xt.selected + 1)..self.cont3xt.tree_order.len() {
                            if let Some(idx) = self.cont3xt.tree_order[i].result_idx() {
                                if self.cont3xt.results.get(idx).map(|r| r.name.as_str()) == Some(&name) {
                                    self.cont3xt.selected = i;
                                    self.cont3xt.detail_scroll = 0;
                                    self.cont3xt.detail_hscroll = 0;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Results {
                    if let Some(name) = self.c3_current_integration_name() {
                        for i in (0..self.cont3xt.selected).rev() {
                            if let Some(idx) = self.cont3xt.tree_order[i].result_idx() {
                                if self.cont3xt.results.get(idx).map(|r| r.name.as_str()) == Some(&name) {
                                    self.cont3xt.selected = i;
                                    self.cont3xt.detail_scroll = 0;
                                    self.cont3xt.detail_hscroll = 0;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            // Jump to next top-level indicator
                            if let Some(next) = self.cont3xt.tree_roots.iter().find(|&&r| r > self.cont3xt.selected) {
                                self.cont3xt.selected = *next;
                            } else if !self.cont3xt.tree_order.is_empty() {
                                self.cont3xt.selected = self.cont3xt.tree_order.len() - 1;
                            }
                            self.cont3xt.detail_scroll = 0;
                            self.cont3xt.detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_add(self.visible_rows as u16);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    let data = self.c3_stats_current_data();
                    let filtered_len = data.iter()
                        .filter(|item| self.cont3xt.stats_filter.is_empty()
                            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                .to_lowercase().contains(&self.cont3xt.stats_filter.to_lowercase()))
                        .count();
                    if filtered_len > 0 {
                        self.cont3xt.stats_selected = (self.cont3xt.stats_selected + self.visible_rows).min(filtered_len - 1);
                        self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                    }
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.cont3xt.history_selected = (self.cont3xt.history_selected + self.visible_rows).min(len - 1);
                        self.cont3xt.history_table_state.select(Some(self.cont3xt.history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views {
                    let filtered = self.c3_settings_filtered_views();
                    if !filtered.is_empty() {
                        self.cont3xt.settings_views_selected = (self.cont3xt.settings_views_selected + self.visible_rows).min(filtered.len() - 1);
                        self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if !filtered.is_empty() {
                        self.cont3xt.int_settings_selected = (self.cont3xt.int_settings_selected + self.visible_rows).min(filtered.len() - 1);
                        self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if !filtered.is_empty() {
                        self.cont3xt.lg_selected = (self.cont3xt.lg_selected + self.visible_rows).min(filtered.len() - 1);
                        self.cont3xt.lg_table_state.select(Some(self.cont3xt.lg_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    let filtered = self.c3_ov_filtered_list();
                    if !filtered.is_empty() {
                        self.cont3xt.ov_selected = (self.cont3xt.ov_selected + self.visible_rows).min(filtered.len() - 1);
                        self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                    }
                }
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            // Jump to previous top-level indicator
                            if let Some(prev) = self.cont3xt.tree_roots.iter().rev().find(|&&r| r < self.cont3xt.selected) {
                                self.cont3xt.selected = *prev;
                            } else {
                                self.cont3xt.selected = 0;
                            }
                            self.cont3xt.detail_scroll = 0;
                            self.cont3xt.detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_sub(self.visible_rows as u16);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    self.cont3xt.stats_selected = self.cont3xt.stats_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                } else if self.active_tab == Tab::History {
                    self.cont3xt.history_selected = self.cont3xt.history_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.history_table_state.select(Some(self.cont3xt.history_selected));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views {
                    self.cont3xt.settings_views_selected = self.cont3xt.settings_views_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    self.cont3xt.int_settings_selected = self.cont3xt.int_settings_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    self.cont3xt.lg_selected = self.cont3xt.lg_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.lg_table_state.select(Some(self.cont3xt.lg_selected));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    self.cont3xt.ov_selected = self.cont3xt.ov_selected.saturating_sub(self.visible_rows);
                    self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            if !self.cont3xt.tree_order.is_empty() {
                                self.cont3xt.selected = (self.cont3xt.selected + 1).min(self.cont3xt.tree_order.len() - 1);
                                self.cont3xt.detail_scroll = 0;
                                self.cont3xt.detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_add(1);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    let data = self.c3_stats_current_data();
                    let filtered_len = data.iter()
                        .filter(|item| self.cont3xt.stats_filter.is_empty()
                            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                .to_lowercase().contains(&self.cont3xt.stats_filter.to_lowercase()))
                        .count();
                    if filtered_len > 0 {
                        self.cont3xt.stats_selected = (self.cont3xt.stats_selected + 1).min(filtered_len - 1);
                        self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                    }
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.cont3xt.history_selected = (self.cont3xt.history_selected + 1).min(len - 1);
                        self.cont3xt.history_table_state.select(Some(self.cont3xt.history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views {
                    let filtered = self.c3_settings_filtered_views();
                    if self.cont3xt.settings_views_selected + 1 < filtered.len() {
                        self.cont3xt.settings_views_selected += 1;
                        self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if self.cont3xt.int_settings_selected + 1 < filtered.len() {
                        self.cont3xt.int_settings_selected += 1;
                        self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if self.cont3xt.lg_selected + 1 < filtered.len() {
                        self.cont3xt.lg_selected += 1;
                        self.cont3xt.lg_table_state.select(Some(self.cont3xt.lg_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    let filtered = self.c3_ov_filtered_list();
                    if self.cont3xt.ov_selected + 1 < filtered.len() {
                        self.cont3xt.ov_selected += 1;
                        self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            self.cont3xt.selected = self.cont3xt.selected.saturating_sub(1);
                            self.cont3xt.detail_scroll = 0;
                            self.cont3xt.detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_sub(1);
                        }
                    }
                } else if self.active_tab == Tab::C3Stats {
                    self.cont3xt.stats_selected = self.cont3xt.stats_selected.saturating_sub(1);
                    self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                } else if self.active_tab == Tab::History {
                    self.cont3xt.history_selected = self.cont3xt.history_selected.saturating_sub(1);
                    self.cont3xt.history_table_state.select(Some(self.cont3xt.history_selected));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views {
                    if self.cont3xt.settings_views_selected > 0 {
                        self.cont3xt.settings_views_selected -= 1;
                        self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    if self.cont3xt.int_settings_selected > 0 {
                        self.cont3xt.int_settings_selected -= 1;
                        self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    if self.cont3xt.lg_selected > 0 {
                        self.cont3xt.lg_selected -= 1;
                        self.cont3xt.lg_table_state.select(Some(self.cont3xt.lg_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    if self.cont3xt.ov_selected > 0 {
                        self.cont3xt.ov_selected -= 1;
                        self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                    }
                }
            }
            KeyCode::PageDown => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_add(self.visible_rows as u16);
                }
            }
            KeyCode::PageUp => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_scroll = self.cont3xt.detail_scroll.saturating_sub(self.visible_rows as u16);
                }
            }
            KeyCode::Home => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_scroll = 0;
                    self.cont3xt.detail_hscroll = 0;
                } else if self.active_tab == Tab::History {
                    self.cont3xt.history_selected = 0;
                    self.cont3xt.history_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    self.cont3xt.int_settings_selected = 0;
                    self.cont3xt.int_settings_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    self.cont3xt.lg_selected = 0;
                    self.cont3xt.lg_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    self.cont3xt.ov_selected = 0;
                    self.cont3xt.ov_table_state.select(Some(0));
                } else if self.active_tab == Tab::Settings {
                    self.cont3xt.settings_views_selected = 0;
                    self.cont3xt.settings_views_table_state.select(Some(0));
                }
            }
            KeyCode::End => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_scroll = u16::MAX;
                } else if self.active_tab == Tab::History {
                    let len = self.c3_history_filtered_len();
                    if len > 0 {
                        self.cont3xt.history_selected = len - 1;
                        self.cont3xt.history_table_state.select(Some(self.cont3xt.history_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations {
                    let filtered = self.c3_int_settings_filtered();
                    if !filtered.is_empty() {
                        self.cont3xt.int_settings_selected = filtered.len() - 1;
                        self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
                    let filtered = self.c3_lg_filtered_groups();
                    if !filtered.is_empty() {
                        self.cont3xt.lg_selected = filtered.len() - 1;
                        self.cont3xt.lg_table_state.select(Some(self.cont3xt.lg_selected));
                    }
                } else if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews {
                    let filtered = self.c3_ov_filtered_list();
                    if !filtered.is_empty() {
                        self.cont3xt.ov_selected = filtered.len() - 1;
                        self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                    }
                } else if self.active_tab == Tab::Settings {
                    let filtered = self.c3_settings_filtered_views();
                    if !filtered.is_empty() {
                        self.cont3xt.settings_views_selected = filtered.len() - 1;
                        self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
                    }
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_hscroll = self.cont3xt.detail_hscroll.saturating_sub(20);
                }
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.active_tab == Tab::Search && self.cont3xt.focus == Cont3xtFocus::Detail {
                    self.cont3xt.detail_hscroll = self.cont3xt.detail_hscroll.saturating_add(20);
                }
            }
            KeyCode::Left => {
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            self.cont3xt.selected = 0;
                            self.cont3xt.detail_scroll = 0;
                            self.cont3xt.detail_hscroll = 0;
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_hscroll = self.cont3xt.detail_hscroll.saturating_sub(4);
                        }
                    }
                } else if self.active_tab == Tab::History && self.cont3xt.history_page > 1 {
                    self.cont3xt.history_page -= 1;
                    self.cont3xt.history_selected = 0;
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                    });
                }
            }
            KeyCode::Right => {
                if self.active_tab == Tab::Search {
                    match self.cont3xt.focus {
                        Cont3xtFocus::Results => {
                            if !self.cont3xt.tree_order.is_empty() {
                                self.cont3xt.selected = self.cont3xt.tree_order.len() - 1;
                                self.cont3xt.detail_scroll = 0;
                                self.cont3xt.detail_hscroll = 0;
                            }
                        }
                        Cont3xtFocus::Detail => {
                            self.cont3xt.detail_hscroll = self.cont3xt.detail_hscroll.saturating_add(4);
                        }
                    }
                } else if self.active_tab == Tab::History {
                    let total_pages = (self.cont3xt.history_total + 99) / 100;
                    if self.cont3xt.history_page < total_pages {
                        self.cont3xt.history_page += 1;
                        self.cont3xt.history_selected = 0;
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                        });
                    }
                }
            }
            // C3 Stats tab keys
            KeyCode::Char('1') if self.active_tab == Tab::C3Stats => {
                if self.cont3xt.stats_tab != C3StatsTab::Integrations {
                    self.cont3xt.stats_tab = C3StatsTab::Integrations;
                    self.cont3xt.stats_selected = 0;
                    self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                }
            }
            KeyCode::Char('2') if self.active_tab == Tab::C3Stats => {
                if self.cont3xt.stats_tab != C3StatsTab::ITypes {
                    self.cont3xt.stats_tab = C3StatsTab::ITypes;
                    self.cont3xt.stats_selected = 0;
                    self.cont3xt.stats_table_state.select(Some(self.cont3xt.stats_selected));
                }
            }
            KeyCode::Char('s') if self.active_tab == Tab::C3Stats => {
                let ncols = self.cont3xt.stats_tab.columns().len();
                self.cont3xt.stats_sort_col = (self.cont3xt.stats_sort_col + 1) % ncols;
            }
            KeyCode::Char('S') if self.active_tab == Tab::C3Stats => {
                self.cont3xt.stats_sort_desc = !self.cont3xt.stats_sort_desc;
            }
            // History tab keys
            KeyCode::Char('/') if self.active_tab == Tab::History => {
                self.cont3xt.history_filtering = true;
            }
            KeyCode::Char('s') if self.active_tab == Tab::History => {
                let sortable_cols: Vec<usize> = C3_HISTORY_COLUMNS.iter().enumerate()
                    .filter(|(_, c)| c.3).map(|(i, _)| i).collect();
                if let Some(pos) = sortable_cols.iter().position(|&i| i == self.cont3xt.history_sort_col) {
                    self.cont3xt.history_sort_col = sortable_cols[(pos + 1) % sortable_cols.len()];
                } else if !sortable_cols.is_empty() {
                    self.cont3xt.history_sort_col = sortable_cols[0];
                }
                self.cont3xt.history_page = 1;
                self.cont3xt.history_selected = 0;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_history())
                });
            }
            KeyCode::Char('S') if self.active_tab == Tab::History => {
                self.cont3xt.history_sort_desc = !self.cont3xt.history_sort_desc;
                self.cont3xt.history_page = 1;
                self.cont3xt.history_selected = 0;
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
                if let Some(item) = self.cont3xt.history_data.get(self.cont3xt.history_selected) {
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
                if let Some(item) = self.cont3xt.history_data.get(self.cont3xt.history_selected) {
                    let indicator = item.get("indicator").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if !indicator.is_empty() {
                        self.expression = indicator;
                        self.active_tab = Tab::Search;
                        self.cont3xt.focus = Cont3xtFocus::Results;
                        self.c3_request_search();
                    }
                }
            }
            // Settings tab keys
            KeyCode::Char('1') if self.active_tab == Tab::Settings => {
                self.cont3xt.settings_tab = C3SettingsTab::Views;
            }
            KeyCode::Char('2') if self.active_tab == Tab::Settings => {
                self.cont3xt.settings_tab = C3SettingsTab::Integrations;
                if !self.cont3xt.int_settings_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                    });
                }
            }
            KeyCode::Char('3') if self.active_tab == Tab::Settings => {
                self.cont3xt.settings_tab = C3SettingsTab::LinkGroups;
                if !self.cont3xt.lg_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(self.c3_fetch_link_groups_settings())
                    });
                }
            }
            KeyCode::Char('4') if self.active_tab == Tab::Settings => {
                self.cont3xt.settings_tab = C3SettingsTab::Overviews;
                if !self.cont3xt.ov_loaded {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            self.c3_fetch_overviews().await;
                            self.cont3xt.ov_loaded = true;
                        })
                    });
                    if !self.cont3xt.ov_list.is_empty() {
                        self.cont3xt.ov_selected = 0;
                        self.cont3xt.ov_table_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.c3_fetch_settings_views().await;
                        self.c3_fetch_roles().await;
                    })
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                self.cont3xt.settings_views_sort = (self.cont3xt.settings_views_sort + 1) % 5;
                self.cont3xt.settings_views_selected = 0;
                self.cont3xt.settings_views_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                self.cont3xt.settings_views_sort_desc = !self.cont3xt.settings_views_sort_desc;
                self.cont3xt.settings_views_selected = 0;
                self.cont3xt.settings_views_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                self.cont3xt.settings_views_filtering = true;
            }
            KeyCode::Char('n') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                self.c3_open_new_view_editor();
            }
            KeyCode::Enter | KeyCode::Char('e') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                let filtered = self.c3_settings_filtered_views();
                if let Some(&idx) = filtered.get(self.cont3xt.settings_views_selected) {
                    let view = self.cont3xt.settings_views[idx].clone();
                    self.c3_open_edit_view_editor(&view);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                let filtered = self.c3_settings_filtered_views();
                if let Some(&idx) = filtered.get(self.cont3xt.settings_views_selected) {
                    let view = &self.cont3xt.settings_views[idx];
                    if view.editable {
                        self.cont3xt.settings_confirm = Some((
                            format!("delete_view:{}", view.id),
                            format!("Delete view \"{}\"?", view.name),
                        ));
                    } else {
                        self.status_msg = "View is not editable".to_string();
                    }
                }
            }
            // Integration settings keys
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_integration_settings())
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cont3xt.int_settings_sort = (self.cont3xt.int_settings_sort + 1) % 2;
                self.cont3xt.int_settings_selected = 0;
                self.cont3xt.int_settings_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                self.cont3xt.int_settings_sort_desc = !self.cont3xt.int_settings_sort_desc;
                self.cont3xt.int_settings_selected = 0;
                self.cont3xt.int_settings_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                self.cont3xt.int_settings_filtering = true;
            }
            KeyCode::Char('d') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                let filtered = self.c3_int_settings_filtered();
                if let Some(&idx) = filtered.get(self.cont3xt.int_settings_selected) {
                    self.cont3xt.int_settings[idx].disabled = !self.cont3xt.int_settings[idx].disabled;
                    self.cont3xt.int_settings_dirty = true;
                }
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations && key.modifiers.contains(KeyModifiers::CONTROL) => {
                let payload = self.c3_build_int_settings_payload();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match self.client.c3_put_integration_settings(&payload).await {
                            Ok(_) => {
                                self.status_msg = "Integration settings saved".to_string();
                                self.cont3xt.int_settings_dirty = false;
                                self.c3_fetch_integration_settings().await;
                            }
                            Err(e) => {
                                self.status_msg = format!("Error saving settings: {e}");
                            }
                        }
                    })
                });
            }
            KeyCode::Enter | KeyCode::Char('e') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                let filtered = self.c3_int_settings_filtered();
                if let Some(&idx) = filtered.get(self.cont3xt.int_settings_selected) {
                    let int = &self.cont3xt.int_settings[idx];
                    let values: Vec<(String, String, bool, bool, bool, String)> = int.fields.iter().map(|f| {
                        let val = int.values.get(&f.name).cloned().unwrap_or_default();
                        (f.name.clone(), val, f.password, f.is_boolean, f.required, f.help.clone())
                    }).collect();
                    self.cont3xt.int_editor_open = true;
                    self.cont3xt.int_editor_idx = idx;
                    self.cont3xt.int_editor_values = values;
                    self.cont3xt.int_editor_selected = 0;
                    self.cont3xt.int_editor_cursor = self.cont3xt.int_editor_values.first().map(|v| v.1.len()).unwrap_or(0);
                    self.cont3xt.int_editor_show_password = false;
                }
            }
            // Link group settings keys (GroupList level)
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_fetch_link_groups_settings())
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                self.cont3xt.lg_sort_col = (self.cont3xt.lg_sort_col + 1) % 4;
                self.cont3xt.lg_selected = 0;
                self.cont3xt.lg_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                self.cont3xt.lg_sort_desc = !self.cont3xt.lg_sort_desc;
                self.cont3xt.lg_selected = 0;
                self.cont3xt.lg_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                self.cont3xt.lg_filtering = true;
            }
            KeyCode::Char('n') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
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
            KeyCode::Char('e') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                let filtered = self.c3_lg_filtered_groups();
                if let Some(&idx) = filtered.get(self.cont3xt.lg_selected) {
                    let group = &self.cont3xt.lg_groups[idx];
                    self.cont3xt.lg_group_editor_idx = idx;
                    self.cont3xt.lg_group_editor_name = group.name.clone();
                    self.cont3xt.lg_group_editor_cursor = group.name.len();
                    self.cont3xt.lg_group_editor_view_roles = group.view_roles.clone();
                    self.cont3xt.lg_group_editor_edit_roles = group.edit_roles.clone();
                    self.cont3xt.lg_group_editor_field = C3GroupEditorField::Name;
                    self.cont3xt.lg_level = C3LinkGroupLevel::GroupEditor;
                }
            }
            KeyCode::Enter if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                let filtered = self.c3_lg_filtered_groups();
                if let Some(&idx) = filtered.get(self.cont3xt.lg_selected) {
                    self.cont3xt.lg_editing_group_idx = idx;
                    self.cont3xt.lg_links_selected = 0;
                    self.cont3xt.lg_links_table_state.select(Some(0));
                    self.cont3xt.lg_links_filter.clear();
                    self.cont3xt.lg_links_filtering = false;
                    self.cont3xt.lg_level = C3LinkGroupLevel::LinkList;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                let filtered = self.c3_lg_filtered_groups();
                if let Some(&idx) = filtered.get(self.cont3xt.lg_selected) {
                    let group = &self.cont3xt.lg_groups[idx];
                    if group.editable {
                        self.cont3xt.settings_confirm = Some((
                            format!("delete_link_group:{}", group.id),
                            format!("Delete link group \"{}\"?", group.name),
                        ));
                    } else {
                        self.status_msg = "Link group is not editable".to_string();
                    }
                }
            }
            KeyCode::Char('B') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::LinkGroups => {
                if self.cont3xt.lg_groups.is_empty() {
                    self.status_msg = "No link groups to backup".to_string();
                } else {
                    self.cont3xt.backup_kind = C3BackupKind::LinkGroupsAll;
                    self.cont3xt.backup_prompt = Some("linkgroups-backup.json".to_string());
                    self.cont3xt.backup_cursor = "linkgroups-backup.json".len();
                }
            }
            // Overview settings keys (List level)
            KeyCode::Char('r') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.c3_fetch_overviews().await;
                        self.cont3xt.ov_loaded = true;
                    })
                });
            }
            KeyCode::Char('s') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                self.cont3xt.ov_sort_col = (self.cont3xt.ov_sort_col + 1) % 5;
                self.cont3xt.ov_selected = 0;
                self.cont3xt.ov_table_state.select(Some(0));
            }
            KeyCode::Char('S') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                self.cont3xt.ov_sort_desc = !self.cont3xt.ov_sort_desc;
                self.cont3xt.ov_selected = 0;
                self.cont3xt.ov_table_state.select(Some(0));
            }
            KeyCode::Char('/') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                self.cont3xt.ov_filtering = true;
            }
            KeyCode::Char('n') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(self.c3_ov_create())
                });
            }
            KeyCode::Char('e') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                let filtered = self.c3_ov_filtered_list();
                if let Some(&idx) = filtered.get(self.cont3xt.ov_selected) {
                    let ov = &self.cont3xt.ov_list[idx];
                    self.cont3xt.ov_editor_idx = idx;
                    self.cont3xt.ov_editor_name = ov.name.clone();
                    self.cont3xt.ov_editor_title = ov.title.clone();
                    self.cont3xt.ov_editor_itype = ov.itype.clone();
                    self.cont3xt.ov_editor_view_roles = ov.view_roles.clone();
                    self.cont3xt.ov_editor_edit_roles = ov.edit_roles.clone();
                    self.cont3xt.ov_editor_field = C3OverviewEditorField::Name;
                    self.cont3xt.ov_editor_cursor = ov.name.len();
                    self.cont3xt.ov_level = C3OverviewLevel::Editor;
                }
            }
            KeyCode::Enter if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                let filtered = self.c3_ov_filtered_list();
                if let Some(&idx) = filtered.get(self.cont3xt.ov_selected) {
                    let ov = &self.cont3xt.ov_list[idx];
                    self.cont3xt.ov_editor_idx = idx;
                    self.cont3xt.ov_editor_name = ov.name.clone();
                    self.cont3xt.ov_editor_title = ov.title.clone();
                    self.cont3xt.ov_editor_itype = ov.itype.clone();
                    self.cont3xt.ov_editor_view_roles = ov.view_roles.clone();
                    self.cont3xt.ov_editor_edit_roles = ov.edit_roles.clone();
                    self.cont3xt.ov_editor_field = C3OverviewEditorField::Name;
                    self.cont3xt.ov_editor_cursor = ov.name.len();
                    // Go directly to field list
                    self.cont3xt.ov_fields_selected = 0;
                    self.cont3xt.ov_fields_table_state.select(Some(0));
                    self.cont3xt.ov_fields_filter.clear();
                    self.cont3xt.ov_fields_filtering = false;
                    self.cont3xt.ov_level = C3OverviewLevel::FieldList;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                let filtered = self.c3_ov_filtered_list();
                if let Some(&idx) = filtered.get(self.cont3xt.ov_selected) {
                    let ov = &self.cont3xt.ov_list[idx];
                    if ov.editable {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(self.c3_ov_delete())
                        });
                    } else {
                        self.status_msg = "Overview is not editable".to_string();
                    }
                }
            }
            KeyCode::Char('B') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Overviews => {
                if self.cont3xt.ov_list.is_empty() {
                    self.status_msg = "No overviews to backup".to_string();
                } else {
                    self.cont3xt.backup_kind = C3BackupKind::OverviewsAll;
                    self.cont3xt.backup_prompt = Some("overviews-backup.json".to_string());
                    self.cont3xt.backup_cursor = "overviews-backup.json".len();
                }
            }
            KeyCode::Char('B') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Integrations => {
                if self.cont3xt.int_settings.is_empty() {
                    self.status_msg = "No integration settings to backup".to_string();
                } else {
                    self.cont3xt.backup_kind = C3BackupKind::Integrations;
                    self.cont3xt.backup_prompt = Some("integrations-backup.json".to_string());
                    self.cont3xt.backup_cursor = "integrations-backup.json".len();
                }
            }
            KeyCode::Char('B') if self.active_tab == Tab::Settings && self.cont3xt.settings_tab == C3SettingsTab::Views => {
                if self.cont3xt.settings_views.is_empty() {
                    self.status_msg = "No views to backup".to_string();
                } else {
                    self.cont3xt.backup_kind = C3BackupKind::Views;
                    self.cont3xt.backup_prompt = Some("views-backup.json".to_string());
                    self.cont3xt.backup_cursor = "views-backup.json".len();
                }
            }
            _ => {}
        }
    }

}
