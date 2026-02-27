use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    pub async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
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
        if self.vr_detail_action_menu.is_some() {
            self.handle_detail_action_key(key).await;
            return;
        }
        if self.input_mode == InputMode::FieldSelector {
            self.handle_field_selector_key(key).await;
            return;
        }
        if self.vr_packets_view.is_some() {
            self.handle_packets_key(key);
            return;
        }
        if self.input_mode == InputMode::Expression {
            self.handle_expression_key(key).await;
            return;
        }
        if self.vr_show_column_editor {
            self.handle_column_editor_key(key).await;
            return;
        }
        if self.vr_show_layout_popup {
            self.handle_layout_popup_key(key).await;
            return;
        }
        if self.vr_show_view_popup {
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
            && self.pl_saved_client.is_some()
        {
            self.pl_return_to_parliament().await;
            return;
        }
        match self.app_mode {
            crate::app::AppMode::Viewer => {
                match self.active_tab {
                    Tab::Stats => {
                        match self.vr_stats_view {
                            StatsView::List => self.handle_stats_key(key).await,
                            StatsView::Detail => self.handle_stats_detail_key(key),
                        }
                    }
                    Tab::Arkime => self.handle_arkime_key(key).await,
                    _ => {
                        match self.vr_session_view {
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
        let is_pl_issues = self.app_mode == crate::app::AppMode::Parliament && self.active_tab == Tab::Issues;
        let is_ws_stats = self.app_mode == crate::app::AppMode::Wise && self.active_tab == Tab::WsStats;
        let is_ws_query = self.app_mode == crate::app::AppMode::Wise && self.active_tab == Tab::WsQuery;
        let edit = if is_pl_issues {
            &mut self.pl_issues_filter_edit
        } else if is_ws_stats {
            &mut self.ws_stats_filter_edit
        } else if is_ws_query {
            &mut self.ws_query_value_edit
        } else if is_stats {
            &mut self.vr_stats_filter_edit
        } else {
            &mut self.expression_edit
        };
        match key.code {
            KeyCode::Enter => {
                if is_pl_issues {
                    self.pl_issues_filter = self.pl_issues_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.pl_issues_selected = 0;
                } else if is_ws_stats {
                    self.ws_stats_filter = self.ws_stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.ws_stats_selected = 0;
                    self.ws_fetch_stats().await;
                } else if is_ws_query {
                    self.ws_query_value = self.ws_query_value_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.ws_run_query().await;
                } else if is_stats {
                    self.vr_stats_filter = self.vr_stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.vr_fetch_stats().await;
                } else {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.vr_page_start = 0;
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
                if is_pl_issues {
                    self.pl_issues_filter_edit = self.pl_issues_filter.clone();
                } else if is_ws_stats {
                    self.ws_stats_filter_edit = self.ws_stats_filter.clone();
                } else if is_ws_query {
                    self.ws_query_value_edit = self.ws_query_value.clone();
                } else if is_stats {
                    self.vr_stats_filter_edit = self.vr_stats_filter.clone();
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
    }

}
