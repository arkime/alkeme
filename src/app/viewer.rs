use super::*;
use crate::api::ArkimeField;
use serde_json::Value;

impl App {

    /// Load user state and fetch data when first visiting Stats tab
    pub async fn vr_init_stats_tab(&mut self) {
        let idx = StatsTab::ALL.iter().position(|&t| t == self.viewer.stats_tab).unwrap_or(0);
        if !self.viewer.stats_state_loaded[idx] {
            self.vr_load_stats_state(self.viewer.stats_tab).await;
            self.viewer.stats_state_loaded[idx] = true;
        }
        self.vr_fetch_stats().await;
    }

    /// Load user state and fetch data when first visiting Files tab
    pub async fn vr_init_files_tab(&mut self) {
        if !self.viewer.files_state_loaded {
            self.vr_load_files_state().await;
            self.viewer.files_state_loaded = true;
        }
        self.vr_fetch_files().await;
    }

    /// Rebuild session_fields from columns
    pub fn vr_sync_session_fields(&mut self) {
        self.viewer.session_fields = self.viewer.columns.iter().map(|c| c.field.clone()).collect();
        if self.viewer.sort_column >= self.viewer.columns.len() {
            self.viewer.sort_column = 0;
        }
    }

    /// Apply a saved layout
    pub fn vr_apply_layout(&mut self, layout: &SavedLayout) {
        // Always start with the special ipProtocol column
        let mut cols = vec![ColumnDef::new("ipProtocol", "ip.protocol", "IP", 4)];
        for field in &layout.columns {
            // Resolve to dbField and exp names (layout may store exp or dbField)
            let found = self.viewer.all_fields.iter()
                .find(|f| f.exp == *field || f.db_field == *field);
            let db_field = found.map(|f| f.db_field.clone()).unwrap_or_else(|| field.clone());
            let exp = found.map(|f| f.exp.clone()).unwrap_or_else(|| field.clone());
            let label = self.viewer.field_friendly_map.get(db_field.as_str())
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
            self.viewer.columns = cols;
            self.vr_sync_session_fields();
            // Apply sort from layout (resolve to dbField)
            if !layout.sort_field.is_empty() {
                let sort_db = self.viewer.all_fields.iter()
                    .find(|f| f.exp == layout.sort_field || f.db_field == layout.sort_field)
                    .map(|f| f.db_field.clone())
                    .unwrap_or_else(|| layout.sort_field.clone());
                if let Some(idx) = self.viewer.columns.iter().position(|c| c.field == sort_db) {
                    self.viewer.sort_column = idx;
                    self.viewer.sort_desc = layout.sort_dir == "desc";
                }
            }
        }
    }

    /// Build column_editor_available from all_fields + current columns
    pub fn vr_build_column_editor(&mut self) {
        let enabled: std::collections::HashSet<String> = self.viewer.columns.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<ColumnEditorItem> = Vec::new();
        // Add enabled columns first, in order
        for col in &self.viewer.columns {
            let friendly = self.viewer.field_friendly_map.get(col.field.as_str())
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
        let mut remaining: Vec<&ArkimeField> = self.viewer.all_fields.iter()
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
        self.viewer.column_editor_available = items;
        self.viewer.column_editor_selected = 0;
        self.viewer.column_editor_mode = ColumnEditorMode::Browse;
        self.viewer.column_editor_filter.clear();
        self.viewer.column_editor_filter_cursor = 0;
    }

    /// Apply column editor selections back to columns
    pub fn vr_apply_column_editor(&mut self) {
        let mut new_cols = Vec::new();
        for item in &self.viewer.column_editor_available {
            if !item.enabled { continue; }
            let width = default_columns().iter()
                .find(|c| c.field == item.db_field)
                .map(|c| c.width)
                .unwrap_or_else(|| {
                    self.viewer.all_fields.iter()
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
            self.viewer.columns = new_cols;
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
                self.viewer.saved_layouts = layouts;
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
        let idx = StatsTab::ALL.iter().position(|&t| t == self.viewer.stats_tab).unwrap_or(0);
        &self.viewer.stats_columns[idx]
    }

    /// Get the sort field for the current stats tab and sort column index
    pub fn vr_stats_sort_field(&self) -> &str {
        self.vr_stats_active_columns()
            .get(self.viewer.stats_sort_column)
            .map(|c| c.sort.as_str())
            .unwrap_or("nodeName")
    }

    /// Build stats column editor items from all-columns + current active columns
    pub fn vr_stats_build_column_editor(&mut self) {
        let all = stats_tab_all_columns(self.viewer.stats_tab);
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
        self.viewer.stats_column_editor_items = items;
        self.viewer.stats_column_editor_selected = 0;
        self.viewer.stats_column_editor_mode = ColumnEditorMode::Browse;
        self.viewer.stats_column_editor_filter.clear();
        self.viewer.stats_column_editor_filter_cursor = 0;
    }

    /// Apply stats column editor selections back to the current tab's columns
    pub fn vr_stats_apply_column_editor(&mut self) {
        let all = stats_tab_all_columns(self.viewer.stats_tab);
        let mut new_cols = Vec::new();
        for item in &self.viewer.stats_column_editor_items {
            if !item.enabled { continue; }
            if let Some(def) = all.iter().find(|c| c.field == item.field) {
                new_cols.push(def.clone());
            }
        }
        if !new_cols.is_empty() {
            let idx = StatsTab::ALL.iter().position(|&t| t == self.viewer.stats_tab).unwrap_or(0);
            self.viewer.stats_columns[idx] = new_cols;
            self.viewer.stats_sort_column = 0;
        }
    }

    /// Reset current stats tab to default columns
    pub fn vr_stats_reset_default_columns(&mut self) {
        let defaults = stats_tab_default_fields(self.viewer.stats_tab);
        let all = stats_tab_all_columns(self.viewer.stats_tab);
        let idx = StatsTab::ALL.iter().position(|&t| t == self.viewer.stats_tab).unwrap_or(0);
        self.viewer.stats_columns[idx] = stats_columns_from_fields(&defaults, &all);
        self.viewer.stats_sort_column = 0;
    }

    /// Apply a saved shareable layout to the current stats tab
    pub fn vr_stats_apply_shareable(&mut self, shareable: &SavedShareable) {
        let all = stats_tab_all_columns(self.viewer.stats_tab);
        let field_refs: Vec<&str> = shareable.columns.iter().map(|s| s.as_str()).collect();
        let new_cols = stats_columns_from_fields(&field_refs, &all);
        if !new_cols.is_empty() {
            let idx = StatsTab::ALL.iter().position(|&t| t == self.viewer.stats_tab).unwrap_or(0);
            self.viewer.stats_columns[idx] = new_cols;
            // Apply sort
            if !shareable.sort_field.is_empty() {
                let active = &self.viewer.stats_columns[idx];
                if let Some(pos) = active.iter().position(|c| c.sort == shareable.sort_field || c.field == shareable.sort_field) {
                    self.viewer.stats_sort_column = pos;
                    self.viewer.stats_sort_desc = shareable.sort_dir == "desc";
                }
            }
        }
    }

    /// Fetch saved shareables for the current stats tab
    pub async fn vr_stats_fetch_shareables(&mut self) {
        let stype = stats_tab_shareable_type(self.viewer.stats_tab);
        match self.client.get_shareables(stype).await {
            Ok(items) => {
                self.viewer.stats_saved_shareables = items;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching layouts: {e}");
            }
        }
    }

    /// Save current stats columns as a new shareable
    pub async fn vr_stats_save_shareable(&mut self, name: &str) {
        let stype = stats_tab_shareable_type(self.viewer.stats_tab);
        let columns: Vec<String> = self.vr_stats_active_columns().iter().map(|c| c.field.clone()).collect();
        let sort_field = self.vr_stats_sort_field().to_string();
        let sort_dir = if self.viewer.stats_sort_desc { "desc" } else { "asc" }.to_string();
        // Check if a shareable with this name already exists
        let existing_id = self.viewer.stats_saved_shareables.iter()
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
        self.viewer.files_columns.get(self.viewer.files_sort_column)
            .map(|c| c.sort.as_str())
            .unwrap_or("num")
    }

    pub fn vr_files_build_column_editor(&mut self) {
        let all = files_all_columns();
        let enabled: std::collections::HashSet<String> = self.viewer.files_columns.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<StatsColumnEditorItem> = Vec::new();
        for col in &self.viewer.files_columns {
            items.push(StatsColumnEditorItem { field: col.field.clone(), label: col.label.clone(), enabled: true });
        }
        let mut remaining: Vec<&StatsColumnDef> = all.iter().filter(|c| !enabled.contains(&c.field)).collect();
        remaining.sort_by(|a, b| a.field.cmp(&b.field));
        for col in remaining {
            items.push(StatsColumnEditorItem { field: col.field.clone(), label: col.label.clone(), enabled: false });
        }
        self.viewer.files_column_editor_items = items;
        self.viewer.files_column_editor_selected = 0;
        self.viewer.files_column_editor_mode = ColumnEditorMode::Browse;
        self.viewer.files_column_editor_filter.clear();
        self.viewer.files_column_editor_filter_cursor = 0;
    }

    pub fn vr_files_apply_column_editor(&mut self) {
        let all = files_all_columns();
        let mut new_cols = Vec::new();
        for item in &self.viewer.files_column_editor_items {
            if !item.enabled { continue; }
            if let Some(def) = all.iter().find(|c| c.field == item.field) {
                new_cols.push(def.clone());
            }
        }
        if !new_cols.is_empty() {
            self.viewer.files_columns = new_cols;
            self.viewer.files_sort_column = 0;
        }
    }

    pub fn vr_files_reset_default_columns(&mut self) {
        let defaults = files_default_fields();
        let all = files_all_columns();
        self.viewer.files_columns = stats_columns_from_fields(&defaults, &all);
        self.viewer.files_sort_column = 0;
    }

    pub fn vr_files_apply_shareable(&mut self, shareable: &SavedShareable) {
        let all = files_all_columns();
        let field_refs: Vec<&str> = shareable.columns.iter().map(|s| s.as_str()).collect();
        let new_cols = stats_columns_from_fields(&field_refs, &all);
        if !new_cols.is_empty() {
            self.viewer.files_columns = new_cols;
            if !shareable.sort_field.is_empty() {
                if let Some(pos) = self.viewer.files_columns.iter().position(|c| c.sort == shareable.sort_field || c.field == shareable.sort_field) {
                    self.viewer.files_sort_column = pos;
                    self.viewer.files_sort_desc = shareable.sort_dir == "desc";
                }
            }
        }
    }

    pub async fn vr_files_fetch_shareables(&mut self) {
        match self.client.get_shareables("files-columns").await {
            Ok(items) => { self.viewer.files_saved_shareables = items; }
            Err(e) => { self.status_msg = format!("Error fetching layouts: {e}"); }
        }
    }

    pub async fn vr_files_save_shareable(&mut self, name: &str) {
        let columns: Vec<String> = self.viewer.files_columns.iter().map(|c| c.field.clone()).collect();
        let sort_field = self.vr_files_sort_field().to_string();
        let sort_dir = if self.viewer.files_sort_desc { "desc" } else { "asc" }.to_string();
        let existing_id = self.viewer.files_saved_shareables.iter()
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
        match self.client.vr_get_files(&self.viewer.files_filter, &sort_field, self.viewer.files_sort_desc, self.viewer.files_page_start, self.viewer.files_page_size).await {
            Ok(value) => {
                self.viewer.files_data = value.get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.viewer.files_total = value.get("recordsTotal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.viewer.files_filtered = value.get("recordsFiltered")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if self.viewer.files_selected >= self.viewer.files_data.len() {
                    self.viewer.files_selected = 0;
                }
                self.viewer.files_table_state.select(if self.viewer.files_data.is_empty() { None } else { Some(self.viewer.files_selected) });
                self.status_msg = String::new();
            }
            Err(e) => {
                self.status_msg = format!("Error fetching files: {e}");
            }
        }
    }

    pub fn vr_open_files_detail(&mut self) {
        if let Some(item) = self.viewer.files_data.get(self.viewer.files_selected) {
            self.viewer.files_detail = Some(StatsDetail { data: item.clone(), scroll: 0, filter: String::new(), filter_cursor: 0 });
            self.viewer.files_view = StatsView::Detail;
        }
    }

    pub async fn vr_fetch_fields(&mut self) {
        match self.client.vr_get_fields().await {
            Ok((fields, date_fields, field_exp_map, field_friendly_map)) => {
                self.viewer.all_fields = fields;
                self.viewer.date_fields = date_fields;
                self.viewer.field_exp_map = field_exp_map;
                self.viewer.field_friendly_map = field_friendly_map;
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
        let sort_field = self.viewer.session_fields.get(self.viewer.sort_column)
            .cloned()
            .unwrap_or_else(|| "firstPacket".into());
        match self.client.vr_get_sessions(&self.viewer.session_fields, &self.expression, self.time_range.date_value(), &sort_field, self.viewer.sort_desc, self.viewer.graph_size.is_visible(), self.viewer.page_start, self.viewer.page_size, &self.viewer.active_view).await {
            Ok(response) => {
                if let Some(err) = response.bsq_err.as_ref().or(response.error.as_ref()) {
                    self.status_msg = format!("Expression error: {err}");
                    return;
                }
                self.viewer.sessions = response.data;
                self.viewer.sessions_total = response.records_total;
                self.viewer.sessions_filtered = response.records_filtered;
                self.viewer.graph_data = response.graph;
                self.viewer.selected_session = 0;
                self.viewer.table_state.select(Some(0));
                let end = (self.viewer.page_start + self.viewer.sessions.len() as u64).min(self.viewer.sessions_filtered);
                self.status_msg = format!(
                    "Showing {}-{} of {} sessions",
                    self.viewer.page_start + 1, end, self.viewer.sessions_filtered
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub async fn vr_open_session_detail(&mut self) {
        if let Some(session) = self.viewer.sessions.get(self.viewer.selected_session) {
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
                    self.viewer.session_detail = Some(SessionDetail { data, scroll: 0, selected: 0, total_rows, filter: String::new(), filter_cursor: 0 });
                    self.viewer.session_view = SessionView::Detail;
                    self.status_msg = "Session detail loaded".into();
                }
                Err(e) => {
                    self.status_msg = format!("Error: {e}");
                }
            }
        }
    }

    pub async fn vr_fetch_stats(&mut self) {
        if self.viewer.stats_tab == StatsTab::DBShards {
            return self.vr_fetch_shards().await;
        }
        self.status_msg = format!("Fetching {}...", self.viewer.stats_tab.name());
        let sort_field = self.vr_stats_sort_field().to_string();

        let result = match self.viewer.stats_tab {
            StatsTab::Capture => self.client.vr_get_stats(&self.viewer.stats_filter, &sort_field, self.viewer.stats_sort_desc).await,
            StatsTab::DBStats => self.client.vr_get_esstats(&self.viewer.stats_filter, &sort_field, self.viewer.stats_sort_desc).await,
            StatsTab::DBIndices => self.client.vr_get_esindices(&self.viewer.stats_filter, &sort_field, self.viewer.stats_sort_desc).await,
            StatsTab::DBTasks => self.client.vr_get_estasks(&self.viewer.stats_filter, &sort_field, self.viewer.stats_sort_desc).await,
            StatsTab::DBShards => unreachable!(),
        };

        match result {
            Ok(value) => {
                self.viewer.stats_data = value.get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.viewer.stats_total = value.get("recordsTotal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.viewer.stats_filtered = value.get("recordsFiltered")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.viewer.stats_selected = 0;
                self.viewer.stats_table_state.select(Some(0));
                self.viewer.stats_last_refresh = std::time::Instant::now();
                self.status_msg = format!(
                    "{}: {} items",
                    self.viewer.stats_tab.name(), self.viewer.stats_data.len()
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub fn vr_open_stats_detail(&mut self) {
        if let Some(item) = self.viewer.stats_data.get(self.viewer.stats_selected) {
            self.viewer.stats_detail = Some(StatsDetail { data: item.clone(), scroll: 0, filter: String::new(), filter_cursor: 0 });
            self.viewer.stats_view = StatsView::Detail;
        }
    }

    pub fn vr_open_shards_detail(&mut self) {
        let index_name = match self.viewer.shards_indices.get(self.viewer.shards_selected_row) {
            Some(name) => name.clone(),
            None => return,
        };
        // Find the index data and build a Value with all shards flattened
        let indices_arr = self.viewer.shards_data.get("indices").and_then(|i| i.as_array());
        let index_data = indices_arr.and_then(|arr| {
            arr.iter().find(|idx| idx.get("name").and_then(|n| n.as_str()) == Some(&index_name))
        });
        if let Some(idx) = index_data {
            // Build a combined Value: { "index": name, "shards": [ {node, shard, prirep, state, ...} ] }
            let mut all_shards = Vec::new();
            if let Some(nodes_obj) = idx.get("nodes").and_then(|n| n.as_object()) {
                // Use the sorted node order from shards_nodes
                for node_name in &self.viewer.shards_nodes {
                    if let Some(shards) = nodes_obj.get(node_name).and_then(|s| s.as_array()) {
                        for shard in shards {
                            let mut entry = shard.clone();
                            if let Some(obj) = entry.as_object_mut() {
                                obj.insert("node".to_string(), serde_json::Value::String(node_name.clone()));
                            }
                            all_shards.push(entry);
                        }
                    }
                }
            }
            let data = serde_json::json!({
                "index": index_name,
                "shards": all_shards,
            });
            self.viewer.shards_detail = Some(StatsDetail {
                data,
                scroll: 0,
                filter: String::new(),
                filter_cursor: 0,
            });
        }
    }

    pub async fn vr_open_shard_sub_detail(&mut self) {
        let detail = match &self.viewer.shards_detail {
            Some(d) => d,
            None => return,
        };
        let selected = detail.scroll as usize;
        let shards = match detail.data.get("shards").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return,
        };
        let filter_lower = detail.filter.to_lowercase();
        // Get the selected shard after filtering
        let filtered: Vec<&serde_json::Value> = shards.iter().filter(|s| {
            if filter_lower.is_empty() { return true; }
            let text = s.as_object().map(|o| {
                o.values().map(|v| match v { serde_json::Value::String(s) => s.as_str(), _ => "" }).collect::<Vec<_>>().join(" ")
            }).unwrap_or_default();
            text.to_lowercase().contains(&filter_lower)
        }).collect();
        let shard = match filtered.get(selected) {
            Some(s) => (*s).clone(),
            None => return,
        };
        let index_name = detail.data.get("index").and_then(|v| v.as_str()).unwrap_or("");
        let shard_num = match shard.get("shard") {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => return,
        };
        let is_primary = shard.get("prirep").and_then(|v| v.as_str()) == Some("p");

        // Fetch allocation explain
        self.status_msg = "Fetching allocation explain...".into();
        let explain = match self.client.vr_get_allocation_explain(index_name, &shard_num, is_primary).await {
            Ok(v) => v,
            Err(e) => {
                self.status_msg = format!("Explain error: {e}");
                serde_json::json!({"error": e.to_string()})
            }
        };

        // Combine shard fields + explain into one Value
        let mut combined = serde_json::Map::new();
        if let Some(obj) = shard.as_object() {
            for (k, v) in obj {
                combined.insert(k.clone(), v.clone());
            }
        }
        combined.insert("_explain".to_string(), explain);

        self.viewer.shards_sub_detail = Some(StatsDetail {
            data: serde_json::Value::Object(combined),
            scroll: 0,
            filter: String::new(),
            filter_cursor: 0,
        });
        self.status_msg = String::new();
    }

    pub async fn vr_fetch_shards(&mut self) {
        self.status_msg = "Fetching DB Shards...".into();
        let show = self.viewer.shards_show.api_value();
        match self.client.vr_get_esshards(&self.viewer.stats_filter, show).await {
            Ok(value) => {
                // Extract sorted node names from "nodes" object
                let mut nodes: Vec<String> = value.get("nodes")
                    .and_then(|n| n.as_object())
                    .map(|obj| obj.keys().cloned().collect())
                    .unwrap_or_default();
                // Put "Unassigned" first, then sort the rest
                nodes.sort_by(|a, b| {
                    if a == "Unassigned" { std::cmp::Ordering::Less }
                    else if b == "Unassigned" { std::cmp::Ordering::Greater }
                    else { a.cmp(b) }
                });
                // Extract index names from "indices" array
                let indices: Vec<String> = value.get("indices")
                    .and_then(|i| i.as_array())
                    .map(|arr| arr.iter().filter_map(|idx| idx.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
                    .unwrap_or_default();
                let num_indices = indices.len();
                self.viewer.shards_nodes = nodes;
                self.viewer.shards_indices = indices;
                self.viewer.shards_data = value;
                self.viewer.shards_selected_row = 0;
                self.viewer.shards_loaded = true;
                self.viewer.stats_last_refresh = std::time::Instant::now();
                self.status_msg = format!(
                    "DB Shards: {} indices, {} nodes [{}]",
                    num_indices, self.viewer.shards_nodes.len(), self.viewer.shards_show.label()
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub fn vr_request_summary_fetch(&mut self) {
        if self.viewer.summary_field.is_empty() {
            return;
        }
        self.show_loading = true;
        self.viewer.pending_summary_fetch = true;
    }

    pub fn vr_sort_summary_data(&mut self) {
        let desc = self.viewer.summary_sort_desc;
        match self.viewer.summary_sort {
            SummarySort::Value => self.viewer.summary_data.sort_by(|a, b| {
                let a_str = a.item.as_str().unwrap_or("");
                let b_str = b.item.as_str().unwrap_or("");
                if desc { b_str.cmp(a_str) } else { a_str.cmp(b_str) }
            }),
            SummarySort::Sessions => self.viewer.summary_data.sort_by(|a, b| {
                if desc { b.sessions.cmp(&a.sessions) } else { a.sessions.cmp(&b.sessions) }
            }),
            SummarySort::Packets => self.viewer.summary_data.sort_by(|a, b| {
                if desc { b.packets.cmp(&a.packets) } else { a.packets.cmp(&b.packets) }
            }),
            SummarySort::Bytes => self.viewer.summary_data.sort_by(|a, b| {
                if desc { b.bytes.cmp(&a.bytes) } else { a.bytes.cmp(&b.bytes) }
            }),
        }
        self.viewer.summary_selected = 0;
        self.viewer.summary_table_state.select(Some(0));
    }

    pub fn vr_filtered_fields(&self) -> Vec<&ArkimeField> {
        if self.viewer.field_filter.is_empty() {
            self.viewer.all_fields.iter().filter(|f| f.is_visible()).collect()
        } else {
            let filter = self.viewer.field_filter.to_lowercase();
            self.viewer.all_fields.iter()
                .filter(|f| f.is_visible())
                .filter(|f| f.exp.to_lowercase().contains(&filter) || f.friendly_name.to_lowercase().contains(&filter))
                .collect()
        }
    }

    // --- User state persistence (column/sort for Stats & Files) ---

    fn stats_state_name(tab: StatsTab) -> &'static str {
        match tab {
            StatsTab::Capture => "captureStatsCols",
            StatsTab::DBStats => "esNodesCols",
            StatsTab::DBIndices => "esIndicesCols",
            StatsTab::DBTasks => "esTasksCols",
            StatsTab::DBShards => "esShardsCols",
        }
    }

    /// Build the state JSON for the current stats tab
    fn vr_stats_state_json(&self) -> Value {
        let cols = self.vr_stats_active_columns();
        let headers: Vec<&str> = cols.iter().map(|c| c.field.as_str()).collect();
        let sort_field = self.vr_stats_sort_field();
        let sort_dir = if self.viewer.stats_sort_desc { "desc" } else { "asc" };
        serde_json::json!({
            "order": [[sort_field, sort_dir]],
            "visibleHeaders": headers
        })
    }

    /// Build the state JSON for files tab
    fn vr_files_state_json(&self) -> Value {
        let headers: Vec<&str> = self.viewer.files_columns.iter().map(|c| c.field.as_str()).collect();
        let sort_field = self.vr_files_sort_field();
        let sort_dir = if self.viewer.files_sort_desc { "desc" } else { "asc" };
        serde_json::json!({
            "order": [[sort_field, sort_dir]],
            "visibleHeaders": headers
        })
    }

    /// Save current stats tab column state to server
    pub async fn vr_save_stats_state(&self) {
        let name = Self::stats_state_name(self.viewer.stats_tab);
        let data = self.vr_stats_state_json();
        let _ = self.client.save_user_state(name, &data).await;
    }

    /// Save current files column state to server
    pub async fn vr_save_files_state(&self) {
        let data = self.vr_files_state_json();
        let _ = self.client.save_user_state("fieldsCols", &data).await;
    }

    /// Load stats column state from server for the given tab
    pub async fn vr_load_stats_state(&mut self, tab: StatsTab) {
        let name = Self::stats_state_name(tab);
        let state = match self.client.get_user_state(name).await {
            Ok(v) => v,
            Err(_) => return,
        };
        let headers = match state.get("visibleHeaders").and_then(|v| v.as_array()) {
            Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
            None => return,
        };
        if headers.is_empty() { return; }
        let all = stats_tab_all_columns(tab);
        let new_cols = stats_columns_from_fields(&headers, &all);
        if new_cols.is_empty() { return; }
        let idx = StatsTab::ALL.iter().position(|&t| t == tab).unwrap_or(0);
        self.viewer.stats_columns[idx] = new_cols;
        // Apply sort from state
        if let Some(order) = state.get("order").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_array()) {
            let sort_field = order.first().and_then(|v| v.as_str()).unwrap_or("");
            let sort_dir = order.get(1).and_then(|v| v.as_str()).unwrap_or("asc");
            if !sort_field.is_empty() {
                if let Some(pos) = self.viewer.stats_columns[idx].iter().position(|c| c.sort == sort_field || c.field == sort_field) {
                    self.viewer.stats_sort_column = pos;
                    self.viewer.stats_sort_desc = sort_dir == "desc";
                }
            }
        }
    }

    /// Load files column state from server
    pub async fn vr_load_files_state(&mut self) {
        let state = match self.client.get_user_state("fieldsCols").await {
            Ok(v) => v,
            Err(_) => return,
        };
        let headers = match state.get("visibleHeaders").and_then(|v| v.as_array()) {
            Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
            None => return,
        };
        if headers.is_empty() { return; }
        let all = files_all_columns();
        let new_cols = stats_columns_from_fields(&headers, &all);
        if new_cols.is_empty() { return; }
        self.viewer.files_columns = new_cols;
        // Apply sort from state
        if let Some(order) = state.get("order").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_array()) {
            let sort_field = order.first().and_then(|v| v.as_str()).unwrap_or("");
            let sort_dir = order.get(1).and_then(|v| v.as_str()).unwrap_or("asc");
            if !sort_field.is_empty() {
                if let Some(pos) = self.viewer.files_columns.iter().position(|c| c.sort == sort_field || c.field == sort_field) {
                    self.viewer.files_sort_column = pos;
                    self.viewer.files_sort_desc = sort_dir == "desc";
                }
            }
        }
    }
}
