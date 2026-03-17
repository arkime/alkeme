use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

// --- Generic column editor helpers ---

/// Mutable state references for column editor key handling.
/// Abstracts the difference between session and stats column editors.
pub(super) struct ColumnEditorState<'a> {
    pub filter: &'a mut String,
    pub filter_cursor: &'a mut usize,
    pub selected: &'a mut usize,
    pub mode: &'a mut ColumnEditorMode,
    pub show: &'a mut bool,
}

/// Items in the column editor — abstracted as (name1, name2, enabled).
pub(super) trait EditorItemTrait {
    fn search_text_a(&self) -> &str;
    fn search_text_b(&self) -> &str;
    fn enabled(&self) -> bool;
    fn set_enabled(&mut self, val: bool);
}

impl EditorItemTrait for crate::app::types::ColumnEditorItem {
    fn search_text_a(&self) -> &str { &self.exp }
    fn search_text_b(&self) -> &str { &self.friendly_name }
    fn enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, val: bool) { self.enabled = val; }
}

impl EditorItemTrait for StatsColumnEditorItem {
    fn search_text_a(&self) -> &str { &self.field }
    fn search_text_b(&self) -> &str { &self.label }
    fn enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, val: bool) { self.enabled = val; }
}

pub(super) fn filtered_indices<T: EditorItemTrait>(filter: &str, items: &[T]) -> Vec<usize> {
    let filter_text = filter.trim_matches('\0');
    if filter_text.is_empty() {
        return (0..items.len()).collect();
    }
    let f = filter_text.to_lowercase();
    items.iter().enumerate()
        .filter(|(_, item)| {
            item.search_text_a().to_lowercase().contains(&f)
                || item.search_text_b().to_lowercase().contains(&f)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Handle keys for a column editor (works for both session and stats).
/// Returns true if the caller should also run a post-action (apply/default).
pub(super) fn handle_column_editor_key_generic<T: EditorItemTrait>(
    key: KeyEvent,
    state: &mut ColumnEditorState,
    items: &mut Vec<T>,
    show_help: &mut bool,
) -> Option<&'static str> {
    let filtered = filtered_indices(state.filter, items);
    let cur_pos = filtered.iter().position(|&i| i == *state.selected);

    // Filter mode active
    if !state.filter.is_empty() {
        match key.code {
            KeyCode::Esc => {
                state.filter.clear();
                *state.filter_cursor = 0;
                *state.selected = 0;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(item) = items.get_mut(*state.selected) {
                    let v = !item.enabled();
                    item.set_enabled(v);
                }
            }
            KeyCode::Down => {
                if let Some(pos) = cur_pos {
                    if pos + 1 < filtered.len() {
                        *state.selected = filtered[pos + 1];
                    }
                } else if !filtered.is_empty() {
                    *state.selected = filtered[0];
                }
            }
            KeyCode::Up => {
                if let Some(pos) = cur_pos {
                    if pos > 0 {
                        *state.selected = filtered[pos - 1];
                    }
                } else if !filtered.is_empty() {
                    *state.selected = filtered[0];
                }
            }
            _ => {
                if handle_text_input_key(key.code, state.filter, state.filter_cursor) {
                    let filtered = filtered_indices(state.filter, items);
                    if !filtered.is_empty() {
                        *state.selected = filtered[0];
                    }
                }
            }
        }
        return None;
    }

    // Normal mode
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            *state.show = false;
        }
        KeyCode::Char('h') | KeyCode::Char('?') => {
            *show_help = !*show_help;
        }
        KeyCode::Char('/') => {
            *state.filter = "\0".to_string();
            *state.filter_cursor = 0;
        }
        KeyCode::Enter => {
            if *state.mode == ColumnEditorMode::Reorder {
                *state.mode = ColumnEditorMode::Browse;
            } else if let Some(item) = items.get_mut(*state.selected) {
                let v = !item.enabled();
                item.set_enabled(v);
            }
        }
        KeyCode::Char(' ') => {
            if let Some(item) = items.get_mut(*state.selected) {
                let v = !item.enabled();
                item.set_enabled(v);
            }
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if let Some(pos) = cur_pos {
                let new_pos = (pos + 10).min(filtered.len().saturating_sub(1));
                *state.selected = filtered[new_pos];
            }
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if let Some(pos) = cur_pos {
                let new_pos = pos.saturating_sub(10);
                *state.selected = filtered[new_pos];
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if *state.mode == ColumnEditorMode::Reorder {
                let len = items.len();
                if *state.selected + 1 < len {
                    items.swap(*state.selected, *state.selected + 1);
                    *state.selected += 1;
                }
            } else if let Some(pos) = cur_pos
                && pos + 1 < filtered.len() {
                    *state.selected = filtered[pos + 1];
                }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if *state.mode == ColumnEditorMode::Reorder {
                if *state.selected > 0 {
                    items.swap(*state.selected, *state.selected - 1);
                    *state.selected -= 1;
                }
            } else if let Some(pos) = cur_pos
                && pos > 0 {
                    *state.selected = filtered[pos - 1];
                }
        }
        KeyCode::Char('m') => {
            if *state.mode == ColumnEditorMode::Reorder {
                *state.mode = ColumnEditorMode::Browse;
            } else {
                *state.mode = ColumnEditorMode::Reorder;
            }
        }
        KeyCode::Char('a') => return Some("apply"),
        KeyCode::Char('d') => return Some("default"),
        _ => {}
    }
    None
}

// --- Generic layout popup helpers ---

pub(super) struct LayoutPopupState<'a> {
    pub mode: &'a mut LayoutPopupMode,
    pub selected: &'a mut usize,
    pub filter: &'a mut String,
    pub filter_cursor: &'a mut usize,
    pub save_name: &'a mut String,
    pub save_cursor: &'a mut usize,
    pub show: &'a mut bool,
    pub delete_name: &'a mut String,
}

pub(super) struct LayoutItem {
    pub name: String,
    pub shared: bool,
}

/// Compute filtered indices for a layout item list
pub(super) fn layout_filtered_indices(filter: &str, items: &[LayoutItem]) -> Vec<usize> {
    let filter_text = filter.trim_matches('\0');
    if filter_text.is_empty() {
        return (0..items.len()).collect();
    }
    let f = filter_text.to_lowercase();
    items.iter().enumerate()
        .filter(|(_, item)| item.name.to_lowercase().contains(&f))
        .map(|(i, _)| i)
        .collect()
}

/// Handle keys for a layout popup (works for both session and stats).
/// Returns a command string: "edit", "save:{name}", "default", "select:{idx}", "delete:{idx}", or None.
pub(super) fn handle_layout_popup_key_generic(
    key: KeyEvent,
    state: &mut LayoutPopupState,
    items: &[LayoutItem],
    show_help: &mut bool,
) -> Option<String> {
    match state.mode {
        LayoutPopupMode::ConfirmDelete => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let name = state.delete_name.clone();
                    *state.mode = LayoutPopupMode::List;
                    return Some(format!("confirm_delete:{name}"));
                }
                _ => {
                    *state.mode = LayoutPopupMode::List;
                }
            }
        }
        LayoutPopupMode::List => {
            // Filter mode
            if !state.filter.is_empty() {
                let filtered = layout_filtered_indices(state.filter, items);
                let cur_pos = filtered.iter().position(|&i| i + 3 == *state.selected);
                match key.code {
                    KeyCode::Esc => {
                        state.filter.clear();
                        *state.filter_cursor = 0;
                        *state.selected = 0;
                    }
                    KeyCode::Enter => {
                        if let Some(&idx) = filtered.iter().find(|&&i| i + 3 == *state.selected) {
                            *state.show = false;
                            state.filter.clear();
                            *state.filter_cursor = 0;
                            return Some(format!("select:{idx}"));
                        }
                    }
                    KeyCode::Down => {
                        if let Some(pos) = cur_pos {
                            if pos + 1 < filtered.len() {
                                *state.selected = filtered[pos + 1] + 3;
                            }
                        } else if let Some(&first) = filtered.first() {
                            *state.selected = first + 3;
                        }
                    }
                    KeyCode::Up => {
                        if let Some(pos) = cur_pos
                            && pos > 0 {
                                *state.selected = filtered[pos - 1] + 3;
                            }
                    }
                    _ => {
                        if handle_text_input_key(key.code, state.filter, state.filter_cursor) {
                            if state.filter.is_empty() || *state.filter == "\0" {
                                state.filter.clear();
                                *state.filter_cursor = 0;
                                *state.selected = 0;
                            } else {
                                let filtered = layout_filtered_indices(state.filter, items);
                                if let Some(&first) = filtered.first() {
                                    *state.selected = first + 3;
                                }
                            }
                        }
                    }
                }
                return None;
            }

            // Normal list mode
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    *state.show = false;
                }
                KeyCode::Char('h') | KeyCode::Char('?') => {
                    *show_help = !*show_help;
                }
                KeyCode::Char('/') => {
                    *state.filter = "\0".to_string();
                    *state.filter_cursor = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = items.len() + 3;
                    if *state.selected + 1 < max {
                        *state.selected += 1;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    *state.selected = state.selected.saturating_sub(1);
                }
                KeyCode::Enter => {
                    if *state.selected == 0 {
                        return Some("edit".to_string());
                    } else if *state.selected == 1 {
                        *state.mode = LayoutPopupMode::SaveInput;
                        state.save_name.clear();
                        *state.save_cursor = 0;
                    } else if *state.selected == 2 {
                        *state.show = false;
                        return Some("default".to_string());
                    } else {
                        let idx = *state.selected - 3;
                        *state.show = false;
                        return Some(format!("select:{idx}"));
                    }
                }
                KeyCode::Char('x') | KeyCode::Delete => {
                    if let Some(idx) = state.selected.checked_sub(3)
                        && let Some(item) = items.get(idx)
                            && !item.shared {
                                *state.delete_name = item.name.clone();
                                *state.mode = LayoutPopupMode::ConfirmDelete;
                            }
                }
                _ => {}
            }
        }
        LayoutPopupMode::SaveInput => {
            match key.code {
                KeyCode::Esc => {
                    *state.mode = LayoutPopupMode::List;
                }
                KeyCode::Enter => {
                    if !state.save_name.is_empty() {
                        let name = state.save_name.clone();
                        *state.show = false;
                        return Some(format!("save:{name}"));
                    }
                }
                KeyCode::Backspace | KeyCode::Left | KeyCode::Right | KeyCode::Char(_) => {
                    handle_text_input_key(key.code, state.save_name, state.save_cursor);
                }
                _ => {}
            }
        }
    }
    None
}
