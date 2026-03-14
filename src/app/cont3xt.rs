use super::*;
use crate::api::{Cont3xtIntegration, parse_card};
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
                self.c3_ov_list = overviews.clone();
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

}
