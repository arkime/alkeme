use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;
use super::*;

impl App {
    pub(crate) async fn handle_users_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => { /* quit handled by caller check in main */ }
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.us_selected = (self.us_selected + 10).min(self.us_users.len().saturating_sub(1));
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.us_selected = self.us_selected.saturating_sub(10);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.us_users.is_empty() {
                    self.us_selected = (self.us_selected + 1).min(self.us_users.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.us_selected > 0 {
                    self.us_selected -= 1;
                }
            }
            KeyCode::Home => {
                self.us_selected = 0;
            }
            KeyCode::End => {
                if !self.us_users.is_empty() {
                    self.us_selected = self.us_users.len() - 1;
                }
            }
            KeyCode::Char('/') | KeyCode::Char('E') => {
                self.expression_edit = self.us_filter.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let fields = ["userId", "userName", "enabled", "lastUsed"];
                let idx = fields.iter().position(|&f| f == self.us_sort_field).unwrap_or(0);
                self.us_sort_field = fields[(idx + 1) % fields.len()].to_string();
                self.us_fetch_users().await;
            }
            KeyCode::Char('S') => {
                self.us_sort_desc = !self.us_sort_desc;
                self.us_fetch_users().await;
            }
            KeyCode::Char('r') => {
                self.us_fetch_users().await;
            }
            KeyCode::Enter => {
                if let Some(user) = self.us_users.get(self.us_selected) {
                    self.us_editor_user = user.clone();
                    self.us_editor_field = 1; // start on userName (skip userId)
                    self.us_editing = true;
                    self.us_load_editor_field();
                }
            }
            KeyCode::Right => {
                if self.us_page_start + self.us_page_size < self.us_filtered {
                    self.us_page_start += self.us_page_size;
                    self.us_selected = 0;
                    self.us_fetch_users().await;
                }
            }
            KeyCode::Left => {
                if self.us_page_start >= self.us_page_size {
                    self.us_page_start -= self.us_page_size;
                    self.us_selected = 0;
                    self.us_fetch_users().await;
                }
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('D') => {
                self.show_debug = true;
                self.debug_selected = 0;
                self.debug_expanded = false;
            }
            _ => {}
        }
    }

    pub(crate) async fn handle_users_editor_key(&mut self, key: KeyEvent) {
        let fields = Self::us_editor_fields();
        let field_count = fields.len();
        let is_text_field = if self.us_editor_field < field_count {
            let (name, ft) = fields[self.us_editor_field];
            ft == "text" && name != "userId"
        } else {
            false
        };

        match key.code {
            KeyCode::Esc => {
                self.us_editing = false;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.us_commit_editor_field();
                self.us_commit_roles();
                self.us_save_user().await;
            }
            KeyCode::Tab | KeyCode::Down => {
                self.us_commit_editor_field();
                self.us_editor_field = (self.us_editor_field + 1) % field_count;
                if self.us_editor_field == 0 { self.us_editor_field = 1; }
                self.us_load_editor_field();
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.us_commit_editor_field();
                if self.us_editor_field <= 1 {
                    self.us_editor_field = field_count - 1;
                } else {
                    self.us_editor_field -= 1;
                }
                self.us_load_editor_field();
            }
            KeyCode::Char(' ') | KeyCode::Enter if !is_text_field => {
                if self.us_editor_field >= field_count { return; }
                let (field_name, field_type) = fields[self.us_editor_field];
                if field_name == "userId" { return; }
                if field_type == "bool" {
                    let current = self.us_editor_user.get(field_name)
                        .and_then(|v| v.as_bool()).unwrap_or(false);
                    self.us_editor_user[field_name] = Value::Bool(!current);
                } else if field_type == "roles" {
                    if self.us_all_roles.is_empty() {
                        self.us_fetch_roles().await;
                    }
                    self.us_build_editor_roles();
                    self.us_role_popup_open = true;
                    self.us_role_popup_selected = 0;
                    self.us_role_popup_filter.clear();
                    self.us_role_popup_cursor = 0;
                    self.us_role_popup_filtering = false;
                }
            }
            _ => {
                if is_text_field {
                    handle_text_input_key(key.code, &mut self.us_editor_text, &mut self.us_editor_cursor);
                }
            }
        }
    }

    pub(crate) async fn handle_users_role_popup_key(&mut self, key: KeyEvent) {
        if self.us_role_popup_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.us_role_popup_filtering = false;
                }
                KeyCode::Enter | KeyCode::Down => {
                    self.us_role_popup_filtering = false;
                }
                _ => {
                    if handle_text_input_key(key.code, &mut self.us_role_popup_filter, &mut self.us_role_popup_cursor) {
                        self.us_role_popup_selected = 0;
                    }
                }
            }
            return;
        }

        let filtered = self.us_role_popup_filtered();
        match key.code {
            KeyCode::Esc => {
                self.us_commit_roles();
                self.us_role_popup_open = false;
            }
            KeyCode::Char('/') => {
                self.us_role_popup_filtering = true;
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(&idx) = filtered.get(self.us_role_popup_selected) {
                    self.us_editor_roles[idx].1 = !self.us_editor_roles[idx].1;
                }
            }
            KeyCode::Char('a') => {
                for &idx in &filtered {
                    self.us_editor_roles[idx].1 = true;
                }
            }
            KeyCode::Char('n') => {
                for &idx in &filtered {
                    self.us_editor_roles[idx].1 = false;
                }
            }
            KeyCode::Char('!') => {
                for &idx in &filtered {
                    self.us_editor_roles[idx].1 = !self.us_editor_roles[idx].1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.us_role_popup_selected > 0 {
                    self.us_role_popup_selected -= 1;
                } else if !filtered.is_empty() {
                    self.us_role_popup_selected = filtered.len() - 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.us_role_popup_selected + 1 < filtered.len() {
                    self.us_role_popup_selected += 1;
                } else {
                    self.us_role_popup_selected = 0;
                }
            }
            _ => {}
        }
    }
}
