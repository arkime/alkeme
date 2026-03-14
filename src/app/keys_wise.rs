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
                self.wise.stats_filter_edit = self.wise.stats_filter.clone();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('1') => {
                self.wise.stats_tab = WsStatsTab::Sources;
                self.wise.stats_selected = 0;
            }
            KeyCode::Char('2') => {
                self.wise.stats_tab = WsStatsTab::Types;
                self.wise.stats_selected = 0;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.wise.stats_selected = self.wise.stats_selected.saturating_sub(20);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let max = match self.wise.stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                self.wise.stats_selected = (self.wise.stats_selected + 20).min(max.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.wise.stats_selected > 0 {
                    self.wise.stats_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = match self.wise.stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                if self.wise.stats_selected + 1 < max {
                    self.wise.stats_selected += 1;
                }
            }
            KeyCode::Home | KeyCode::Left => {
                self.wise.stats_selected = 0;
            }
            KeyCode::End | KeyCode::Right => {
                let max = match self.wise.stats_tab {
                    WsStatsTab::Sources => self.ws_filtered_sources().len(),
                    WsStatsTab::Types => self.ws_filtered_types().len(),
                };
                self.wise.stats_selected = max.saturating_sub(1);
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
                self.wise.query_value_edit = self.wise.query_value.clone();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                // Cycle source
                if self.wise.sources.is_empty() { return; }
                let mut all = vec!["any".to_string()];
                all.extend(self.wise.sources.iter().cloned());
                let idx = all.iter().position(|s| s == &self.wise.query_source).unwrap_or(0);
                self.wise.query_source = all[(idx + 1) % all.len()].clone();
            }
            KeyCode::Char('t') => {
                // Cycle type
                if self.wise.types.is_empty() { return; }
                let idx = self.wise.types.iter().position(|t| t == &self.wise.query_type).unwrap_or(0);
                self.wise.query_type = self.wise.types[(idx + 1) % self.wise.types.len()].clone();
            }
            KeyCode::Enter => {
                self.ws_run_query().await;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.wise.query_selected > 0 {
                    self.wise.query_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.wise.query_selected + 1 < self.wise.query_results.len() {
                    self.wise.query_selected += 1;
                }
            }
            KeyCode::Home | KeyCode::Left => {
                self.wise.query_selected = 0;
            }
            KeyCode::End | KeyCode::Right => {
                self.wise.query_selected = self.wise.query_results.len().saturating_sub(1);
            }
            _ => {}
        }
    }
}
