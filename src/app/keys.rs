use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    pub async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.confirm_dialog.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let dialog = self.confirm_dialog.take().unwrap();
                    self.handle_confirm(dialog.action).await;
                }
                _ => {
                    self.confirm_dialog = None;
                }
            }
            return;
        }
        if self.show_debug {
            let total = self.http_log.lock().unwrap().len();
            match key.code {
                KeyCode::Esc | KeyCode::Char('D') | KeyCode::Char('q') => {
                    if self.debug_expanded {
                        self.debug_expanded = false;
                    } else {
                        self.show_debug = false;
                    }
                }
                KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if self.debug_expanded {
                        self.debug_scroll = self.debug_scroll.saturating_sub(10);
                    } else {
                        self.debug_selected = self.debug_selected.saturating_sub(10);
                    }
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if self.debug_expanded {
                        self.debug_scroll += 10;
                    } else if total > 0 {
                        self.debug_selected = (self.debug_selected + 10).min(total - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.debug_expanded {
                        self.debug_scroll = self.debug_scroll.saturating_sub(1);
                    } else {
                        self.debug_selected = self.debug_selected.saturating_sub(1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.debug_expanded {
                        self.debug_scroll += 1;
                    } else if total > 0 {
                        self.debug_selected = (self.debug_selected + 1).min(total - 1);
                    }
                }
                KeyCode::Home => {
                    if self.debug_expanded {
                        self.debug_scroll = 0;
                    } else {
                        self.debug_selected = 0;
                    }
                }
                KeyCode::End => {
                    if self.debug_expanded {
                        // scroll handled by render
                    } else if total > 0 {
                        self.debug_selected = total - 1;
                    }
                }
                KeyCode::Enter => {
                    if self.debug_expanded {
                        self.debug_expanded = false;
                    } else {
                        self.debug_expanded = true;
                        self.debug_scroll = 0;
                    }
                }
                _ => {}
            }
            return;
        }
        // Users tab: handle early to avoid viewer-specific intercepts
        if self.active_tab == Tab::Users && self.us_role_popup_open {
            self.handle_users_role_popup_key(key).await;
            return;
        }
        if self.active_tab == Tab::Users && self.us_editing {
            self.handle_users_editor_key(key).await;
            return;
        }
        if self.active_tab == Tab::Users && self.input_mode == InputMode::Normal && !self.us_editing {
            self.handle_users_key(key).await;
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
        if self.viewer.detail_action_menu.is_some() {
            self.handle_detail_action_key(key).await;
            return;
        }
        if self.input_mode == InputMode::FieldSelector {
            self.handle_field_selector_key(key).await;
            return;
        }
        if self.viewer.packets_view.is_some() {
            self.handle_packets_key(key);
            return;
        }
        if self.input_mode == InputMode::Expression {
            self.handle_expression_key(key).await;
            return;
        }
        if self.viewer.show_column_editor {
            self.handle_column_editor_key(key).await;
            return;
        }
        if self.viewer.show_layout_popup {
            self.handle_layout_popup_key(key).await;
            return;
        }
        if self.viewer.stats_show_column_editor {
            self.handle_stats_column_editor_key(key).await;
            return;
        }
        if self.viewer.stats_show_layout_popup {
            self.handle_stats_layout_popup_key(key).await;
            return;
        }
        if self.viewer.files_show_column_editor {
            self.handle_files_column_editor_key(key).await;
            return;
        }
        if self.viewer.files_show_layout_popup {
            self.handle_files_layout_popup_key(key).await;
            return;
        }
        if self.viewer.show_view_popup {
            self.handle_view_popup_key(key).await;
            return;
        }
        if key.code == KeyCode::Char('D') {
            self.show_debug = true;
            self.debug_selected = 0;
            self.debug_expanded = false;
            return;
        }
        // Ctrl+P: return to Parliament from Viewer, Cont3xt, or WISE mode
        if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL)
            && (self.app_mode == crate::app::AppMode::Viewer || self.app_mode == crate::app::AppMode::Cont3xt || self.app_mode == crate::app::AppMode::Wise)
            && self.parliament.saved_client.is_some()
        {
            self.pl_return_to_parliament().await;
            return;
        }
        // Users tab is handled the same way regardless of app mode
        if self.active_tab == Tab::Users && self.us_role_popup_open {
            self.handle_users_role_popup_key(key).await;
            return;
        }
        if self.active_tab == Tab::Users && !self.us_editing {
            // Skip if expression mode — that's handled above (but we already returned if so)
            self.handle_users_key(key).await;
            return;
        }
        if self.active_tab == Tab::Users && self.us_editing {
            self.handle_users_editor_key(key).await;
            return;
        }
        match self.app_mode {
            crate::app::AppMode::Viewer => {
                match self.active_tab {
                    Tab::Stats => {
                        match self.viewer.stats_view {
                            StatsView::List => self.handle_stats_key(key).await,
                            StatsView::Detail => self.handle_stats_detail_key(key),
                        }
                    }
                    Tab::Files => {
                        match self.viewer.files_view {
                            StatsView::List => self.handle_files_key(key).await,
                            StatsView::Detail => self.handle_files_detail_key(key),
                        }
                    }
                    Tab::Arkime => self.handle_arkime_key(key).await,
                    _ => {
                        match self.viewer.session_view {
                            SessionView::List => self.handle_list_key(key).await,
                            SessionView::Detail => self.handle_detail_key(key).await,
                        }
                    }
                }
            }
            crate::app::AppMode::Cont3xt => {
                self.handle_cont3xt_key(key).await;
            }
            crate::app::AppMode::Parliament => {
                self.handle_parliament_key(key).await;
            }
            crate::app::AppMode::Wise => {
                self.handle_wise_key(key).await;
            }
        }
    }

    async fn handle_expression_key(&mut self, key: KeyEvent) {
        let is_stats = self.active_tab == Tab::Stats;
        let is_files = self.active_tab == Tab::Files;
        let is_users = self.active_tab == Tab::Users;
        let is_pl_issues = self.app_mode == crate::app::AppMode::Parliament && self.active_tab == Tab::Issues;
        let is_ws_stats = self.app_mode == crate::app::AppMode::Wise && self.active_tab == Tab::WsStats;
        let is_ws_query = self.app_mode == crate::app::AppMode::Wise && self.active_tab == Tab::WsQuery;
        let edit = if is_pl_issues {
            &mut self.parliament.issues_filter_edit
        } else if is_ws_stats {
            &mut self.wise.stats_filter_edit
        } else if is_ws_query {
            &mut self.wise.query_value_edit
        } else if is_stats {
            &mut self.viewer.stats_filter_edit
        } else if is_files {
            &mut self.viewer.files_filter_edit
        } else {
            &mut self.expression_edit
        };
        match key.code {
            KeyCode::Enter => {
                if is_users {
                    self.us_filter = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.us_page_start = 0;
                    self.us_selected = 0;
                    self.us_fetch_users().await;
                } else if is_pl_issues {
                    self.parliament.issues_filter = self.parliament.issues_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.parliament.issues_selected = 0;
                } else if is_ws_stats {
                    self.wise.stats_filter = self.wise.stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.wise.stats_selected = 0;
                    self.ws_fetch_stats().await;
                } else if is_ws_query {
                    self.wise.query_value = self.wise.query_value_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.ws_run_query().await;
                } else if is_stats {
                    self.viewer.stats_filter = self.viewer.stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.vr_fetch_stats().await;
                } else if is_files {
                    self.viewer.files_filter = self.viewer.files_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.viewer.files_page_start = 0;
                    self.viewer.files_selected = 0;
                    self.vr_fetch_files().await;
                } else {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.viewer.page_start = 0;
                    match self.app_mode {
                        crate::app::AppMode::Cont3xt => {
                            self.c3_request_search();
                        }
                        _ => {
                            if self.active_tab == Tab::Arkime {
                                self.vr_request_summary_fetch();
                            } else {
                                self.vr_fetch_sessions().await;
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                if is_users {
                    self.expression_edit = self.us_filter.clone();
                } else if is_pl_issues {
                    self.parliament.issues_filter_edit = self.parliament.issues_filter.clone();
                } else if is_ws_stats {
                    self.wise.stats_filter_edit = self.wise.stats_filter.clone();
                } else if is_ws_query {
                    self.wise.query_value_edit = self.wise.query_value.clone();
                } else if is_stats {
                    self.viewer.stats_filter_edit = self.viewer.stats_filter.clone();
                } else if is_files {
                    self.viewer.files_filter_edit = self.viewer.files_filter.clone();
                } else {
                    self.expression_edit = self.expression.clone();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Jump to start of previous word
                    let mut pos = self.expression_cursor;
                    let bytes = edit.as_bytes();
                    // Skip whitespace/punctuation going left
                    while pos > 0 && !bytes[pos - 1].is_ascii_alphanumeric() {
                        pos -= 1;
                    }
                    // Skip word chars
                    while pos > 0 && bytes[pos - 1].is_ascii_alphanumeric() {
                        pos -= 1;
                    }
                    self.expression_cursor = pos;
                } else if self.expression_cursor > 0 {
                    self.expression_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Jump to end of next word
                    let mut pos = self.expression_cursor;
                    let bytes = edit.as_bytes();
                    let len = bytes.len();
                    // Skip whitespace/punctuation going right
                    while pos < len && !bytes[pos].is_ascii_alphanumeric() {
                        pos += 1;
                    }
                    // Skip word chars
                    while pos < len && bytes[pos].is_ascii_alphanumeric() {
                        pos += 1;
                    }
                    self.expression_cursor = pos;
                } else if self.expression_cursor < edit.len() {
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
        // Users filter: live search as-you-type (only on text-changing keys)
        if is_users {
            match key.code {
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                    self.us_filter = self.expression_edit.clone();
                    self.us_page_start = 0;
                    self.us_selected = 0;
                    self.us_fetch_users().await;
                }
                _ => {}
            }
        }
    }

}
