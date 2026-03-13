use super::*;
use crate::api::{Cont3xtIntegration, Cont3xtView, IntegrationSettings, parse_card};
use std::collections::HashMap;

impl App {

    pub async fn c3_fetch_integrations(&mut self) {
        match self.client.c3_get_integrations().await {
            Ok(val) => {
                let mut integrations = Vec::new();
                if let Some(obj) = val.get("integrations").and_then(|v| v.as_object()) {
                    for (name, info) in obj {
                        let doable = info.get("doable").and_then(|v| v.as_bool()).unwrap_or(false);
                        let order = info.get("order").and_then(|v| v.as_u64()).unwrap_or(10000) as u32;
                        let card = info.get("card").and_then(|c| parse_card(c));
                        integrations.push(Cont3xtIntegration {
                            name: name.clone(),
                            doable,
                            order,
                            card,
                        });
                    }
                }
                integrations.sort_by(|a, b| a.order.cmp(&b.order).then(a.name.cmp(&b.name)));
                self.c3_integrations = integrations;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching integrations: {e}");
            }
        }
    }

    pub fn c3_request_search(&mut self) {
        if self.expression.is_empty() {
            return;
        }
        self.c3_loaded_file = None;
        self.c3_pending_search = true;
    }

    pub async fn c3_fetch_views(&mut self) {
        match self.client.c3_get_views().await {
            Ok(views) => {
                self.c3_views = views;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
    }

    pub async fn c3_fetch_overviews(&mut self) {
        match self.client.c3_get_overviews().await {
            Ok(mut overviews) => {
                // Fetch selectedOverviews from settings to mark correct defaults
                if let Ok(selected) = self.client.c3_get_selected_overviews().await {
                    for ov in &mut overviews {
                        let itype_lower = ov.itype.to_lowercase();
                        if let Some(default_id) = selected.get(&itype_lower).or_else(|| selected.get(&ov.itype)) {
                            ov.is_default = ov.id == *default_id;
                        }
                    }
                }
                self.c3_overviews = overviews;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching overviews: {e}");
            }
        }
    }

    /// Get the list of currently enabled integration names
    pub fn c3_enabled_integration_names(&self) -> Vec<String> {
        self.c3_integrations.iter()
            .filter(|i| !self.c3_disabled_integrations.contains(&i.name))
            .map(|i| i.name.clone())
            .collect()
    }

    /// Apply a view: set disabled integrations to everything NOT in the view's list
    pub fn c3_apply_view(&mut self, integrations: &[String]) {
        self.c3_disabled_integrations.clear();
        for int in &self.c3_integrations {
            if !integrations.contains(&int.name) {
                self.c3_disabled_integrations.insert(int.name.clone());
            }
        }
    }

    pub async fn c3_fetch_stats(&mut self) {
        match self.client.c3_get_stats().await {
            Ok(val) => {
                if let Some(stats) = val.get("stats").and_then(|v| v.as_array()) {
                    self.c3_stats_data = stats.clone();
                }
                if let Some(itype_stats) = val.get("itypeStats").and_then(|v| v.as_array()) {
                    self.c3_itype_stats_data = itype_stats.clone();
                }
            }
            Err(e) => {
                self.status_msg = format!("Error fetching stats: {e}");
            }
        }
    }

    pub async fn c3_fetch_settings_views(&mut self) {
        match self.client.c3_get_views().await {
            Ok(views) => {
                self.c3_settings_views = views;
                self.c3_settings_views_loaded = true;
                if self.c3_settings_views_selected >= self.c3_settings_views.len() {
                    self.c3_settings_views_selected = 0;
                }
                self.c3_settings_views_table_state.select(Some(self.c3_settings_views_selected));
            }
            Err(e) => {
                self.status_msg = format!("Error fetching views: {e}");
            }
        }
    }

    pub async fn c3_fetch_roles(&mut self) {
        match self.client.c3_get_roles().await {
            Ok(roles) => self.c3_all_roles = roles,
            Err(e) => self.status_msg = format!("Error fetching roles: {e}"),
        }
    }

    pub async fn c3_fetch_integration_settings(&mut self) {
        match self.client.c3_get_integration_settings().await {
            Ok(settings) => {
                self.c3_int_settings = settings;
                self.c3_int_settings_loaded = true;
                if self.c3_int_settings_selected >= self.c3_int_settings.len() {
                    self.c3_int_settings_selected = 0;
                }
                let filtered = self.c3_int_settings_filtered();
                if !filtered.is_empty() {
                    self.c3_int_settings_table_state.select(Some(self.c3_int_settings_selected.min(filtered.len() - 1)));
                } else {
                    self.c3_int_settings_table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.status_msg = format!("Error fetching integration settings: {e}");
            }
        }
    }

    pub fn c3_build_int_settings_payload(&self) -> HashMap<String, HashMap<String, serde_json::Value>> {
        let mut payload: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        for int in &self.c3_int_settings {
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
        let filter = self.c3_int_settings_filter.to_lowercase();
        let mut indices: Vec<usize> = self.c3_int_settings.iter().enumerate()
            .filter(|(_, s)| filter.is_empty() || s.name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let settings = &self.c3_int_settings;
        let col = self.c3_int_settings_sort;
        let desc = self.c3_int_settings_sort_desc;
        indices.sort_by(|&a, &b| {
            let cmp = match col {
                0 => settings[a].name.to_lowercase().cmp(&settings[b].name.to_lowercase()),
                _ => {
                    // Sort by status: locked, disabled, global, configured
                    let status_rank = |s: &IntegrationSettings| -> u8 {
                        if s.locked { 0 }
                        else if s.disabled { 4 }
                        else if s.global_configed { 1 }
                        else if s.fields.iter().any(|f| f.required && s.values.get(&f.name).map_or(true, |v| v.is_empty())) { 3 }
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
        self.c3_view_editor_open = true;
        self.c3_view_editor_id = None;
        self.c3_view_editor_name = String::new();
        self.c3_view_editor_name_cursor = 0;
        self.c3_view_editor_field = C3ViewEditorField::Name;
        self.c3_view_editor_integration_filter = String::new();
        self.c3_view_editor_integration_filtering = false;
        self.c3_view_editor_integration_selected = 0;
        // All integrations unchecked
        self.c3_view_editor_integrations = self.c3_integrations.iter()
            .map(|i| (i.name.clone(), false)).collect();
        self.c3_view_editor_view_roles = self.c3_all_roles.iter()
            .map(|r| (r.clone(), false)).collect();
        self.c3_view_editor_edit_roles = self.c3_all_roles.iter()
            .map(|r| (r.clone(), false)).collect();
    }

    /// Open the view editor to edit an existing view
    pub fn c3_open_edit_view_editor(&mut self, view: &Cont3xtView) {
        self.c3_view_editor_open = true;
        self.c3_view_editor_id = Some(view.id.clone());
        self.c3_view_editor_name = view.name.clone();
        self.c3_view_editor_name_cursor = view.name.len();
        self.c3_view_editor_field = C3ViewEditorField::Name;
        self.c3_view_editor_integration_filter = String::new();
        self.c3_view_editor_integration_filtering = false;
        self.c3_view_editor_integration_selected = 0;
        self.c3_view_editor_integrations = self.c3_integrations.iter()
            .map(|i| (i.name.clone(), view.integrations.contains(&i.name))).collect();
        self.c3_view_editor_view_roles = self.c3_all_roles.iter()
            .map(|r| (r.clone(), view.view_roles.contains(r))).collect();
        self.c3_view_editor_edit_roles = self.c3_all_roles.iter()
            .map(|r| (r.clone(), view.edit_roles.contains(r))).collect();
    }

    /// Get filtered integrations for view editor
    pub fn c3_view_editor_filtered_integrations(&self) -> Vec<usize> {
        let filter = self.c3_view_editor_integration_filter.to_lowercase();
        self.c3_view_editor_integrations.iter().enumerate()
            .filter(|(_, (name, _))| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get filtered roles for role popup
    pub fn c3_role_popup_filtered_roles(&self) -> Vec<usize> {
        let filter = self.c3_role_popup_filter.to_lowercase();
        let roles = if self.c3_role_popup_for_edit {
            &self.c3_view_editor_edit_roles
        } else {
            &self.c3_view_editor_view_roles
        };
        roles.iter().enumerate()
            .filter(|(_, (name, _))| filter.is_empty() || name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn c3_stats_current_data(&self) -> &Vec<serde_json::Value> {
        match self.c3_stats_tab {
            C3StatsTab::Integrations => &self.c3_stats_data,
            C3StatsTab::ITypes => &self.c3_itype_stats_data,
        }
    }

    pub fn c3_history_sort_field(&self) -> &str {
        C3_HISTORY_COLUMNS.get(self.c3_history_sort_col).map(|c| c.0).unwrap_or("issuedAt")
    }

    pub fn c3_history_filtered_len(&self) -> usize {
        if self.c3_history_filter.is_empty() {
            self.c3_history_data.len()
        } else {
            let filter_lower = self.c3_history_filter.to_lowercase();
            self.c3_history_data.iter().filter(|item| {
                item.get("indicator").and_then(|v| v.as_str()).unwrap_or("")
                    .to_lowercase().contains(&filter_lower)
                || item.get("iType").and_then(|v| v.as_str()).unwrap_or("")
                    .to_lowercase().contains(&filter_lower)
                || item.get("tags").and_then(|v| v.as_array())
                    .map(|a| a.iter().any(|t| t.as_str().unwrap_or("").to_lowercase().contains(&filter_lower)))
                    .unwrap_or(false)
            }).count()
        }
    }

    pub async fn c3_fetch_history(&mut self) {
        let sort_by = self.c3_history_sort_field().to_string();
        let sort_order = if self.c3_history_sort_desc { "desc" } else { "asc" };
        match self.client.c3_get_audits(&sort_by, sort_order, self.c3_history_page, 100).await {
            Ok(val) => {
                if let Some(audits) = val.get("audits").and_then(|v| v.as_array()) {
                    self.c3_history_data = audits.clone();
                }
                self.c3_history_total = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                self.c3_history_loaded = true;
                self.c3_history_selected = 0;
                self.c3_history_table_state.select(Some(0));
            }
            Err(e) => {
                self.status_msg = format!("Error fetching history: {e}");
            }
        }
    }

    pub async fn c3_delete_history(&mut self, id: &str) {
        match self.client.c3_delete_audit(id).await {
            Ok(val) => {
                if val.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                    self.c3_history_data.retain(|item| {
                        item.get("_id").and_then(|v| v.as_str()).unwrap_or("") != id
                    });
                    self.c3_history_total = self.c3_history_total.saturating_sub(1);
                    if self.c3_history_selected >= self.c3_history_data.len() && self.c3_history_selected > 0 {
                        self.c3_history_selected -= 1;
                    }
                    self.c3_history_table_state.select(Some(self.c3_history_selected));
                    self.status_msg = "History entry deleted".to_string();
                } else {
                    let text = val.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    self.status_msg = format!("Delete failed: {text}");
                }
            }
            Err(e) => {
                self.status_msg = format!("Error deleting history: {e}");
            }
        }
    }

    pub fn c3_save_json(&mut self, filename: &str) {
        // Build a combined JSON object from all results
        let mut combined = serde_json::Map::new();

        // Add _cont3xt metadata
        let mut meta = serde_json::Map::new();
        meta.insert("query".to_string(), serde_json::Value::String(self.expression.clone()));
        meta.insert("itype".to_string(), serde_json::Value::String(self.c3_search_itype.clone()));
        if !self.c3_tags.is_empty() {
            meta.insert("tags".to_string(), serde_json::json!(self.c3_tags));
        }
        meta.insert("init_indicators".to_string(), serde_json::Value::Array(
            self.c3_init_indicators.iter()
                .map(|(itype, query)| serde_json::json!([itype, query]))
                .collect(),
        ));
        // Parent-child relationships: key="indicator\titype", value=[[parent_query, parent_itype], ...]
        let mut parents = serde_json::Map::new();
        for ((indicator, itype), parent_list) in &self.c3_indicator_parents {
            let key = format!("{}\t{}", indicator, itype);
            parents.insert(key, serde_json::Value::Array(
                parent_list.iter()
                    .map(|(pq, pi)| serde_json::json!([pq, pi]))
                    .collect(),
            ));
        }
        meta.insert("parents".to_string(), serde_json::Value::Object(parents));
        combined.insert("_cont3xt".to_string(), serde_json::Value::Object(meta));

        for result in &self.c3_results {
            if result.data.is_null() { continue; }
            let indicator_obj = combined.entry(&result.indicator)
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let serde_json::Value::Object(map) = indicator_obj {
                // Store itype alongside integration data
                map.insert(format!("_cont3xt_itype"), serde_json::Value::String(result.itype.clone()));
                map.insert(result.name.clone(), result.data.clone());
            }
        }
        let json = serde_json::Value::Object(combined);
        match serde_json::to_string_pretty(&json) {
            Ok(text) => {
                match std::fs::write(filename, &text) {
                    Ok(_) => self.status_msg = format!("JSON saved to {} ({} bytes)", filename, text.len()),
                    Err(e) => self.status_msg = format!("Error writing {}: {e}", filename),
                }
            }
            Err(e) => self.status_msg = format!("Error serializing JSON: {e}"),
        }
    }

    /// Load cont3xt results from a JSON file saved by c3_save_json
    pub fn c3_load_json(&mut self, filename: &str) -> Result<(), String> {
        let text = std::fs::read_to_string(filename)
            .map_err(|e| format!("Error reading {filename}: {e}"))?;
        let root: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("Error parsing JSON: {e}"))?;
        let obj = root.as_object()
            .ok_or_else(|| "JSON root is not an object".to_string())?;

        // Parse _cont3xt metadata
        if let Some(meta) = obj.get("_cont3xt").and_then(|v| v.as_object()) {
            if let Some(query) = meta.get("query").and_then(|v| v.as_str()) {
                self.expression = query.to_string();
                self.expression_edit = query.to_string();
            }
            if let Some(itype) = meta.get("itype").and_then(|v| v.as_str()) {
                self.c3_search_itype = itype.to_string();
            }
            if let Some(tags) = meta.get("tags").and_then(|v| v.as_array()) {
                self.c3_tags = tags.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
            if let Some(inits) = meta.get("init_indicators").and_then(|v| v.as_array()) {
                self.c3_init_indicators = inits.iter()
                    .filter_map(|v| v.as_array())
                    .filter_map(|arr| {
                        let itype = arr.first()?.as_str()?;
                        let query = arr.get(1)?.as_str()?;
                        Some((itype.to_string(), query.to_string()))
                    })
                    .collect();
            }
            if let Some(parents_obj) = meta.get("parents").and_then(|v| v.as_object()) {
                for (key, val) in parents_obj {
                    if let Some((indicator, itype)) = key.split_once('\t') {
                        let parent_list: Vec<(String, String)> = val.as_array()
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_array())
                                .filter_map(|a| {
                                    let pq = a.first()?.as_str()?;
                                    let pi = a.get(1)?.as_str()?;
                                    Some((pq.to_string(), pi.to_string()))
                                })
                                .collect())
                            .unwrap_or_default();
                        self.c3_indicator_parents.insert(
                            (indicator.to_string(), itype.to_string()),
                            parent_list,
                        );
                    }
                }
            }
        }

        // Parse results: each top-level key (except _cont3xt) is an indicator
        for (indicator, indicator_val) in obj {
            if indicator == "_cont3xt" { continue; }
            if let Some(integ_map) = indicator_val.as_object() {
                let itype = integ_map.get("_cont3xt_itype")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                for (integ_name, data) in integ_map {
                    if integ_name.starts_with("_cont3xt") { continue; }
                    self.c3_results.push(crate::api::Cont3xtResult {
                        name: integ_name.clone(),
                        indicator: indicator.clone(),
                        itype: itype.clone(),
                        data: data.clone(),
                        has_data: !data.is_null(),
                    });
                }
            }
        }

        self.c3_focus = Cont3xtFocus::Results;
        self.c3_loaded_file = Some(filename.to_string());
        let count = self.c3_results.len();
        self.status_msg = format!("Loaded {count} results from {filename}");
        Ok(())
    }

    pub async fn c3_fetch_link_groups(&mut self) {
        match self.client.c3_get_link_groups().await {
            Ok(groups) => {
                self.c3_link_groups = groups;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching link groups: {e}");
            }
        }
    }

    pub async fn c3_fetch_link_groups_settings(&mut self) {
        match self.client.c3_get_link_groups().await {
            Ok(groups) => {
                self.c3_lg_groups = groups;
                self.c3_lg_selected = 0;
                self.c3_lg_table_state.select(Some(0));
                self.c3_lg_loaded = true;
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
                let groups: Vec<serde_json::Value> = self.c3_lg_groups.iter()
                    .map(|g| self.c3_lg_build_group_json(g))
                    .collect();
                serde_json::json!({ "linkGroups": groups })
            }
            C3BackupKind::LinkGroupSingle => {
                let idx = self.c3_lg_editing_group_idx;
                match self.c3_lg_groups.get(idx) {
                    Some(g) => self.c3_lg_build_group_json(g),
                    None => {
                        self.status_msg = "No group selected".to_string();
                        return;
                    }
                }
            }
            C3BackupKind::Integrations => {
                let settings: Vec<serde_json::Value> = self.c3_int_settings.iter().map(|int| {
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
                let views: Vec<serde_json::Value> = self.c3_settings_views.iter().map(|v| {
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
        let filter = self.c3_lg_filter.to_lowercase();
        let mut indices: Vec<usize> = self.c3_lg_groups.iter().enumerate()
            .filter(|(_, g)| filter.is_empty() || g.name.to_lowercase().contains(&filter) || g.creator.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let groups = &self.c3_lg_groups;
        let col = self.c3_lg_sort_col;
        let desc = self.c3_lg_sort_desc;
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
        let gi = self.c3_lg_editing_group_idx;
        let group = match self.c3_lg_groups.get(gi) {
            Some(g) => g,
            None => return Vec::new(),
        };
        let filter = self.c3_lg_links_filter.to_lowercase();
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
    /// Get the integration name of the currently selected tree item (if it's a Result)
    pub fn c3_current_integration_name(&self) -> Option<String> {
        if let Some(C3TreeItem::Result(idx)) = self.c3_tree_order.get(self.c3_selected) {
            self.c3_results.get(*idx).map(|r| r.name.clone())
        } else {
            None
        }
    }

    pub fn c3_build_link_flat(&mut self) {
        // Use the selected result's itype and indicator
        let (itype, indicator) = match self.c3_tree_order.get(self.c3_selected) {
            Some(C3TreeItem::Result(idx)) => {
                if let Some(result) = self.c3_results.get(*idx) {
                    (result.itype.clone(), result.indicator.clone())
                } else {
                    (self.c3_search_itype.clone(), self.expression.clone())
                }
            }
            Some(C3TreeItem::Indicator(itype, query)) => {
                (itype.clone(), query.clone())
            }
            None => (self.c3_search_itype.clone(), self.expression.clone()),
        };

        // Collect indicators by itype for ${array,...}
        // "top" = indicators from the init packet (what the user searched for)
        let mut top_indicators_by_itype: HashMap<String, Vec<String>> = HashMap::new();
        for (it, query) in &self.c3_init_indicators {
            let entries = top_indicators_by_itype.entry(it.clone()).or_default();
            if !entries.contains(query) {
                entries.push(query.clone());
            }
        }
        // "all" = all unique indicators in the results tree (init + discovered children)
        let mut all_indicators_by_itype: HashMap<String, Vec<String>> = HashMap::new();
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        for (it, query) in &self.c3_init_indicators {
            if seen.insert((it.clone(), query.clone())) {
                all_indicators_by_itype.entry(it.clone()).or_default().push(query.clone());
            }
        }
        for result in &self.c3_results {
            if !result.indicator.is_empty() && seen.insert((result.itype.clone(), result.indicator.clone())) {
                all_indicators_by_itype.entry(result.itype.clone()).or_default().push(result.indicator.clone());
            }
        }

        let now = self.c3_stop_date;
        let start = self.c3_start_date;
        let filter = self.c3_link_popup_filter.to_lowercase();
        self.c3_link_flat.clear();
        for group in &self.c3_link_groups {
            for link in &group.links {
                if link.is_separator() {
                    continue;
                }
                if !link.itypes.iter().any(|t| *t == itype) {
                    continue;
                }
                if !filter.is_empty() {
                    let gn = group.name.to_lowercase();
                    let ln = link.name.to_lowercase();
                    if !gn.contains(&filter) && !ln.contains(&filter) {
                        continue;
                    }
                }
                let url = substitute_link_url(
                    &link.url, &indicator, &itype, start, now,
                    &all_indicators_by_itype, &top_indicators_by_itype,
                );
                self.c3_link_flat.push((group.name.clone(), link.name.clone(), url, link.info.clone(), link.color.clone()));
            }
        }
        if self.c3_link_popup_selected >= self.c3_link_flat.len() {
            self.c3_link_popup_selected = self.c3_link_flat.len().saturating_sub(1);
        }
    }


    /// Get the currently selected overview from the filtered+sorted list
    pub fn c3_overview_filtered_get(&self) -> Option<crate::api::Cont3xtOverview> {
        let itype = match self.c3_tree_order.get(self.c3_selected) {
            Some(C3TreeItem::Indicator(itype, _)) => itype.to_lowercase(),
            _ => return None,
        };
        let filter_lower = self.c3_overview_popup_filter.to_lowercase();
        let mut matching: Vec<&crate::api::Cont3xtOverview> = self.c3_overviews.iter()
            .filter(|o| o.itype.to_lowercase() == itype)
            .filter(|o| filter_lower.is_empty() || o.name.to_lowercase().contains(&filter_lower))
            .collect();
        matching.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        matching.get(self.c3_overview_popup_selected).map(|o| (*o).clone())
    }

    pub fn c3_lg_editor_field_value(&self) -> &str {
        match self.c3_lg_editor_field {
            C3LinkEditorField::Name => &self.c3_lg_editor_link.name,
            C3LinkEditorField::Url => &self.c3_lg_editor_link.url,
            C3LinkEditorField::Color => &self.c3_lg_editor_link.color,
            C3LinkEditorField::InfoField => &self.c3_lg_editor_link.info,
            C3LinkEditorField::ExternalDocName => &self.c3_lg_editor_link.external_doc_name,
            C3LinkEditorField::ExternalDocUrl => &self.c3_lg_editor_link.external_doc_url,
            C3LinkEditorField::Itypes => "",
        }
    }

    pub fn c3_lg_editor_field_value_mut(&mut self) -> &mut String {
        match self.c3_lg_editor_field {
            C3LinkEditorField::Name => &mut self.c3_lg_editor_link.name,
            C3LinkEditorField::Url => &mut self.c3_lg_editor_link.url,
            C3LinkEditorField::Color => &mut self.c3_lg_editor_link.color,
            C3LinkEditorField::InfoField => &mut self.c3_lg_editor_link.info,
            C3LinkEditorField::ExternalDocName => &mut self.c3_lg_editor_link.external_doc_name,
            C3LinkEditorField::ExternalDocUrl => &mut self.c3_lg_editor_link.external_doc_url,
            C3LinkEditorField::Itypes => &mut self.c3_lg_editor_link.name, // unreachable in practice
        }
    }
}
