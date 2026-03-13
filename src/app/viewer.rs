use super::*;
use crate::api::ArkimeField;
use serde_json::Value;

impl App {

    /// Rebuild session_fields from columns
    pub fn vr_sync_session_fields(&mut self) {
        self.vr_session_fields = self.vr_columns.iter().map(|c| c.field.clone()).collect();
        if self.vr_sort_column >= self.vr_columns.len() {
            self.vr_sort_column = 0;
        }
    }

    /// Apply a saved layout
    pub fn vr_apply_layout(&mut self, layout: &SavedLayout) {
        // Always start with the special ipProtocol column
        let mut cols = vec![ColumnDef::new("ipProtocol", "ip.protocol", "IP", 4)];
        for field in &layout.columns {
            // Resolve to dbField and exp names (layout may store exp or dbField)
            let found = self.vr_all_fields.iter()
                .find(|f| f.exp == *field || f.db_field == *field);
            let db_field = found.map(|f| f.db_field.clone()).unwrap_or_else(|| field.clone());
            let exp = found.map(|f| f.exp.clone()).unwrap_or_else(|| field.clone());
            let label = self.vr_field_friendly_map.get(db_field.as_str())
                .cloned()
                .unwrap_or_else(|| field.clone());
            let width = found.map(|f| width_for_field(&f.field_type)).unwrap_or(16);
            // Use default widths/labels for known fields
            let width = default_columns().iter()
                .find(|c| c.field == db_field)
                .map(|c| c.width)
                .unwrap_or(width);
            let label = default_columns().iter()
                .find(|c| c.field == db_field)
                .map(|c| c.label.clone())
                .unwrap_or(label);
            cols.push(ColumnDef::new(&db_field, &exp, &label, width));
        }
        if !cols.is_empty() {
            self.vr_columns = cols;
            self.vr_sync_session_fields();
            // Apply sort from layout (resolve to dbField)
            if !layout.sort_field.is_empty() {
                let sort_db = self.vr_all_fields.iter()
                    .find(|f| f.exp == layout.sort_field || f.db_field == layout.sort_field)
                    .map(|f| f.db_field.clone())
                    .unwrap_or_else(|| layout.sort_field.clone());
                if let Some(idx) = self.vr_columns.iter().position(|c| c.field == sort_db) {
                    self.vr_sort_column = idx;
                    self.vr_sort_desc = layout.sort_dir == "desc";
                }
            }
        }
    }

    /// Build column_editor_available from all_fields + current columns
    pub fn vr_build_column_editor(&mut self) {
        let enabled: std::collections::HashSet<String> = self.vr_columns.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<ColumnEditorItem> = Vec::new();
        // Add enabled columns first, in order
        for col in &self.vr_columns {
            let friendly = self.vr_field_friendly_map.get(col.field.as_str())
                .cloned()
                .unwrap_or_else(|| col.label.clone());
            items.push(ColumnEditorItem {
                db_field: col.field.clone(),
                exp: col.exp.clone(),
                friendly_name: friendly,
                enabled: true,
            });
        }
        // Add remaining fields (sorted by exp), excluding hidden fields
        let mut remaining: Vec<&ArkimeField> = self.vr_all_fields.iter()
            .filter(|f| !f.db_field.is_empty() && !enabled.contains(&f.db_field) && f.is_visible())
            .collect();
        remaining.sort_by(|a, b| a.exp.cmp(&b.exp));
        for field in remaining {
            items.push(ColumnEditorItem {
                db_field: field.db_field.clone(),
                exp: field.exp.clone(),
                friendly_name: field.friendly_name.clone(),
                enabled: false,
            });
        }
        self.vr_column_editor_available = items;
        self.vr_column_editor_selected = 0;
        self.vr_column_editor_mode = ColumnEditorMode::Browse;
        self.vr_column_editor_filter.clear();
    }

    /// Apply column editor selections back to columns
    pub fn vr_apply_column_editor(&mut self) {
        let mut new_cols = Vec::new();
        for item in &self.vr_column_editor_available {
            if !item.enabled { continue; }
            let width = default_columns().iter()
                .find(|c| c.field == item.db_field)
                .map(|c| c.width)
                .unwrap_or_else(|| {
                    self.vr_all_fields.iter()
                        .find(|f| f.db_field == item.db_field)
                        .map(|f| width_for_field(&f.field_type))
                        .unwrap_or(16)
                });
            let label = default_columns().iter()
                .find(|c| c.field == item.db_field)
                .map(|c| c.label.clone())
                .unwrap_or_else(|| item.friendly_name.clone());
            new_cols.push(ColumnDef::new(&item.db_field, &item.exp, &label, width));
        }
        if !new_cols.is_empty() {
            self.vr_columns = new_cols;
            self.vr_sync_session_fields();
        }
    }

    pub async fn vr_fetch_layouts(&mut self) {
        match self.client.vr_get_layouts().await {
            Ok(val) => {
                // Response can be an array of layouts or an object keyed by name
                let mut layouts = Vec::new();
                let items: Vec<&Value> = if let Some(arr) = val.as_array() {
                    arr.iter().collect()
                } else if let Some(obj) = val.as_object() {
                    obj.values().collect()
                } else {
                    Vec::new()
                };
                for v in items {
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let columns: Vec<String> = v.get("columns")
                        .and_then(|c| c.as_array())
                        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    let (sort_field, sort_dir) = v.get("order")
                        .and_then(|o| o.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|pair| pair.as_array())
                        .map(|pair| {
                            let f = pair.first().and_then(|x| x.as_str()).unwrap_or("").to_string();
                            let d = pair.get(1).and_then(|x| x.as_str()).unwrap_or("asc").to_string();
                            (f, d)
                        })
                        .unwrap_or_default();
                    layouts.push(SavedLayout { name, columns, sort_field, sort_dir });
                }
                self.vr_saved_layouts = layouts;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching layouts: {e}");
            }
        }
    }

    pub fn vr_remove_enabled(&self) -> bool {
        self.user.get("removeEnabled").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    /// Get the active columns for the current stats tab
    pub fn vr_stats_active_columns(&self) -> &Vec<StatsColumnDef> {
        let idx = StatsTab::ALL.iter().position(|&t| t == self.vr_stats_tab).unwrap_or(0);
        &self.vr_stats_columns[idx]
    }

    /// Get the sort field for the current stats tab and sort column index
    pub fn vr_stats_sort_field(&self) -> &str {
        self.vr_stats_active_columns()
            .get(self.vr_stats_sort_column)
            .map(|c| c.sort.as_str())
            .unwrap_or("nodeName")
    }

    /// Build stats column editor items from all-columns + current active columns
    pub fn vr_stats_build_column_editor(&mut self) {
        let all = stats_tab_all_columns(self.vr_stats_tab);
        let active = self.vr_stats_active_columns();
        let enabled: std::collections::HashSet<String> = active.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<StatsColumnEditorItem> = Vec::new();
        // Enabled columns first, in order
        for col in active {
            items.push(StatsColumnEditorItem {
                field: col.field.clone(),
                label: col.label.clone(),
                enabled: true,
            });
        }
        // Remaining columns, sorted by field name
        let mut remaining: Vec<&StatsColumnDef> = all.iter()
            .filter(|c| !enabled.contains(&c.field))
            .collect();
        remaining.sort_by(|a, b| a.field.cmp(&b.field));
        for col in remaining {
            items.push(StatsColumnEditorItem {
                field: col.field.clone(),
                label: col.label.clone(),
                enabled: false,
            });
        }
        self.vr_stats_column_editor_items = items;
        self.vr_stats_column_editor_selected = 0;
        self.vr_stats_column_editor_mode = ColumnEditorMode::Browse;
        self.vr_stats_column_editor_filter.clear();
    }

    /// Apply stats column editor selections back to the current tab's columns
    pub fn vr_stats_apply_column_editor(&mut self) {
        let all = stats_tab_all_columns(self.vr_stats_tab);
        let mut new_cols = Vec::new();
        for item in &self.vr_stats_column_editor_items {
            if !item.enabled { continue; }
            if let Some(def) = all.iter().find(|c| c.field == item.field) {
                new_cols.push(def.clone());
            }
        }
        if !new_cols.is_empty() {
            let idx = StatsTab::ALL.iter().position(|&t| t == self.vr_stats_tab).unwrap_or(0);
            self.vr_stats_columns[idx] = new_cols;
            self.vr_stats_sort_column = 0;
        }
    }

    /// Reset current stats tab to default columns
    pub fn vr_stats_reset_default_columns(&mut self) {
        let defaults = stats_tab_default_fields(self.vr_stats_tab);
        let all = stats_tab_all_columns(self.vr_stats_tab);
        let idx = StatsTab::ALL.iter().position(|&t| t == self.vr_stats_tab).unwrap_or(0);
        self.vr_stats_columns[idx] = stats_columns_from_fields(&defaults, &all);
        self.vr_stats_sort_column = 0;
    }

    /// Apply a saved shareable layout to the current stats tab
    pub fn vr_stats_apply_shareable(&mut self, shareable: &SavedShareable) {
        let all = stats_tab_all_columns(self.vr_stats_tab);
        let field_refs: Vec<&str> = shareable.columns.iter().map(|s| s.as_str()).collect();
        let new_cols = stats_columns_from_fields(&field_refs, &all);
        if !new_cols.is_empty() {
            let idx = StatsTab::ALL.iter().position(|&t| t == self.vr_stats_tab).unwrap_or(0);
            self.vr_stats_columns[idx] = new_cols;
            // Apply sort
            if !shareable.sort_field.is_empty() {
                let active = &self.vr_stats_columns[idx];
                if let Some(pos) = active.iter().position(|c| c.sort == shareable.sort_field || c.field == shareable.sort_field) {
                    self.vr_stats_sort_column = pos;
                    self.vr_stats_sort_desc = shareable.sort_dir == "desc";
                }
            }
        }
    }

    /// Fetch saved shareables for the current stats tab
    pub async fn vr_stats_fetch_shareables(&mut self) {
        let stype = stats_tab_shareable_type(self.vr_stats_tab);
        match self.client.get_shareables(stype).await {
            Ok(items) => {
                self.vr_stats_saved_shareables = items;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching layouts: {e}");
            }
        }
    }

    /// Save current stats columns as a new shareable
    pub async fn vr_stats_save_shareable(&mut self, name: &str) {
        let stype = stats_tab_shareable_type(self.vr_stats_tab);
        let columns: Vec<String> = self.vr_stats_active_columns().iter().map(|c| c.field.clone()).collect();
        let sort_field = self.vr_stats_sort_field().to_string();
        let sort_dir = if self.vr_stats_sort_desc { "desc" } else { "asc" }.to_string();
        // Check if a shareable with this name already exists
        let existing_id = self.vr_stats_saved_shareables.iter()
            .find(|s| s.name == name && !s.shared)
            .map(|s| s.id.clone());
        let result = if let Some(id) = existing_id {
            self.client.update_shareable(&id, name, stype, &columns, &sort_field, &sort_dir).await
        } else {
            self.client.create_shareable(name, stype, &columns, &sort_field, &sort_dir).await
        };
        match result {
            Ok(_) => {
                self.status_msg = format!("Saved layout '{name}'");
                self.vr_stats_fetch_shareables().await;
            }
            Err(e) => {
                self.status_msg = format!("Error saving layout: {e}");
            }
        }
    }

    /// Delete a shareable by ID
    pub async fn vr_stats_delete_shareable(&mut self, id: &str) {
        match self.client.delete_shareable(id).await {
            Ok(_) => {
                self.status_msg = "Layout deleted".into();
                self.vr_stats_fetch_shareables().await;
            }
            Err(e) => {
                self.status_msg = format!("Error deleting layout: {e}");
            }
        }
    }

    // --- Files tab methods ---

    pub fn vr_files_sort_field(&self) -> &str {
        self.vr_files_columns.get(self.vr_files_sort_column)
            .map(|c| c.sort.as_str())
            .unwrap_or("num")
    }

    pub fn vr_files_build_column_editor(&mut self) {
        let all = files_all_columns();
        let enabled: std::collections::HashSet<String> = self.vr_files_columns.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<StatsColumnEditorItem> = Vec::new();
        for col in &self.vr_files_columns {
            items.push(StatsColumnEditorItem { field: col.field.clone(), label: col.label.clone(), enabled: true });
        }
        let mut remaining: Vec<&StatsColumnDef> = all.iter().filter(|c| !enabled.contains(&c.field)).collect();
        remaining.sort_by(|a, b| a.field.cmp(&b.field));
        for col in remaining {
            items.push(StatsColumnEditorItem { field: col.field.clone(), label: col.label.clone(), enabled: false });
        }
        self.vr_files_column_editor_items = items;
        self.vr_files_column_editor_selected = 0;
        self.vr_files_column_editor_mode = ColumnEditorMode::Browse;
        self.vr_files_column_editor_filter.clear();
    }

    pub fn vr_files_apply_column_editor(&mut self) {
        let all = files_all_columns();
        let mut new_cols = Vec::new();
        for item in &self.vr_files_column_editor_items {
            if !item.enabled { continue; }
            if let Some(def) = all.iter().find(|c| c.field == item.field) {
                new_cols.push(def.clone());
            }
        }
        if !new_cols.is_empty() {
            self.vr_files_columns = new_cols;
            self.vr_files_sort_column = 0;
        }
    }

    pub fn vr_files_reset_default_columns(&mut self) {
        let defaults = files_default_fields();
        let all = files_all_columns();
        self.vr_files_columns = stats_columns_from_fields(&defaults, &all);
        self.vr_files_sort_column = 0;
    }

    pub fn vr_files_apply_shareable(&mut self, shareable: &SavedShareable) {
        let all = files_all_columns();
        let field_refs: Vec<&str> = shareable.columns.iter().map(|s| s.as_str()).collect();
        let new_cols = stats_columns_from_fields(&field_refs, &all);
        if !new_cols.is_empty() {
            self.vr_files_columns = new_cols;
            if !shareable.sort_field.is_empty() {
                if let Some(pos) = self.vr_files_columns.iter().position(|c| c.sort == shareable.sort_field || c.field == shareable.sort_field) {
                    self.vr_files_sort_column = pos;
                    self.vr_files_sort_desc = shareable.sort_dir == "desc";
                }
            }
        }
    }

    pub async fn vr_files_fetch_shareables(&mut self) {
        match self.client.get_shareables("files-columns").await {
            Ok(items) => { self.vr_files_saved_shareables = items; }
            Err(e) => { self.status_msg = format!("Error fetching layouts: {e}"); }
        }
    }

    pub async fn vr_files_save_shareable(&mut self, name: &str) {
        let columns: Vec<String> = self.vr_files_columns.iter().map(|c| c.field.clone()).collect();
        let sort_field = self.vr_files_sort_field().to_string();
        let sort_dir = if self.vr_files_sort_desc { "desc" } else { "asc" }.to_string();
        let existing_id = self.vr_files_saved_shareables.iter()
            .find(|s| s.name == name && !s.shared)
            .map(|s| s.id.clone());
        let result = if let Some(id) = existing_id {
            self.client.update_shareable(&id, name, "files-columns", &columns, &sort_field, &sort_dir).await
        } else {
            self.client.create_shareable(name, "files-columns", &columns, &sort_field, &sort_dir).await
        };
        match result {
            Ok(_) => {
                self.status_msg = format!("Saved layout '{name}'");
                self.vr_files_fetch_shareables().await;
            }
            Err(e) => { self.status_msg = format!("Error saving layout: {e}"); }
        }
    }

    pub async fn vr_files_delete_shareable(&mut self, id: &str) {
        match self.client.delete_shareable(id).await {
            Ok(_) => {
                self.status_msg = "Layout deleted".into();
                self.vr_files_fetch_shareables().await;
            }
            Err(e) => { self.status_msg = format!("Error deleting layout: {e}"); }
        }
    }

    pub async fn vr_fetch_files(&mut self) {
        self.status_msg = "Fetching files...".into();
        let sort_field = self.vr_files_sort_field().to_string();
        match self.client.vr_get_files(&self.vr_files_filter, &sort_field, self.vr_files_sort_desc, self.vr_files_page_start, self.vr_files_page_size).await {
            Ok(value) => {
                self.vr_files_data = value.get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.vr_files_total = value.get("recordsTotal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.vr_files_filtered = value.get("recordsFiltered")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if self.vr_files_selected >= self.vr_files_data.len() {
                    self.vr_files_selected = 0;
                }
                self.vr_files_table_state.select(if self.vr_files_data.is_empty() { None } else { Some(self.vr_files_selected) });
                self.status_msg = String::new();
            }
            Err(e) => {
                self.status_msg = format!("Error fetching files: {e}");
            }
        }
    }

    pub fn vr_open_files_detail(&mut self) {
        if let Some(item) = self.vr_files_data.get(self.vr_files_selected) {
            self.vr_files_detail = Some(StatsDetail { data: item.clone(), scroll: 0, filter: String::new() });
            self.vr_files_view = StatsView::Detail;
        }
    }

    pub async fn vr_fetch_fields(&mut self) {
        match self.client.vr_get_fields().await {
            Ok((fields, date_fields, field_exp_map, field_friendly_map)) => {
                self.vr_all_fields = fields;
                self.vr_date_fields = date_fields;
                self.vr_field_exp_map = field_exp_map;
                self.vr_field_friendly_map = field_friendly_map;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching fields: {e}");
            }
        }
    }

    pub(crate) async fn refresh_for_active_tab(&mut self) {
        if self.active_tab == Tab::Arkime {
            self.vr_request_summary_fetch();
        } else {
            self.vr_fetch_sessions().await;
        }
    }

    pub async fn vr_fetch_sessions(&mut self) {
        self.status_msg = "Fetching sessions...".into();
        let sort_field = self.vr_session_fields.get(self.vr_sort_column)
            .cloned()
            .unwrap_or_else(|| "firstPacket".into());
        match self.client.vr_get_sessions(&self.vr_session_fields, &self.expression, self.time_range.date_value(), &sort_field, self.vr_sort_desc, self.vr_graph_size.is_visible(), self.vr_page_start, self.vr_page_size, &self.vr_active_view).await {
            Ok(response) => {
                if let Some(err) = response.bsq_err.as_ref().or(response.error.as_ref()) {
                    self.status_msg = format!("Expression error: {err}");
                    return;
                }
                self.vr_sessions = response.data;
                self.vr_sessions_total = response.records_total;
                self.vr_sessions_filtered = response.records_filtered;
                self.vr_graph_data = response.graph;
                self.vr_selected_session = 0;
                self.vr_table_state.select(Some(0));
                let end = (self.vr_page_start + self.vr_sessions.len() as u64).min(self.vr_sessions_filtered);
                self.status_msg = format!(
                    "Showing {}-{} of {} sessions",
                    self.vr_page_start + 1, end, self.vr_sessions_filtered
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub async fn vr_open_session_detail(&mut self) {
        if let Some(session) = self.vr_sessions.get(self.vr_selected_session) {
            let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                self.status_msg = "No session id".into();
                return;
            }
            self.status_msg = "Fetching session detail...".into();
            match self.client.vr_get_session(id).await {
                Ok(data) => {
                    let total_rows = data.as_object()
                        .map(|o| o.keys().filter(|k| !is_hidden_detail_field(k)).count())
                        .unwrap_or(0);
                    self.vr_session_detail = Some(SessionDetail { data, scroll: 0, selected: 0, total_rows, filter: String::new() });
                    self.vr_session_view = SessionView::Detail;
                    self.status_msg = "Session detail loaded".into();
                }
                Err(e) => {
                    self.status_msg = format!("Error: {e}");
                }
            }
        }
    }

    pub async fn vr_fetch_stats(&mut self) {
        self.status_msg = format!("Fetching {}...", self.vr_stats_tab.name());
        let sort_field = self.vr_stats_sort_field().to_string();

        let result = match self.vr_stats_tab {
            StatsTab::Capture => self.client.vr_get_stats(&self.vr_stats_filter, &sort_field, self.vr_stats_sort_desc).await,
            StatsTab::DBStats => self.client.vr_get_esstats(&self.vr_stats_filter, &sort_field, self.vr_stats_sort_desc).await,
            StatsTab::DBIndices => self.client.vr_get_esindices(&self.vr_stats_filter, &sort_field, self.vr_stats_sort_desc).await,
        };

        match result {
            Ok(value) => {
                self.vr_stats_data = value.get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.vr_stats_total = value.get("recordsTotal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.vr_stats_filtered = value.get("recordsFiltered")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.vr_stats_selected = 0;
                self.vr_stats_table_state.select(Some(0));
                self.vr_stats_last_refresh = std::time::Instant::now();
                self.status_msg = format!(
                    "{}: {} items",
                    self.vr_stats_tab.name(), self.vr_stats_data.len()
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub fn vr_open_stats_detail(&mut self) {
        if let Some(item) = self.vr_stats_data.get(self.vr_stats_selected) {
            self.vr_stats_detail = Some(StatsDetail { data: item.clone(), scroll: 0, filter: String::new() });
            self.vr_stats_view = StatsView::Detail;
        }
    }

    pub fn vr_request_summary_fetch(&mut self) {
        if self.vr_summary_field.is_empty() {
            return;
        }
        self.show_loading = true;
        self.vr_pending_summary_fetch = true;
    }

    pub fn vr_sort_summary_data(&mut self) {
        let desc = self.vr_summary_sort_desc;
        match self.vr_summary_sort {
            SummarySort::Value => self.vr_summary_data.sort_by(|a, b| {
                let a_str = a.item.as_str().unwrap_or("");
                let b_str = b.item.as_str().unwrap_or("");
                if desc { b_str.cmp(a_str) } else { a_str.cmp(b_str) }
            }),
            SummarySort::Sessions => self.vr_summary_data.sort_by(|a, b| {
                if desc { b.sessions.cmp(&a.sessions) } else { a.sessions.cmp(&b.sessions) }
            }),
            SummarySort::Packets => self.vr_summary_data.sort_by(|a, b| {
                if desc { b.packets.cmp(&a.packets) } else { a.packets.cmp(&b.packets) }
            }),
            SummarySort::Bytes => self.vr_summary_data.sort_by(|a, b| {
                if desc { b.bytes.cmp(&a.bytes) } else { a.bytes.cmp(&b.bytes) }
            }),
        }
        self.vr_summary_selected = 0;
        self.vr_summary_table_state.select(Some(0));
    }

    pub fn vr_filtered_fields(&self) -> Vec<&ArkimeField> {
        if self.vr_field_filter.is_empty() {
            self.vr_all_fields.iter().filter(|f| f.is_visible()).collect()
        } else {
            let filter = self.vr_field_filter.to_lowercase();
            self.vr_all_fields.iter()
                .filter(|f| f.is_visible())
                .filter(|f| f.exp.to_lowercase().contains(&filter) || f.friendly_name.to_lowercase().contains(&filter))
                .collect()
        }
    }
}
