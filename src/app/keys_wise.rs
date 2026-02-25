use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, InputMode, Tab, WsStatsTab};

impl App {
    pub(crate) async fn handle_wise_key(&mut self, key: KeyEvent) {
        match self.active_tab {
            Tab::WsStats => self.handle_ws_stats_key(key).await,
            Tab::WsQuery => self.handle_ws_query_key(key).await,
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

    async fn handle_ws_stats_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Expression {
            return; // handled by expression handler
        }

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('r') => {
                self.ws_fetch_stats().await;
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.ws_stats_filter_edit = self.ws_stats_filter.clone();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('1') => {
                self.ws_stats_tab = WsStatsTab::Sources;
                self.ws_stats_selected = 0;
            }
            KeyCode::Char('2') => {
                self.ws_stats_tab = WsStatsTab::Types;
                self.ws_stats_selected = 0;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.ws_stats_selected = self.ws_stats_selected.saturating_sub(20);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let max = match self.ws_stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                self.ws_stats_selected = (self.ws_stats_selected + 20).min(max.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.ws_stats_selected > 0 {
                    self.ws_stats_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = match self.ws_stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                if self.ws_stats_selected + 1 < max {
                    self.ws_stats_selected += 1;
                }
            }
            KeyCode::Home | KeyCode::Left => {
                self.ws_stats_selected = 0;
            }
            KeyCode::End | KeyCode::Right => {
                let max = match self.ws_stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                self.ws_stats_selected = max.saturating_sub(1);
            }
            _ => {}
        }
    }

    async fn handle_ws_query_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Expression {
            return; // handled by expression handler
        }

        match key.code {
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.ws_query_value_edit = self.ws_query_value.clone();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                // Cycle source
                if self.ws_sources.is_empty() { return; }
                let mut all = vec!["any".to_string()];
                all.extend(self.ws_sources.iter().cloned());
                let idx = all.iter().position(|s| s == &self.ws_query_source).unwrap_or(0);
                self.ws_query_source = all[(idx + 1) % all.len()].clone();
            }
            KeyCode::Char('t') => {
                // Cycle type
                if self.ws_types.is_empty() { return; }
                let idx = self.ws_types.iter().position(|t| t == &self.ws_query_type).unwrap_or(0);
                self.ws_query_type = self.ws_types[(idx + 1) % self.ws_types.len()].clone();
            }
            KeyCode::Enter => {
                self.ws_run_query().await;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.ws_query_selected > 0 {
                    self.ws_query_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.ws_query_selected + 1 < self.ws_query_results.len() {
                    self.ws_query_selected += 1;
                }
            }
            KeyCode::Home | KeyCode::Left => {
                self.ws_query_selected = 0;
            }
            KeyCode::End | KeyCode::Right => {
                self.ws_query_selected = self.ws_query_results.len().saturating_sub(1);
            }
            _ => {}
        }
    }
}
