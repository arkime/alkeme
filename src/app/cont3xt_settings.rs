use super::*;
use crate::api::{Cont3xtView, IntegrationSettings};
use std::collections::HashMap;

impl App {
    pub async fn c3_fetch_settings_views(&mut self) {
        match self.client.c3_get_views().await {
            Ok(views) => {
                self.cont3xt.settings_views = views;
                self.cont3xt.settings_views_loaded = true;
                if self.cont3xt.settings_views_selected >= self.cont3xt.settings_views.len() {
                    self.cont3xt.settings_views_selected = 0;
                }
                self.cont3xt.settings_views_table_state.select(Some(self.cont3xt.settings_views_selected));
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
    }

    pub async fn c3_fetch_roles(&mut self) {
        match self.client.c3_get_roles().await {
            Ok(roles) => {
                self.cont3xt.all_roles = roles;
            }
            Err(e) => self.status_msg = format!("Error fetching roles: {e}"),
        }
    }

    pub async fn c3_fetch_integration_settings(&mut self) {
        match self.client.c3_get_integration_settings().await {
            Ok(settings) => {
                self.cont3xt.int_settings = settings;
                self.cont3xt.int_settings_loaded = true;
                if self.cont3xt.int_settings_selected >= self.cont3xt.int_settings.len() {
                    self.cont3xt.int_settings_selected = 0;
                }
                let filtered = self.c3_int_settings_filtered();
                if !filtered.is_empty() {
                    self.cont3xt.int_settings_table_state.select(Some(self.cont3xt.int_settings_selected.min(filtered.len() - 1)));
                } else {
                    self.cont3xt.int_settings_table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.status_msg = format!("Error fetching integration settings: {e}");
            }
        }
    }

    pub fn c3_build_int_settings_payload(&self) -> HashMap<String, HashMap<String, serde_json::Value>> {
        let mut payload: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        for int in &self.cont3xt.int_settings {
            let mut fields: HashMap<String, serde_json::Value> = HashMap::new();
            fields.insert("disabled".to_string(), serde_json::Value::Bool(int.disabled));
            for field_def in &int.fields {
                let val = int.values.get(&field_def.name).cloned().unwrap_or_default();
                if field_def.is_boolean {
                    fields.insert(field_def.name.clone(), serde_json::Value::Bool(val == "true"));
                } else {
                    fields.insert(field_def.name.clone(), serde_json::Value::String(val));
                }
            }
            payload.insert(int.name.clone(), fields);
        }
        payload
    }

    pub(crate) fn c3_int_settings_filtered(&self) -> Vec<usize> {
        let filter = self.cont3xt.int_settings_filter.to_lowercase();
        let mut indices: Vec<usize> = self.cont3xt.int_settings.iter().enumerate()
            .filter(|(_, s)| filter.is_empty() || s.name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let settings = &self.cont3xt.int_settings;
        let col = self.cont3xt.int_settings_sort;
        let desc = self.cont3xt.int_settings_sort_desc;
        indices.sort_by(|&a, &b| {
            let cmp = match col {
                0 => settings[a].name.to_lowercase().cmp(&settings[b].name.to_lowercase()),
                _ => {
                    // Sort by status: locked, disabled, global, configured
                    let status_rank = |s: &IntegrationSettings| -> u8 {
                        if s.locked { 0 }
                        else if s.disabled { 4 }
                        else if s.global_configed { 1 }
                        else if s.fields.iter().any(|f| f.required && s.values.get(&f.name).is_none_or(|v| v.is_empty())) { 3 }
                        else { 2 }
                    };
                    status_rank(&settings[a]).cmp(&status_rank(&settings[b]))
                        .then_with(|| settings[a].name.to_lowercase().cmp(&settings[b].name.to_lowercase()))
                }
            };
            if desc { cmp.reverse() } else { cmp }
        });
        indices
    }

    /// Open the view editor for a new view
    pub fn c3_open_new_view_editor(&mut self) {
        self.cont3xt.view_editor_open = true;
        self.cont3xt.view_editor_id = None;
        self.cont3xt.view_editor_name = String::new();
        self.cont3xt.view_editor_name_cursor = 0;
        self.cont3xt.view_editor_field = C3ViewEditorField::Name;
        self.cont3xt.view_editor_integration_filter = String::new();
        self.cont3xt.view_editor_integration_filtering = false;
        self.cont3xt.view_editor_integration_selected = 0;
        // All integrations unchecked
        self.cont3xt.view_editor_integrations = self.cont3xt.integrations.iter()
            .map(|i| (i.name.clone(), false)).collect();
        self.cont3xt.view_editor_view_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), false)).collect();
        self.cont3xt.view_editor_edit_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), false)).collect();
    }

    /// Open the view editor to edit an existing view
    pub fn c3_open_edit_view_editor(&mut self, view: &Cont3xtView) {
        self.cont3xt.view_editor_open = true;
        self.cont3xt.view_editor_id = Some(view.id.clone());
        self.cont3xt.view_editor_name = view.name.clone();
        self.cont3xt.view_editor_name_cursor = view.name.len();
        self.cont3xt.view_editor_field = C3ViewEditorField::Name;
        self.cont3xt.view_editor_integration_filter = String::new();
        self.cont3xt.view_editor_integration_filtering = false;
        self.cont3xt.view_editor_integration_selected = 0;
        self.cont3xt.view_editor_integrations = self.cont3xt.integrations.iter()
            .map(|i| (i.name.clone(), view.integrations.contains(&i.name))).collect();
        self.cont3xt.view_editor_view_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), view.view_roles.contains(r))).collect();
        self.cont3xt.view_editor_edit_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), view.edit_roles.contains(r))).collect();
    }

    /// Get filtered integrations for view editor
    pub fn c3_view_editor_filtered_integrations(&self) -> Vec<usize> {
        let filter = self.cont3xt.view_editor_integration_filter.to_lowercase();
        self.cont3xt.view_editor_integrations.iter().enumerate()
            .filter(|(_, (name, _))| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get filtered roles for role popup
    pub fn c3_role_popup_filtered_roles(&self) -> Vec<usize> {
        let filter = self.cont3xt.role_popup_filter.to_lowercase();
        let roles = if self.cont3xt.role_popup_for_edit {
            &self.cont3xt.view_editor_edit_roles
        } else {
            &self.cont3xt.view_editor_view_roles
        };
        roles.iter().enumerate()
            .filter(|(_, (name, _))| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// Filtered indices into c3_all_roles (for link group / overview role popups)
    pub fn c3_all_roles_filtered(&self) -> Vec<usize> {
        let filter = self.cont3xt.role_popup_filter.to_lowercase();
        self.cont3xt.all_roles.iter().enumerate()
            .filter(|(_, name)| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// Rebuild view editor role lists from c3_all_roles, preserving any already-selected roles
    pub fn c3_rebuild_view_editor_roles(&mut self) {
        let prev_view: std::collections::HashSet<String> = self.cont3xt.view_editor_view_roles.iter()
            .filter(|(_, sel)| *sel).map(|(n, _)| n.clone()).collect();
        let prev_edit: std::collections::HashSet<String> = self.cont3xt.view_editor_edit_roles.iter()
            .filter(|(_, sel)| *sel).map(|(n, _)| n.clone()).collect();
        self.cont3xt.view_editor_view_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), prev_view.contains(r))).collect();
        self.cont3xt.view_editor_edit_roles = self.cont3xt.all_roles.iter()
            .map(|r| (r.clone(), prev_edit.contains(r))).collect();
    }

    /// Get sorted integration names for the overview field editor From popup
    pub fn c3_ov_fe_integration_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.cont3xt.integrations.iter()
            .map(|i| i.name.clone())
            .collect();
        names.sort_by_key(|a| a.to_lowercase());
        names
    }

    /// Get field labels for the selected integration + "Custom" for the overview field editor Field popup
    pub fn c3_ov_fe_field_labels(&self) -> Vec<String> {
        let from = &self.cont3xt.ov_field_editor_from;
        let mut labels: Vec<String> = self.cont3xt.integrations.iter()
            .find(|i| i.name == *from)
            .and_then(|i| i.card.as_ref())
            .map(|c| c.fields.iter().map(|f| f.label.clone()).collect())
            .unwrap_or_default();
        labels.sort_by_key(|a| a.to_lowercase());
        labels.push("Custom".to_string());
        labels
    }

    /// Get filtered items for the overview field editor selector popup
    pub fn c3_ov_fe_popup_filtered(&self) -> Vec<usize> {
        let filter = self.cont3xt.ov_fe_popup_filter.to_lowercase();
        self.cont3xt.ov_fe_popup_items.iter().enumerate()
            .filter(|(_, name)| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    pub async fn c3_fetch_link_groups_settings(&mut self) {
        match self.client.c3_get_link_groups().await {
            Ok(groups) => {
                self.cont3xt.lg_groups = groups;
                self.cont3xt.lg_selected = 0;
                self.cont3xt.lg_table_state.select(Some(0));
                self.cont3xt.lg_loaded = true;
            }
            Err(e) => self.status_msg = format!("Link groups error: {e}"),
        }
    }

    pub fn c3_lg_build_group_json(&self, group: &crate::api::Cont3xtLinkGroup) -> serde_json::Value {
        let links: Vec<serde_json::Value> = group.links.iter().map(|l| {
            let mut link = serde_json::json!({
                "name": l.name,
                "url": l.url,
                "itypes": l.itypes,
            });
            if !l.info.is_empty() { link["infoField"] = serde_json::json!(l.info); }
            if !l.color.is_empty() { link["color"] = serde_json::json!(l.color); }
            if !l.external_doc_name.is_empty() { link["externalDocName"] = serde_json::json!(l.external_doc_name); }
            if !l.external_doc_url.is_empty() { link["externalDocUrl"] = serde_json::json!(l.external_doc_url); }
            link
        }).collect();
        serde_json::json!({
            "name": group.name,
            "links": links,
            "viewRoles": group.view_roles,
            "editRoles": group.edit_roles,
        })
    }

    pub fn c3_save_backup(&mut self, filename: &str, kind: C3BackupKind) {
        let json = match kind {
            C3BackupKind::LinkGroupsAll => {
                let groups: Vec<serde_json::Value> = self.cont3xt.lg_groups.iter()
                    .map(|g| self.c3_lg_build_group_json(g))
                    .collect();
                serde_json::json!({ "linkGroups": groups })
            }
            C3BackupKind::LinkGroupSingle => {
                let idx = self.cont3xt.lg_editing_group_idx;
                match self.cont3xt.lg_groups.get(idx) {
                    Some(g) => self.c3_lg_build_group_json(g),
                    None => {
                        self.status_msg = "No group selected".to_string();
                        return;
                    }
                }
            }
            C3BackupKind::Integrations => {
                let settings: Vec<serde_json::Value> = self.cont3xt.int_settings.iter().map(|int| {
                    let values: serde_json::Map<String, serde_json::Value> = int.values.iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                        .collect();
                    serde_json::json!({
                        "name": int.name,
                        "disabled": int.disabled,
                        "globalConfiged": int.global_configed,
                        "locked": int.locked,
                        "values": values,
                    })
                }).collect();
                serde_json::json!({ "integrations": settings })
            }
            C3BackupKind::Views => {
                let views: Vec<serde_json::Value> = self.cont3xt.settings_views.iter().map(|v| {
                    serde_json::json!({
                        "id": v.id,
                        "name": v.name,
                        "integrations": v.integrations,
                        "creator": v.creator,
                        "viewRoles": v.view_roles,
                        "editRoles": v.edit_roles,
                    })
                }).collect();
                serde_json::json!({ "views": views })
            }
            C3BackupKind::OverviewsAll => {
                let overviews: Vec<serde_json::Value> = self.cont3xt.ov_list.iter()
                    .map(|o| self.c3_ov_build_json(o))
                    .collect();
                serde_json::json!({ "overviews": overviews })
            }
            C3BackupKind::OverviewSingle => {
                let idx = self.cont3xt.ov_editor_idx;
                match self.cont3xt.ov_list.get(idx) {
                    Some(o) => self.c3_ov_build_json(o),
                    None => {
                        self.status_msg = "No overview selected".to_string();
                        return;
                    }
                }
            }
        };

        let desc = kind.title().trim();
        match serde_json::to_string_pretty(&json) {
            Ok(data) => {
                match std::fs::write(filename, &data) {
                    Ok(_) => self.status_msg = format!("{} saved to {}", desc, filename),
                    Err(e) => self.status_msg = format!("Error writing {}: {}", filename, e),
                }
            }
            Err(e) => self.status_msg = format!("JSON error: {}", e),
        }
    }

    pub fn c3_lg_filtered_groups(&self) -> Vec<usize> {
        let filter = self.cont3xt.lg_filter.to_lowercase();
        let mut indices: Vec<usize> = self.cont3xt.lg_groups.iter().enumerate()
            .filter(|(_, g)| filter.is_empty() || g.name.to_lowercase().contains(&filter) || g.creator.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let groups = &self.cont3xt.lg_groups;
        let col = self.cont3xt.lg_sort_col;
        let desc = self.cont3xt.lg_sort_desc;
        indices.sort_by(|&a, &b| {
            let cmp = match col {
                0 => groups[a].name.to_lowercase().cmp(&groups[b].name.to_lowercase()),
                1 => groups[a].creator.to_lowercase().cmp(&groups[b].creator.to_lowercase()),
                2 => groups[a].links.len().cmp(&groups[b].links.len()),
                _ => groups[a].editable.cmp(&groups[b].editable),
            };
            if desc { cmp.reverse() } else { cmp }
        });
        indices
    }

    pub fn c3_lg_filtered_links(&self) -> Vec<usize> {
        let gi = self.cont3xt.lg_editing_group_idx;
        let group = match self.cont3xt.lg_groups.get(gi) {
            Some(g) => g,
            None => return Vec::new(),
        };
        let filter = self.cont3xt.lg_links_filter.to_lowercase();
        group.links.iter().enumerate()
            .filter(|(_, l)| {
                filter.is_empty()
                    || l.name.to_lowercase().contains(&filter)
                    || l.url.to_lowercase().contains(&filter)
                    || l.itypes.iter().any(|t| t.to_lowercase().contains(&filter))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn c3_ov_filtered_list(&self) -> Vec<usize> {
        let filter = self.cont3xt.ov_filter.to_lowercase();
        let mut indices: Vec<usize> = self.cont3xt.ov_list.iter().enumerate()
            .filter(|(_, o)| filter.is_empty()
                || o.name.to_lowercase().contains(&filter)
                || o.itype.to_lowercase().contains(&filter)
                || o.creator.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let list = &self.cont3xt.ov_list;
        let col = self.cont3xt.ov_sort_col;
        let desc = self.cont3xt.ov_sort_desc;
        indices.sort_by(|&a, &b| {
            let cmp = match col {
                0 => list[a].name.to_lowercase().cmp(&list[b].name.to_lowercase()),
                1 => list[a].itype.to_lowercase().cmp(&list[b].itype.to_lowercase()),
                2 => list[a].is_default.cmp(&list[b].is_default),
                3 => list[a].creator.to_lowercase().cmp(&list[b].creator.to_lowercase()),
                _ => list[a].fields.len().cmp(&list[b].fields.len()),
            };
            if desc { cmp.reverse() } else { cmp }
        });
        indices
    }

    pub fn c3_ov_filtered_fields(&self) -> Vec<usize> {
        let idx = self.cont3xt.ov_editor_idx;
        let ov = match self.cont3xt.ov_list.get(idx) {
            Some(o) => o,
            None => return Vec::new(),
        };
        let filter = self.cont3xt.ov_fields_filter.to_lowercase();
        ov.fields.iter().enumerate()
            .filter(|(_, f)| {
                filter.is_empty()
                    || f.from.to_lowercase().contains(&filter)
                    || f.field.to_lowercase().contains(&filter)
                    || f.alias.as_deref().unwrap_or("").to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn c3_ov_build_json(&self, ov: &crate::api::Cont3xtOverview) -> serde_json::Value {
        let fields: Vec<serde_json::Value> = ov.fields.iter().map(|f| {
            if let Some(ref custom) = f.custom {
                custom.clone()
            } else {
                let mut obj = serde_json::json!({
                    "type": f.field_type,
                    "from": f.from,
                    "field": f.field,
                });
                if let Some(ref alias) = f.alias {
                    obj["alias"] = serde_json::json!(alias);
                }
                obj
            }
        }).collect();
        serde_json::json!({
            "name": ov.name,
            "title": ov.title,
            "iType": ov.itype,
            "fields": fields,
            "viewRoles": ov.view_roles,
            "editRoles": ov.edit_roles,
        })
    }

    pub async fn c3_ov_save(&mut self) {
        let idx = self.cont3xt.ov_editor_idx;
        // Block save for non-editable overviews
        if let Some(ov) = self.cont3xt.ov_list.get(idx)
            && !ov.editable {
                self.status_msg = "Overview is read-only".to_string();
                return;
            }
        // Apply editor fields to the overview
        if let Some(ov) = self.cont3xt.ov_list.get_mut(idx) {
            ov.name = self.cont3xt.ov_editor_name.clone();
            ov.title = self.cont3xt.ov_editor_title.clone();
            ov.itype = self.cont3xt.ov_editor_itype.clone();
            ov.view_roles = self.cont3xt.ov_editor_view_roles.clone();
            ov.edit_roles = self.cont3xt.ov_editor_edit_roles.clone();
        }
        // Build JSON from (now updated) overview
        let (id, json) = match self.cont3xt.ov_list.get(idx) {
            Some(ov) => (ov.id.clone(), self.c3_ov_build_json(ov)),
            None => return,
        };
        match self.client.c3_update_overview(&id, &json).await {
            Ok(_) => self.status_msg = "Overview saved".to_string(),
            Err(e) => self.status_msg = format!("Error saving overview: {e}"),
        }
    }

    /// Save the field editor contents back to the overview field and persist
    pub fn c3_ov_fe_save_field(&mut self) {
        let ov_idx = self.cont3xt.ov_editor_idx;
        let is_editable = self.cont3xt.ov_list.get(ov_idx)
            .map(|ov| ov.editable).unwrap_or(false);
        if !is_editable {
            self.status_msg = "Overview is not editable".to_string();
            return;
        }
        let fi = self.cont3xt.ov_field_editor_idx;
        if let Some(ov) = self.cont3xt.ov_list.get_mut(ov_idx)
            && let Some(field) = ov.fields.get_mut(fi) {
                field.from = self.cont3xt.ov_field_editor_from.clone();
                if self.cont3xt.ov_field_editor_is_custom {
                    let json_str = self.cont3xt.ov_fe_json_lines.join("\n");
                    match serde_json::from_str::<serde_json::Value>(&json_str) {
                        Ok(custom_inner) => {
                            field.field_type = "custom".to_string();
                            field.field.clear();
                            field.alias = None;
                            field.custom = Some(serde_json::json!({
                                "from": field.from,
                                "custom": custom_inner,
                            }));
                        }
                        Err(e) => {
                            self.status_msg = format!("Invalid JSON: {e}");
                            return;
                        }
                    }
                } else {
                    field.field_type = "linked".to_string();
                    field.field = self.cont3xt.ov_field_editor_field_name.clone();
                    let label = self.cont3xt.ov_field_editor_label.trim().to_string();
                    field.alias = if label.is_empty() { None } else { Some(label) };
                    field.custom = None;
                }
            }
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.c3_ov_save())
        });
        self.cont3xt.ov_level = C3OverviewLevel::FieldList;
    }

    pub async fn c3_ov_create(&mut self) {
        let json = serde_json::json!({
            "name": "New Overview",
            "title": "",
            "iType": "domain",
            "fields": [],
            "viewRoles": [],
            "editRoles": [],
        });
        match self.client.c3_create_overview(&json).await {
            Ok(resp) => {
                self.status_msg = "Overview created".to_string();
                // Extract new ID from response
                if let Some(id) = resp.get("overview").and_then(|o| o.get("_id")).and_then(|v| v.as_str()) {
                    self.cont3xt.ov_list.push(crate::api::Cont3xtOverview {
                        id: id.to_string(),
                        name: "New Overview".to_string(),
                        title: String::new(),
                        itype: "domain".to_string(),
                        is_default: false,
                        creator: String::new(),
                        editable: true,
                        view_roles: Vec::new(),
                        edit_roles: Vec::new(),
                        fields: Vec::new(),
                    });
                    // Select the new overview
                    let filtered = self.c3_ov_filtered_list();
                    if let Some(pos) = filtered.iter().position(|&i| i == self.cont3xt.ov_list.len() - 1) {
                        self.cont3xt.ov_selected = pos;
                        self.cont3xt.ov_table_state.select(Some(pos));
                    }
                } else {
                    self.c3_fetch_overviews().await;
                }
            }
            Err(e) => self.status_msg = format!("Error creating overview: {e}"),
        }
    }

    pub async fn c3_ov_delete(&mut self) {
        let filtered = self.c3_ov_filtered_list();
        let real_idx = match filtered.get(self.cont3xt.ov_selected) {
            Some(&i) => i,
            None => return,
        };
        let id = self.cont3xt.ov_list[real_idx].id.clone();
        match self.client.c3_delete_overview(&id).await {
            Ok(_) => {
                self.cont3xt.ov_list.remove(real_idx);
                let new_filtered = self.c3_ov_filtered_list();
                if self.cont3xt.ov_selected >= new_filtered.len() {
                    self.cont3xt.ov_selected = new_filtered.len().saturating_sub(1);
                }
                self.cont3xt.ov_table_state.select(Some(self.cont3xt.ov_selected));
                self.status_msg = "Overview deleted".to_string();
            }
            Err(e) => self.status_msg = format!("Error deleting overview: {e}"),
        }
    }

    pub fn c3_lg_editor_field_value(&self) -> &str {
        match self.cont3xt.lg_editor_field {
            C3LinkEditorField::Name => &self.cont3xt.lg_editor_link.name,
            C3LinkEditorField::Url => &self.cont3xt.lg_editor_link.url,
            C3LinkEditorField::Color => &self.cont3xt.lg_editor_link.color,
            C3LinkEditorField::InfoField => &self.cont3xt.lg_editor_link.info,
            C3LinkEditorField::ExternalDocName => &self.cont3xt.lg_editor_link.external_doc_name,
            C3LinkEditorField::ExternalDocUrl => &self.cont3xt.lg_editor_link.external_doc_url,
            C3LinkEditorField::Itypes => "",
        }
    }

    pub fn c3_lg_editor_field_value_mut(&mut self) -> &mut String {
        match self.cont3xt.lg_editor_field {
            C3LinkEditorField::Name => &mut self.cont3xt.lg_editor_link.name,
            C3LinkEditorField::Url => &mut self.cont3xt.lg_editor_link.url,
            C3LinkEditorField::Color => &mut self.cont3xt.lg_editor_link.color,
            C3LinkEditorField::InfoField => &mut self.cont3xt.lg_editor_link.info,
            C3LinkEditorField::ExternalDocName => &mut self.cont3xt.lg_editor_link.external_doc_name,
            C3LinkEditorField::ExternalDocUrl => &mut self.cont3xt.lg_editor_link.external_doc_url,
            C3LinkEditorField::Itypes => &mut self.cont3xt.lg_editor_link.name, // unreachable in practice
        }
    }
}
