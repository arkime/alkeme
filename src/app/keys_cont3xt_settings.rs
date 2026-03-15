use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::*;

impl App {
    /// Handle keys for Link Groups settings sub-tab (returns true if handled/should return)
    pub(crate) fn handle_c3_lg_settings_key(&mut self, key: KeyEvent) -> bool {
            match self.cont3xt.lg_level {
                C3LinkGroupLevel::LinkEditor => {
                    let all_itypes = ["domain", "ip", "url", "email", "hash", "phone", "text"];
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.lg_level = C3LinkGroupLevel::LinkList;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Apply edits back to the group's link
                            let gi = self.cont3xt.lg_editing_group_idx;
                            let is_editable = self.cont3xt.lg_groups.get(gi)
                                .map(|g| g.editable).unwrap_or(false);
                            if !is_editable {
                                self.status_msg = "Link group is not editable".to_string();
                                return true;
                            }
                            let li = self.cont3xt.lg_editor_link_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                if let Some(link) = group.links.get_mut(li) {
                                    *link = self.cont3xt.lg_editor_link.clone();
                                }
                            }
                            self.cont3xt.lg_level = C3LinkGroupLevel::LinkList;
                        }
                        KeyCode::Up | KeyCode::Char('k') if self.cont3xt.lg_editor_field == C3LinkEditorField::Itypes => {
                            if self.cont3xt.lg_editor_itype_selected > 0 {
                                self.cont3xt.lg_editor_itype_selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') if self.cont3xt.lg_editor_field == C3LinkEditorField::Itypes => {
                            if self.cont3xt.lg_editor_itype_selected + 1 < all_itypes.len() {
                                self.cont3xt.lg_editor_itype_selected += 1;
                            }
                        }
                        KeyCode::Char(' ') if self.cont3xt.lg_editor_field == C3LinkEditorField::Itypes => {
                            let itype = all_itypes[self.cont3xt.lg_editor_itype_selected].to_string();
                            if let Some(pos) = self.cont3xt.lg_editor_link.itypes.iter().position(|t| t == &itype) {
                                self.cont3xt.lg_editor_link.itypes.remove(pos);
                            } else {
                                self.cont3xt.lg_editor_link.itypes.push(itype);
                            }
                        }
                        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                            let fields = C3LinkEditorField::all();
                            if let Some(pos) = fields.iter().position(|f| *f == self.cont3xt.lg_editor_field) {
                                self.cont3xt.lg_editor_field = fields[(pos + 1) % fields.len()];
                                self.cont3xt.lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                self.cont3xt.lg_editor_itype_selected = 0;
                            }
                        }
                        KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                            let fields = C3LinkEditorField::all();
                            if let Some(pos) = fields.iter().position(|f| *f == self.cont3xt.lg_editor_field) {
                                self.cont3xt.lg_editor_field = fields[(pos + fields.len() - 1) % fields.len()];
                                self.cont3xt.lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                self.cont3xt.lg_editor_itype_selected = 0;
                            }
                        }
                        _ if self.cont3xt.lg_editor_field != C3LinkEditorField::Itypes => {
                            // Text input for non-itypes fields
                            match key.code {
                                KeyCode::Char(c) => {
                                    let pos = self.cont3xt.lg_editor_cursor;
                                    self.c3_lg_editor_field_value_mut().insert(pos, c);
                                    self.cont3xt.lg_editor_cursor += 1;
                                }
                                KeyCode::Backspace => {
                                    if self.cont3xt.lg_editor_cursor > 0 {
                                        self.cont3xt.lg_editor_cursor -= 1;
                                        let pos = self.cont3xt.lg_editor_cursor;
                                        self.c3_lg_editor_field_value_mut().remove(pos);
                                    }
                                }
                                KeyCode::Delete => {
                                    let len = self.c3_lg_editor_field_value().len();
                                    let pos = self.cont3xt.lg_editor_cursor;
                                    if pos < len {
                                        self.c3_lg_editor_field_value_mut().remove(pos);
                                    }
                                }
                                KeyCode::Left => {
                                    self.cont3xt.lg_editor_cursor = self.cont3xt.lg_editor_cursor.saturating_sub(1);
                                }
                                KeyCode::Right => {
                                    let len = self.c3_lg_editor_field_value().len();
                                    if self.cont3xt.lg_editor_cursor < len {
                                        self.cont3xt.lg_editor_cursor += 1;
                                    }
                                }
                                KeyCode::Home => self.cont3xt.lg_editor_cursor = 0,
                                KeyCode::End => {
                                    self.cont3xt.lg_editor_cursor = self.c3_lg_editor_field_value().len();
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') if self.cont3xt.lg_editor_field == C3LinkEditorField::Itypes => {
                            self.show_help = true;
                        }
                        _ => {}
                    }
                    return true;
                }
                C3LinkGroupLevel::GroupEditor => {
                    // Role popup intercept (reuse existing role popup)
                    if self.cont3xt.role_popup_open {
                        match key.code {
                            KeyCode::Esc => { self.cont3xt.role_popup_open = false; }
                            KeyCode::Char('/') => { self.cont3xt.role_popup_filtering = !self.cont3xt.role_popup_filtering; }
                            KeyCode::Char(' ') | KeyCode::Enter if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if let Some(&idx) = filtered.get(self.cont3xt.role_popup_selected) {
                                    if let Some(role) = self.cont3xt.all_roles.get(idx) {
                                        let role = role.clone();
                                        let roles = if self.cont3xt.lg_group_editor_field == C3GroupEditorField::ViewRoles {
                                            &mut self.cont3xt.lg_group_editor_view_roles
                                        } else {
                                            &mut self.cont3xt.lg_group_editor_edit_roles
                                        };
                                        if let Some(pos) = roles.iter().position(|r| r == &role) {
                                            roles.remove(pos);
                                        } else {
                                            roles.push(role);
                                        }
                                    }
                                }
                            }
                            KeyCode::Down if self.cont3xt.role_popup_filtering => {
                                self.cont3xt.role_popup_filtering = false;
                            }
                            KeyCode::Up | KeyCode::Char('k') if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if self.cont3xt.role_popup_selected > 0 { self.cont3xt.role_popup_selected -= 1; }
                                else if !filtered.is_empty() { self.cont3xt.role_popup_selected = filtered.len() - 1; }
                            }
                            KeyCode::Down | KeyCode::Char('j') if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if self.cont3xt.role_popup_selected + 1 < filtered.len() { self.cont3xt.role_popup_selected += 1; }
                                else { self.cont3xt.role_popup_selected = 0; }
                            }
                            _ if self.cont3xt.role_popup_filtering => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.role_popup_filter, &mut self.cont3xt.role_popup_cursor) {
                                    self.cont3xt.role_popup_selected = 0;
                                }
                            }
                            _ => {}
                        }
                        return true;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.lg_level = C3LinkGroupLevel::GroupList;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Save group name + roles
                            let idx = self.cont3xt.lg_group_editor_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get(idx) {
                                if !group.editable {
                                    self.status_msg = "Link group is read-only".to_string();
                                    return true;
                                }
                            }
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(idx) {
                                group.name = self.cont3xt.lg_group_editor_name.clone();
                                group.view_roles = self.cont3xt.lg_group_editor_view_roles.clone();
                                group.edit_roles = self.cont3xt.lg_group_editor_edit_roles.clone();
                            }
                            if let Some(group) = self.cont3xt.lg_groups.get(idx) {
                                let payload = self.c3_lg_build_group_json(group);
                                let id = group.id.clone();
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        match self.client.c3_update_link_group(&id, &payload).await {
                                            Ok(_) => {
                                                self.status_msg = "Link group updated".to_string();
                                                self.c3_fetch_link_groups_settings().await;
                                                self.c3_fetch_link_groups().await;
                                            }
                                            Err(e) => self.status_msg = format!("Save error: {e}"),
                                        }
                                    })
                                });
                            }
                            self.cont3xt.lg_level = C3LinkGroupLevel::GroupList;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.cont3xt.lg_group_editor_field = self.cont3xt.lg_group_editor_field.prev();
                            if self.cont3xt.lg_group_editor_field == C3GroupEditorField::Name {
                                self.cont3xt.lg_group_editor_cursor = self.cont3xt.lg_group_editor_name.len();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.cont3xt.lg_group_editor_field = self.cont3xt.lg_group_editor_field.next();
                            if self.cont3xt.lg_group_editor_field == C3GroupEditorField::Name {
                                self.cont3xt.lg_group_editor_cursor = self.cont3xt.lg_group_editor_name.len();
                            }
                        }
                        KeyCode::Enter if self.cont3xt.lg_group_editor_field == C3GroupEditorField::ViewRoles
                            || self.cont3xt.lg_group_editor_field == C3GroupEditorField::EditRoles => {
                            if self.cont3xt.all_roles.is_empty() {
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_roles().await;
                                    })
                                });
                            }
                            self.cont3xt.role_popup_open = true;
                            self.cont3xt.role_popup_selected = 0;
                            self.cont3xt.role_popup_filter.clear(); self.cont3xt.role_popup_cursor = 0;
                            self.cont3xt.role_popup_filtering = false;
                        }
                        _ if self.cont3xt.lg_group_editor_field == C3GroupEditorField::Name => {
                            handle_text_input_key(key.code, &mut self.cont3xt.lg_group_editor_name, &mut self.cont3xt.lg_group_editor_cursor);
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                        _ => {}
                    }
                    return true;
                }
                C3LinkGroupLevel::LinkList => {
                    // Link list filter input mode
                    if self.cont3xt.lg_links_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.cont3xt.lg_links_filtering = false;
                            }
                            _ => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.lg_links_filter, &mut self.cont3xt.lg_links_filter_cursor) {
                                    self.cont3xt.lg_links_selected = 0;
                                    self.cont3xt.lg_links_table_state.select(Some(0));
                                }
                            }
                        }
                        return true;
                    }
                    let filtered = self.c3_lg_filtered_links();
                    let has_filter = !self.cont3xt.lg_links_filter.is_empty();
                    // Map selected index to real link index
                    let real_idx = filtered.get(self.cont3xt.lg_links_selected).copied();
                    let is_editable = self.cont3xt.lg_groups.get(self.cont3xt.lg_editing_group_idx)
                        .map(|g| g.editable).unwrap_or(false);
                    match key.code {
                        KeyCode::Esc => {
                            if has_filter {
                                self.cont3xt.lg_links_filter.clear();
                                self.cont3xt.lg_links_filter_cursor = 0;
                                self.cont3xt.lg_links_selected = 0;
                                self.cont3xt.lg_links_table_state.select(Some(0));
                            } else {
                                self.cont3xt.lg_level = C3LinkGroupLevel::GroupList;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.cont3xt.lg_links_filtering = true;
                        }
                        KeyCode::Enter => {
                            if let Some(ri) = real_idx {
                                let gi = self.cont3xt.lg_editing_group_idx;
                                if let Some(group) = self.cont3xt.lg_groups.get(gi) {
                                    if let Some(link) = group.links.get(ri) {
                                        if !link.is_separator() {
                                            self.cont3xt.lg_editor_link = link.clone();
                                            self.cont3xt.lg_editor_link_idx = ri;
                                            self.cont3xt.lg_editor_field = C3LinkEditorField::Name;
                                            self.cont3xt.lg_editor_cursor = link.name.len();
                                            self.cont3xt.lg_editor_itype_selected = 0;
                                            self.cont3xt.lg_level = C3LinkGroupLevel::LinkEditor;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('x') if is_editable => {
                            if let Some(ri) = real_idx {
                                let gi = self.cont3xt.lg_editing_group_idx;
                                if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                    if ri < group.links.len() {
                                        group.links.remove(ri);
                                        let new_filtered = self.c3_lg_filtered_links();
                                        if self.cont3xt.lg_links_selected >= new_filtered.len() && !new_filtered.is_empty() {
                                            self.cont3xt.lg_links_selected = new_filtered.len() - 1;
                                        }
                                        self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') if !has_filter && is_editable => {
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                let insert_pos = real_idx.map(|r| r + 1).unwrap_or(group.links.len()).min(group.links.len());
                                group.links.insert(insert_pos, crate::api::Cont3xtLink {
                                    name: "New Link".to_string(),
                                    url: String::new(),
                                    itypes: vec!["domain".to_string(), "ip".to_string(), "url".to_string()],
                                    info: String::new(),
                                    color: String::new(),
                                    external_doc_name: String::new(),
                                    external_doc_url: String::new(),
                                });
                                self.cont3xt.lg_links_selected = self.cont3xt.lg_links_selected + 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Char('N') if !has_filter && is_editable => {
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                group.links.push(crate::api::Cont3xtLink {
                                    name: "New Link".to_string(),
                                    url: String::new(),
                                    itypes: vec!["domain".to_string(), "ip".to_string(), "url".to_string()],
                                    info: String::new(),
                                    color: String::new(),
                                    external_doc_name: String::new(),
                                    external_doc_url: String::new(),
                                });
                                self.cont3xt.lg_links_selected = group.links.len() - 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Char('a') if !has_filter && is_editable => {
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                let insert_pos = real_idx.map(|r| r + 1).unwrap_or(group.links.len()).min(group.links.len());
                                group.links.insert(insert_pos, crate::api::Cont3xtLink::new_separator());
                                self.cont3xt.lg_links_selected = self.cont3xt.lg_links_selected + 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Char('A') if !has_filter && is_editable => {
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                group.links.push(crate::api::Cont3xtLink::new_separator());
                                self.cont3xt.lg_links_selected = group.links.len() - 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) && !has_filter && is_editable => {
                            if let Some(ri) = real_idx {
                                let gi = self.cont3xt.lg_editing_group_idx;
                                if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                    if ri > 0 {
                                        group.links.swap(ri, ri - 1);
                                        self.cont3xt.lg_links_selected = self.cont3xt.lg_links_selected.saturating_sub(1);
                                        self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) && !has_filter && is_editable => {
                            if let Some(ri) = real_idx {
                                let gi = self.cont3xt.lg_editing_group_idx;
                                if let Some(group) = self.cont3xt.lg_groups.get_mut(gi) {
                                    if ri + 1 < group.links.len() {
                                        group.links.swap(ri, ri + 1);
                                        self.cont3xt.lg_links_selected += 1;
                                        self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Save the group to server
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get(gi) {
                                if !group.editable {
                                    self.status_msg = "Link group is read-only".to_string();
                                    return true;
                                }
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
                                let new_idx = self.cont3xt.lg_groups.iter().position(|g| g.id == group_id).unwrap_or(0);
                                self.cont3xt.lg_editing_group_idx = new_idx;
                                self.cont3xt.lg_links_selected = 0;
                                self.cont3xt.lg_links_table_state.select(Some(0));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.cont3xt.lg_links_selected + 1 < filtered.len() {
                                self.cont3xt.lg_links_selected += 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.cont3xt.lg_links_selected > 0 {
                                self.cont3xt.lg_links_selected -= 1;
                                self.cont3xt.lg_links_table_state.select(Some(self.cont3xt.lg_links_selected));
                            }
                        }
                        KeyCode::Char('B') => {
                            let gi = self.cont3xt.lg_editing_group_idx;
                            if let Some(group) = self.cont3xt.lg_groups.get(gi) {
                                let safe_name = group.name.replace(['/', '\\', ' '], "_");
                                self.cont3xt.backup_kind = C3BackupKind::LinkGroupSingle;
                                let fname = format!("{}.json", safe_name);
                                self.cont3xt.backup_cursor = fname.len();
                                self.cont3xt.backup_prompt = Some(fname);
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                        _ => {}
                    }
                    return true;
                }
                C3LinkGroupLevel::GroupList => {
                    // Group list filter mode
                    if self.cont3xt.lg_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.cont3xt.lg_filtering = false;
                            }
                            _ => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.lg_filter, &mut self.cont3xt.lg_filter_cursor) {
                                    self.cont3xt.lg_selected = 0;
                                    self.cont3xt.lg_table_state.select(Some(0));
                                }
                            }
                        }
                        return true;
                    }
                    // handled below in the main match
                }
            }
        false
    }

    /// Handle keys for Overviews settings sub-tab (returns true if handled/should return)
    pub(crate) fn handle_c3_ov_settings_key(&mut self, key: KeyEvent) -> bool {
            match self.cont3xt.ov_level {
                C3OverviewLevel::FieldEditor => {
                    // Selector popup intercept (From or Field selector)
                    if self.cont3xt.ov_fe_popup_open {
                        if self.cont3xt.ov_fe_popup_filtering {
                            match key.code {
                                KeyCode::Esc => { self.cont3xt.ov_fe_popup_filtering = false; }
                                KeyCode::Enter | KeyCode::Down => { self.cont3xt.ov_fe_popup_filtering = false; }
                                _ => {
                                    if handle_text_input_key(key.code, &mut self.cont3xt.ov_fe_popup_filter, &mut self.cont3xt.ov_fe_popup_cursor) {
                                        self.cont3xt.ov_fe_popup_selected = 0;
                                    }
                                }
                            }
                            return true;
                        }
                        let filtered = self.c3_ov_fe_popup_filtered();
                        match key.code {
                            KeyCode::Esc => { self.cont3xt.ov_fe_popup_open = false; }
                            KeyCode::Char('/') => { self.cont3xt.ov_fe_popup_filtering = true; }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if self.cont3xt.ov_fe_popup_selected > 0 { self.cont3xt.ov_fe_popup_selected -= 1; }
                                else if !filtered.is_empty() { self.cont3xt.ov_fe_popup_selected = filtered.len() - 1; }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if self.cont3xt.ov_fe_popup_selected + 1 < filtered.len() { self.cont3xt.ov_fe_popup_selected += 1; }
                                else { self.cont3xt.ov_fe_popup_selected = 0; }
                            }
                            KeyCode::Enter => {
                                if let Some(&idx) = filtered.get(self.cont3xt.ov_fe_popup_selected) {
                                    if let Some(name) = self.cont3xt.ov_fe_popup_items.get(idx).cloned() {
                                        if self.cont3xt.ov_fe_popup_for_field {
                                            // Field selector
                                            if name == "Custom" {
                                                self.cont3xt.ov_field_editor_is_custom = true;
                                                self.cont3xt.ov_field_editor_field_name.clear();
                                                self.cont3xt.ov_field_editor_label.clear();
                                                if self.cont3xt.ov_fe_json_lines.is_empty() {
                                                    self.cont3xt.ov_fe_json_lines = vec![
                                                        "{".to_string(),
                                                        "  \"field\": \"\",".to_string(),
                                                        "  \"label\": \"\",".to_string(),
                                                        "  \"type\": \"string\"".to_string(),
                                                        "}".to_string(),
                                                    ];
                                                }
                                                self.cont3xt.ov_field_editor_field = C3OvFieldEditorField::CustomJson;
                                                self.cont3xt.ov_fe_json_line = 0;
                                                self.cont3xt.ov_fe_json_col = 0;
                                            } else {
                                                self.cont3xt.ov_field_editor_is_custom = false;
                                                self.cont3xt.ov_field_editor_field_name = name;
                                                self.cont3xt.ov_fe_json_lines.clear();
                                            }
                                        } else {
                                            // From selector — clear field if integration changed
                                            if self.cont3xt.ov_field_editor_from != name {
                                                self.cont3xt.ov_field_editor_field_name.clear();
                                                self.cont3xt.ov_field_editor_is_custom = false;
                                                self.cont3xt.ov_fe_json_lines.clear();
                                            }
                                            self.cont3xt.ov_field_editor_from = name;
                                        }
                                    }
                                }
                                self.cont3xt.ov_fe_popup_open = false;
                            }
                            _ => {}
                        }
                        return true;
                    }

                    // Multiline JSON editor when on CustomJson field
                    if self.cont3xt.ov_field_editor_field == C3OvFieldEditorField::CustomJson {
                        match key.code {
                            KeyCode::Esc => {
                                self.cont3xt.ov_level = C3OverviewLevel::FieldList;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.c3_ov_fe_save_field();
                                return true;
                            }
                            KeyCode::Up => {
                                if self.cont3xt.ov_fe_json_line > 0 {
                                    self.cont3xt.ov_fe_json_line -= 1;
                                    let line_len = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).map(|l| l.len()).unwrap_or(0);
                                    self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_col.min(line_len);
                                } else {
                                    // Move to From field
                                    self.cont3xt.ov_field_editor_field = C3OvFieldEditorField::From;
                                }
                            }
                            KeyCode::Down => {
                                if self.cont3xt.ov_fe_json_line + 1 < self.cont3xt.ov_fe_json_lines.len() {
                                    self.cont3xt.ov_fe_json_line += 1;
                                    let line_len = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).map(|l| l.len()).unwrap_or(0);
                                    self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_col.min(line_len);
                                }
                            }
                            KeyCode::Left => {
                                if self.cont3xt.ov_fe_json_col > 0 {
                                    self.cont3xt.ov_fe_json_col -= 1;
                                } else if self.cont3xt.ov_fe_json_line > 0 {
                                    self.cont3xt.ov_fe_json_line -= 1;
                                    self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).map(|l| l.len()).unwrap_or(0);
                                }
                            }
                            KeyCode::Right => {
                                let line_len = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).map(|l| l.len()).unwrap_or(0);
                                if self.cont3xt.ov_fe_json_col < line_len {
                                    self.cont3xt.ov_fe_json_col += 1;
                                } else if self.cont3xt.ov_fe_json_line + 1 < self.cont3xt.ov_fe_json_lines.len() {
                                    self.cont3xt.ov_fe_json_line += 1;
                                    self.cont3xt.ov_fe_json_col = 0;
                                }
                            }
                            KeyCode::Home => { self.cont3xt.ov_fe_json_col = 0; }
                            KeyCode::End => {
                                self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).map(|l| l.len()).unwrap_or(0);
                            }
                            KeyCode::Enter => {
                                // Split current line at cursor
                                if let Some(line) = self.cont3xt.ov_fe_json_lines.get(self.cont3xt.ov_fe_json_line).cloned() {
                                    let col = self.cont3xt.ov_fe_json_col.min(line.len());
                                    let remainder = line[col..].to_string();
                                    self.cont3xt.ov_fe_json_lines[self.cont3xt.ov_fe_json_line] = line[..col].to_string();
                                    self.cont3xt.ov_fe_json_line += 1;
                                    self.cont3xt.ov_fe_json_lines.insert(self.cont3xt.ov_fe_json_line, remainder);
                                    self.cont3xt.ov_fe_json_col = 0;
                                }
                            }
                            KeyCode::Backspace => {
                                if self.cont3xt.ov_fe_json_col > 0 {
                                    if let Some(line) = self.cont3xt.ov_fe_json_lines.get_mut(self.cont3xt.ov_fe_json_line) {
                                        let col = self.cont3xt.ov_fe_json_col.min(line.len());
                                        if col > 0 {
                                            line.remove(col - 1);
                                            self.cont3xt.ov_fe_json_col -= 1;
                                        }
                                    }
                                } else if self.cont3xt.ov_fe_json_line > 0 {
                                    // Join with previous line
                                    let current = self.cont3xt.ov_fe_json_lines.remove(self.cont3xt.ov_fe_json_line);
                                    self.cont3xt.ov_fe_json_line -= 1;
                                    self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_lines[self.cont3xt.ov_fe_json_line].len();
                                    self.cont3xt.ov_fe_json_lines[self.cont3xt.ov_fe_json_line].push_str(&current);
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(line) = self.cont3xt.ov_fe_json_lines.get_mut(self.cont3xt.ov_fe_json_line) {
                                    let col = self.cont3xt.ov_fe_json_col.min(line.len());
                                    if col < line.len() {
                                        line.remove(col);
                                    } else if self.cont3xt.ov_fe_json_line + 1 < self.cont3xt.ov_fe_json_lines.len() {
                                        let next = self.cont3xt.ov_fe_json_lines.remove(self.cont3xt.ov_fe_json_line + 1);
                                        self.cont3xt.ov_fe_json_lines[self.cont3xt.ov_fe_json_line].push_str(&next);
                                    }
                                }
                            }
                            KeyCode::Tab => {
                                if let Some(line) = self.cont3xt.ov_fe_json_lines.get_mut(self.cont3xt.ov_fe_json_line) {
                                    let col = self.cont3xt.ov_fe_json_col.min(line.len());
                                    line.insert_str(col, "  ");
                                    self.cont3xt.ov_fe_json_col += 2;
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(line) = self.cont3xt.ov_fe_json_lines.get_mut(self.cont3xt.ov_fe_json_line) {
                                    let col = self.cont3xt.ov_fe_json_col.min(line.len());
                                    line.insert(col, c);
                                    self.cont3xt.ov_fe_json_col += 1;
                                }
                            }
                            _ => {}
                        }
                        return true;
                    }

                    // Non-JSON field handling (From selector, Field selector, Label text input)
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.ov_level = C3OverviewLevel::FieldList;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.c3_ov_fe_save_field();
                        }
                        KeyCode::Enter => {
                            // Open selector popup for From or Field
                            match self.cont3xt.ov_field_editor_field {
                                C3OvFieldEditorField::From => {
                                    self.cont3xt.ov_fe_popup_items = self.c3_ov_fe_integration_names();
                                    self.cont3xt.ov_fe_popup_for_field = false;
                                    self.cont3xt.ov_fe_popup_selected = self.cont3xt.ov_fe_popup_items.iter()
                                        .position(|n| n == &self.cont3xt.ov_field_editor_from).unwrap_or(0);
                                    self.cont3xt.ov_fe_popup_filter.clear(); self.cont3xt.ov_fe_popup_cursor = 0;
                                    self.cont3xt.ov_fe_popup_filtering = false;
                                    self.cont3xt.ov_fe_popup_open = true;
                                }
                                C3OvFieldEditorField::Field => {
                                    self.cont3xt.ov_fe_popup_items = self.c3_ov_fe_field_labels();
                                    self.cont3xt.ov_fe_popup_for_field = true;
                                    self.cont3xt.ov_fe_popup_selected = self.cont3xt.ov_fe_popup_items.iter()
                                        .position(|n| n == &self.cont3xt.ov_field_editor_field_name).unwrap_or(0);
                                    self.cont3xt.ov_fe_popup_filter.clear(); self.cont3xt.ov_fe_popup_cursor = 0;
                                    self.cont3xt.ov_fe_popup_filtering = false;
                                    self.cont3xt.ov_fe_popup_open = true;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.cont3xt.ov_field_editor_field = self.cont3xt.ov_field_editor_field.prev(self.cont3xt.ov_field_editor_is_custom);
                            self.cont3xt.ov_field_editor_cursor = match self.cont3xt.ov_field_editor_field {
                                C3OvFieldEditorField::Label => self.cont3xt.ov_field_editor_label.len(),
                                C3OvFieldEditorField::CustomJson => {
                                    self.cont3xt.ov_fe_json_line = self.cont3xt.ov_fe_json_lines.len().saturating_sub(1);
                                    self.cont3xt.ov_fe_json_col = self.cont3xt.ov_fe_json_lines.last().map(|l| l.len()).unwrap_or(0);
                                    0
                                }
                                _ => 0,
                            };
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.cont3xt.ov_field_editor_field = self.cont3xt.ov_field_editor_field.next(self.cont3xt.ov_field_editor_is_custom);
                            self.cont3xt.ov_field_editor_cursor = match self.cont3xt.ov_field_editor_field {
                                C3OvFieldEditorField::Label => self.cont3xt.ov_field_editor_label.len(),
                                C3OvFieldEditorField::CustomJson => {
                                    self.cont3xt.ov_fe_json_line = 0;
                                    self.cont3xt.ov_fe_json_col = 0;
                                    0
                                }
                                _ => 0,
                            };
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                        _ => {
                            // Text editing only for Label
                            if self.cont3xt.ov_field_editor_field == C3OvFieldEditorField::Label {
                                handle_text_input_key(key.code, &mut self.cont3xt.ov_field_editor_label, &mut self.cont3xt.ov_field_editor_cursor);
                            }
                        }
                    }
                    return true;
                }
                C3OverviewLevel::Editor => {
                    // Role popup intercept (reuse existing role popup)
                    if self.cont3xt.role_popup_open {
                        match key.code {
                            KeyCode::Esc => { self.cont3xt.role_popup_open = false; }
                            KeyCode::Char('/') => { self.cont3xt.role_popup_filtering = !self.cont3xt.role_popup_filtering; }
                            KeyCode::Char(' ') | KeyCode::Enter if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if let Some(&idx) = filtered.get(self.cont3xt.role_popup_selected) {
                                    if let Some(role) = self.cont3xt.all_roles.get(idx) {
                                        let role = role.clone();
                                        let roles = if self.cont3xt.role_popup_for_edit {
                                            &mut self.cont3xt.ov_editor_edit_roles
                                        } else {
                                            &mut self.cont3xt.ov_editor_view_roles
                                        };
                                        if let Some(pos) = roles.iter().position(|r| r == &role) {
                                            roles.remove(pos);
                                        } else {
                                            roles.push(role);
                                        }
                                    }
                                }
                            }
                            KeyCode::Down if self.cont3xt.role_popup_filtering => {
                                self.cont3xt.role_popup_filtering = false;
                            }
                            KeyCode::Up | KeyCode::Char('k') if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if self.cont3xt.role_popup_selected > 0 { self.cont3xt.role_popup_selected -= 1; }
                                else if !filtered.is_empty() { self.cont3xt.role_popup_selected = filtered.len() - 1; }
                            }
                            KeyCode::Down | KeyCode::Char('j') if !self.cont3xt.role_popup_filtering => {
                                let filtered = self.c3_all_roles_filtered();
                                if self.cont3xt.role_popup_selected + 1 < filtered.len() { self.cont3xt.role_popup_selected += 1; }
                                else { self.cont3xt.role_popup_selected = 0; }
                            }
                            _ if self.cont3xt.role_popup_filtering => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.role_popup_filter, &mut self.cont3xt.role_popup_cursor) {
                                    self.cont3xt.role_popup_selected = 0;
                                }
                            }
                            _ => {}
                        }
                        return true;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.cont3xt.ov_level = C3OverviewLevel::List;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let idx = self.cont3xt.ov_editor_idx;
                            let is_editable = self.cont3xt.ov_list.get(idx)
                                .map(|ov| ov.editable).unwrap_or(false);
                            if !is_editable {
                                self.status_msg = "Overview is not editable".to_string();
                                return true;
                            }
                            // Apply editor fields back to the overview
                            if let Some(ov) = self.cont3xt.ov_list.get_mut(idx) {
                                ov.name = self.cont3xt.ov_editor_name.clone();
                                ov.title = self.cont3xt.ov_editor_title.clone();
                                ov.itype = self.cont3xt.ov_editor_itype.clone();
                                ov.view_roles = self.cont3xt.ov_editor_view_roles.clone();
                                ov.edit_roles = self.cont3xt.ov_editor_edit_roles.clone();
                            }
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(self.c3_ov_save())
                            });
                            self.cont3xt.ov_level = C3OverviewLevel::List;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.cont3xt.ov_editor_field = self.cont3xt.ov_editor_field.prev();
                            self.cont3xt.ov_editor_cursor = match self.cont3xt.ov_editor_field {
                                C3OverviewEditorField::Name => self.cont3xt.ov_editor_name.len(),
                                C3OverviewEditorField::Title => self.cont3xt.ov_editor_title.len(),
                                C3OverviewEditorField::Itype => self.cont3xt.ov_editor_itype.len(),
                                _ => 0,
                            };
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.cont3xt.ov_editor_field = self.cont3xt.ov_editor_field.next();
                            self.cont3xt.ov_editor_cursor = match self.cont3xt.ov_editor_field {
                                C3OverviewEditorField::Name => self.cont3xt.ov_editor_name.len(),
                                C3OverviewEditorField::Title => self.cont3xt.ov_editor_title.len(),
                                C3OverviewEditorField::Itype => self.cont3xt.ov_editor_itype.len(),
                                _ => 0,
                            };
                        }
                        KeyCode::Enter if self.cont3xt.ov_editor_field == C3OverviewEditorField::ViewRoles
                            || self.cont3xt.ov_editor_field == C3OverviewEditorField::EditRoles => {
                            if self.cont3xt.all_roles.is_empty() {
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        self.c3_fetch_roles().await;
                                    })
                                });
                            }
                            self.cont3xt.role_popup_open = true;
                            self.cont3xt.role_popup_for_edit = self.cont3xt.ov_editor_field == C3OverviewEditorField::EditRoles;
                            self.cont3xt.role_popup_selected = 0;
                            self.cont3xt.role_popup_filter.clear(); self.cont3xt.role_popup_cursor = 0;
                            self.cont3xt.role_popup_filtering = false;
                        }
                        KeyCode::Enter => {
                            // Enter field list for this overview
                            self.cont3xt.ov_fields_selected = 0;
                            self.cont3xt.ov_fields_table_state.select(Some(0));
                            self.cont3xt.ov_fields_filter.clear();
                            self.cont3xt.ov_fields_filter_cursor = 0;
                            self.cont3xt.ov_fields_filtering = false;
                            self.cont3xt.ov_level = C3OverviewLevel::FieldList;
                        }
                        _ if matches!(self.cont3xt.ov_editor_field, C3OverviewEditorField::Name | C3OverviewEditorField::Title | C3OverviewEditorField::Itype) => {
                            let (text, cursor) = match self.cont3xt.ov_editor_field {
                                C3OverviewEditorField::Name => (&mut self.cont3xt.ov_editor_name, &mut self.cont3xt.ov_editor_cursor),
                                C3OverviewEditorField::Title => (&mut self.cont3xt.ov_editor_title, &mut self.cont3xt.ov_editor_cursor),
                                C3OverviewEditorField::Itype => (&mut self.cont3xt.ov_editor_itype, &mut self.cont3xt.ov_editor_cursor),
                                _ => unreachable!(),
                            };
                            handle_text_input_key(key.code, text, cursor);
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                        _ => {}
                    }
                    return true;
                }
                C3OverviewLevel::FieldList => {
                    // Field list filter input mode
                    if self.cont3xt.ov_fields_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.cont3xt.ov_fields_filtering = false;
                            }
                            _ => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.ov_fields_filter, &mut self.cont3xt.ov_fields_filter_cursor) {
                                    self.cont3xt.ov_fields_selected = 0;
                                    self.cont3xt.ov_fields_table_state.select(Some(0));
                                }
                            }
                        }
                        return true;
                    }
                    let filtered = self.c3_ov_filtered_fields();
                    let has_filter = !self.cont3xt.ov_fields_filter.is_empty();
                    let real_idx = filtered.get(self.cont3xt.ov_fields_selected).copied();
                    let is_editable = self.cont3xt.ov_list.get(self.cont3xt.ov_editor_idx)
                        .map(|ov| ov.editable).unwrap_or(false);
                    match key.code {
                        KeyCode::Esc => {
                            if has_filter {
                                self.cont3xt.ov_fields_filter.clear();
                                self.cont3xt.ov_fields_filter_cursor = 0;
                                self.cont3xt.ov_fields_selected = 0;
                                self.cont3xt.ov_fields_table_state.select(Some(0));
                            } else {
                                self.cont3xt.ov_level = C3OverviewLevel::List;
                            }
                        }
                        KeyCode::Char('/') => {
                            self.cont3xt.ov_fields_filtering = true;
                        }
                        KeyCode::Enter | KeyCode::Char('e') => {
                            if let Some(ri) = real_idx {
                                let ov_idx = self.cont3xt.ov_editor_idx;
                                if let Some(ov) = self.cont3xt.ov_list.get(ov_idx) {
                                    if let Some(field) = ov.fields.get(ri) {
                                        self.cont3xt.ov_field_editor_idx = ri;
                                        self.cont3xt.ov_field_editor_from = field.from.clone();
                                        self.cont3xt.ov_field_editor_is_custom = field.field_type == "custom";
                                        if self.cont3xt.ov_field_editor_is_custom {
                                            let custom_inner = field.custom.as_ref()
                                                .and_then(|v| v.get("custom"))
                                                .cloned()
                                                .unwrap_or(serde_json::json!({}));
                                            let json_str = serde_json::to_string_pretty(&custom_inner).unwrap_or_default();
                                            self.cont3xt.ov_fe_json_lines = json_str.lines().map(String::from).collect();
                                            if self.cont3xt.ov_fe_json_lines.is_empty() {
                                                self.cont3xt.ov_fe_json_lines.push(String::new());
                                            }
                                            self.cont3xt.ov_fe_json_line = 0;
                                            self.cont3xt.ov_fe_json_col = 0;
                                            self.cont3xt.ov_fe_json_scroll = 0;
                                            self.cont3xt.ov_field_editor_field_name.clear();
                                            self.cont3xt.ov_field_editor_label.clear();
                                        } else {
                                            self.cont3xt.ov_field_editor_field_name = field.field.clone();
                                            self.cont3xt.ov_field_editor_label = field.alias.clone().unwrap_or_default();
                                            self.cont3xt.ov_fe_json_lines.clear();
                                        }
                                        self.cont3xt.ov_field_editor_field = C3OvFieldEditorField::From;
                                        self.cont3xt.ov_field_editor_cursor = 0;
                                        self.cont3xt.ov_fe_popup_open = false;
                                        self.cont3xt.ov_level = C3OverviewLevel::FieldEditor;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('x') if is_editable => {
                            if let Some(ri) = real_idx {
                                let ov_idx = self.cont3xt.ov_editor_idx;
                                if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx) {
                                    if ri < ov.fields.len() {
                                        ov.fields.remove(ri);
                                        let new_filtered = self.c3_ov_filtered_fields();
                                        if self.cont3xt.ov_fields_selected >= new_filtered.len() && !new_filtered.is_empty() {
                                            self.cont3xt.ov_fields_selected = new_filtered.len() - 1;
                                        }
                                        self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('a') if !has_filter && is_editable => {
                            let ov_idx = self.cont3xt.ov_editor_idx;
                            if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx) {
                                let insert_pos = real_idx.map(|r| r + 1).unwrap_or(ov.fields.len()).min(ov.fields.len());
                                ov.fields.insert(insert_pos, crate::api::Cont3xtOverviewField {
                                    field_type: "linked".to_string(),
                                    from: String::new(),
                                    field: String::new(),
                                    alias: None,
                                    custom: None,
                                });
                                self.cont3xt.ov_fields_selected += 1;
                                self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                            }
                        }
                        KeyCode::Char('N') | KeyCode::Char('A') if !has_filter && is_editable => {
                            let ov_idx = self.cont3xt.ov_editor_idx;
                            if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx) {
                                ov.fields.push(crate::api::Cont3xtOverviewField {
                                    field_type: "linked".to_string(),
                                    from: String::new(),
                                    field: String::new(),
                                    alias: None,
                                    custom: None,
                                });
                                self.cont3xt.ov_fields_selected = ov.fields.len() - 1;
                                self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                            }
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) && !has_filter && is_editable => {
                            if let Some(ri) = real_idx {
                                let ov_idx = self.cont3xt.ov_editor_idx;
                                if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx) {
                                    if ri > 0 {
                                        ov.fields.swap(ri, ri - 1);
                                        self.cont3xt.ov_fields_selected = self.cont3xt.ov_fields_selected.saturating_sub(1);
                                        self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) && !has_filter && is_editable => {
                            if let Some(ri) = real_idx {
                                let ov_idx = self.cont3xt.ov_editor_idx;
                                if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx) {
                                    if ri + 1 < ov.fields.len() {
                                        ov.fields.swap(ri, ri + 1);
                                        self.cont3xt.ov_fields_selected += 1;
                                        self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(self.c3_ov_save())
                            });
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.cont3xt.ov_fields_selected + 1 < filtered.len() {
                                self.cont3xt.ov_fields_selected += 1;
                                self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.cont3xt.ov_fields_selected > 0 {
                                self.cont3xt.ov_fields_selected -= 1;
                                self.cont3xt.ov_fields_table_state.select(Some(self.cont3xt.ov_fields_selected));
                            }
                        }
                        KeyCode::Char('B') => {
                            let ov_idx = self.cont3xt.ov_editor_idx;
                            if let Some(ov) = self.cont3xt.ov_list.get(ov_idx) {
                                let safe_name = ov.name.replace(['/', '\\', ' '], "_");
                                self.cont3xt.backup_kind = C3BackupKind::OverviewSingle;
                                let fname = format!("{}.json", safe_name);
                                self.cont3xt.backup_cursor = fname.len();
                                self.cont3xt.backup_prompt = Some(fname);
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                        _ => {}
                    }
                    return true;
                }
                C3OverviewLevel::List => {
                    // List filter mode
                    if self.cont3xt.ov_filtering {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.cont3xt.ov_filtering = false;
                            }
                            _ => {
                                if handle_text_input_key(key.code, &mut self.cont3xt.ov_filter, &mut self.cont3xt.ov_filter_cursor) {
                                    self.cont3xt.ov_selected = 0;
                                    self.cont3xt.ov_table_state.select(Some(0));
                                }
                            }
                        }
                        return true;
                    }
                    // handled below in the main match
                }
            }
        false
    }

    pub(crate) fn c3_settings_filtered_views(&self) -> Vec<usize> {
        let filter = self.cont3xt.settings_views_filter.to_lowercase();
        let mut indices: Vec<usize> = self.cont3xt.settings_views.iter().enumerate()
            .filter(|(_, v)| filter.is_empty() || v.name.to_lowercase().contains(&filter) || v.creator.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let views = &self.cont3xt.settings_views;
        let col = self.cont3xt.settings_views_sort;
        let desc = self.cont3xt.settings_views_sort_desc;
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

    pub(crate) fn c3_save_view_editor(&mut self) {
        // Block save for non-editable views
        if let Some(ref id) = self.cont3xt.view_editor_id {
            if let Some(v) = self.cont3xt.settings_views.iter().find(|v| v.id == *id) {
                if !v.editable {
                    self.status_msg = "View is read-only".to_string();
                    return;
                }
            }
        }
        let name = self.cont3xt.view_editor_name.trim().to_string();
        if name.is_empty() {
            self.status_msg = "View name cannot be empty".to_string();
            return;
        }
        let integrations: Vec<String> = self.cont3xt.view_editor_integrations.iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(name, _)| name.clone())
            .collect();
        let view_roles: Vec<String> = self.cont3xt.view_editor_view_roles.iter()
            .filter(|(_, sel)| *sel)
            .map(|(r, _)| r.clone())
            .collect();
        let edit_roles: Vec<String> = self.cont3xt.view_editor_edit_roles.iter()
            .filter(|(_, sel)| *sel)
            .map(|(r, _)| r.clone())
            .collect();

        let result = if let Some(id) = &self.cont3xt.view_editor_id {
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
                let action = if self.cont3xt.view_editor_id.is_some() { "Updated" } else { "Created" };
                self.status_msg = format!("{action} view: {name}");
                self.cont3xt.view_editor_open = false;
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
