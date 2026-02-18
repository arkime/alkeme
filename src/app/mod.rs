mod types;
mod keys;

pub use types::*;

use crate::api::{ArkimeClient, ArkimeField, ArkimeView, GraphData, HttpLog, SummaryItem};
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::HashMap;

pub struct App {
    pub client: ArkimeClient,
    pub user: Value,
    pub active_tab: Tab,
    pub time_range: TimeRange,
    pub expression: String,
    pub expression_edit: String,
    pub expression_cursor: usize,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub action_menu: Option<ActionMenu>,
    pub action_prompt: Option<ActionPrompt>,
    pub date_fields: HashMap<String, String>, // dbField -> type ("seconds" or "date")
    pub field_exp_map: HashMap<String, String>, // dbField -> exp (expression field name)
    pub field_friendly_map: HashMap<String, String>, // dbField -> friendlyName
    pub sessions: Vec<Value>,
    pub sessions_total: u64,
    pub sessions_filtered: u64,
    pub session_fields: Vec<String>,
    pub columns: Vec<ColumnDef>,
    pub saved_layouts: Vec<SavedLayout>,
    pub show_column_editor: bool,
    pub column_editor_selected: usize,
    pub column_editor_mode: ColumnEditorMode,
    pub column_editor_available: Vec<ColumnEditorItem>,
    pub column_editor_filter: String,
    pub show_layout_popup: bool,
    pub layout_popup_mode: LayoutPopupMode,
    pub layout_popup_selected: usize,
    pub layout_save_name: String,
    pub layout_save_cursor: usize,
    pub layout_delete_name: String,
    pub layout_filter: String,
    // View state
    pub active_view: Option<String>,      // view id for API calls
    pub active_view_name: Option<String>,  // view name for display
    pub saved_views: Vec<ArkimeView>,
    pub show_view_popup: bool,
    pub view_popup_mode: ViewPopupMode,
    pub view_popup_selected: usize,
    pub view_save_name: String,
    pub view_save_cursor: usize,
    pub view_save_columns: bool,
    pub view_delete_id: String,
    pub view_delete_name: String,
    pub view_filter: String,
    pub view_filter_active: bool,
    pub page_start: u64,
    pub page_size: u64,
    pub selected_session: usize,
    pub table_state: TableState,
    pub session_view: SessionView,
    pub session_detail: Option<SessionDetail>,
    pub detail_action_menu: Option<DetailActionMenu>,
    pub packets_view: Option<crate::api::PacketsData>,
    pub packets_scroll: u16,
    pub packets_raw: bool,
    pub packets_line: LineMode,
    pub show_loading: bool,
    pub loading_owl_x: u16,
    pub loading_owl_dx: i16,
    pub loading_owl_tick: std::time::Instant,
    pub pending_packets_fetch: bool,
    pub pending_summary_fetch: bool,
    pub packets_node_pending: String,
    pub packets_id_pending: String,
    pub packets_total_pending: u64,
    pub sort_column: usize,
    pub sort_desc: bool,
    pub graph_size: GraphSize,
    pub graph_type: GraphType,
    pub graph_data: Option<GraphData>,
    pub status_msg: String,
    pub show_debug: bool,
    pub debug_scroll: usize,
    pub http_log: HttpLog,
    // Stats tab state
    pub stats_tab: StatsTab,
    pub stats_data: Vec<Value>,
    pub stats_total: u64,
    pub stats_filtered: u64,
    pub stats_filter: String,
    pub stats_filter_edit: String,
    pub stats_selected: usize,
    pub stats_table_state: TableState,
    pub stats_view: StatsView,
    pub stats_detail: Option<StatsDetail>,
    pub stats_sort_column: usize,
    pub stats_sort_desc: bool,
    pub stats_last_refresh: std::time::Instant,
    pub visible_rows: usize,
    // Arkime (Summary) tab state
    pub all_fields: Vec<ArkimeField>,
    pub summary_field: String,
    pub summary_data: Vec<SummaryItem>,
    pub summary_metric: SummaryMetric,
    pub summary_selected: usize,
    pub summary_table_state: TableState,
    pub summary_sort: SummarySort,
    pub summary_sort_desc: bool,
    pub field_filter: String,
    pub field_filter_selected: usize,
    // Owl animation
    pub owl_x: f32,
    pub owl_y: f32,
    pub owl_dx: f32,
    pub owl_dy: f32,
    pub owl_frame: usize,
    pub owl_tick: std::time::Instant,
    pub anim_start: std::time::Instant,
}

impl App {
    pub fn new(base_url: &str, auth_mode: crate::api::AuthMode, username: Option<String>, password: Option<String>) -> Self {
        let client = ArkimeClient::new(base_url, auth_mode, username, password);
        let http_log = client.http_log();
        Self {
            client,
            user: Value::Null,
            active_tab: Tab::Sessions,
            time_range: TimeRange::All,
            expression: String::new(),
            expression_edit: String::new(),
            expression_cursor: 0,
            input_mode: InputMode::Normal,
            show_help: false,
            action_menu: None,
            action_prompt: None,
            date_fields: HashMap::new(),
            field_exp_map: HashMap::new(),
            field_friendly_map: HashMap::new(),
            sessions: Vec::new(),
            sessions_total: 0,
            sessions_filtered: 0,
            session_fields: vec![
                "ipProtocol".into(),
                "firstPacket".into(),
                "lastPacket".into(),
                "source.ip".into(),
                "source.port".into(),
                "destination.ip".into(),
                "destination.port".into(),
                "protocol".into(),
                "source.packets".into(),
                "destination.packets".into(),
                "source.bytes".into(),
                "destination.bytes".into(),
            ],
            columns: default_columns(),
            saved_layouts: Vec::new(),
            show_column_editor: false,
            column_editor_selected: 0,
            column_editor_mode: ColumnEditorMode::Browse,
            column_editor_available: Vec::new(),
            column_editor_filter: String::new(),
            show_layout_popup: false,
            layout_popup_mode: LayoutPopupMode::List,
            layout_popup_selected: 0,
            layout_save_name: String::new(),
            layout_save_cursor: 0,
            layout_delete_name: String::new(),
            layout_filter: String::new(),
            active_view: None,
            active_view_name: None,
            saved_views: Vec::new(),
            show_view_popup: false,
            view_popup_mode: ViewPopupMode::List,
            view_popup_selected: 0,
            view_save_name: String::new(),
            view_save_cursor: 0,
            view_save_columns: false,
            view_delete_id: String::new(),
            view_delete_name: String::new(),
            view_filter: String::new(),
            view_filter_active: false,
            page_start: 0,
            page_size: 100,
            selected_session: 0,
            table_state: TableState::default().with_selected(0),
            session_view: SessionView::List,
            session_detail: None,
            detail_action_menu: None,
            packets_view: None,
            packets_scroll: 0,
            packets_raw: false,
            packets_line: LineMode::Hex,
            show_loading: false,
            loading_owl_x: 0,
            loading_owl_dx: 1,
            loading_owl_tick: std::time::Instant::now(),
            pending_packets_fetch: false,
            pending_summary_fetch: false,
            packets_node_pending: String::new(),
            packets_id_pending: String::new(),
            packets_total_pending: 0,
            sort_column: 2,
            sort_desc: true,
            graph_size: GraphSize::Off,
            graph_type: GraphType::Sessions,
            graph_data: None,
            status_msg: String::new(),
            show_debug: false,
            debug_scroll: 0,
            http_log,
            // Stats tab state
            stats_tab: StatsTab::Capture,
            stats_data: Vec::new(),
            stats_total: 0,
            stats_filtered: 0,
            stats_filter: String::new(),
            stats_filter_edit: String::new(),
            stats_selected: 0,
            stats_table_state: TableState::default().with_selected(0),
            stats_view: StatsView::List,
            stats_detail: None,
            stats_sort_column: 0,
            stats_sort_desc: false,
            stats_last_refresh: std::time::Instant::now(),
            visible_rows: 20,
            // Arkime (Summary) tab state
            all_fields: Vec::new(),
            summary_field: String::new(),
            summary_data: Vec::new(),
            summary_metric: SummaryMetric::Sessions,
            summary_selected: 0,
            summary_table_state: TableState::default().with_selected(0),
            summary_sort: SummarySort::Sessions,
            summary_sort_desc: true,
            field_filter: String::new(),
            field_filter_selected: 0,
            // Owl animation
            owl_x: 5.0,
            owl_y: 3.0,
            owl_dx: 1.0,
            owl_dy: 0.5,
            owl_frame: 0,
            owl_tick: std::time::Instant::now(),
            anim_start: std::time::Instant::now(),
        }
    }

    pub fn is_detail_view(&self) -> bool {
        self.session_view == SessionView::Detail || self.stats_view == StatsView::Detail
    }

    /// Rebuild session_fields from columns
    pub fn sync_session_fields(&mut self) {
        self.session_fields = self.columns.iter().map(|c| c.field.clone()).collect();
        if self.sort_column >= self.columns.len() {
            self.sort_column = 0;
        }
    }

    /// Apply a saved layout
    pub fn apply_layout(&mut self, layout: &SavedLayout) {
        // Always start with the special ipProtocol column
        let mut cols = vec![ColumnDef::new("ipProtocol", "ip.protocol", "IP", 4)];
        for field in &layout.columns {
            // Resolve to dbField and exp names (layout may store exp or dbField)
            let found = self.all_fields.iter()
                .find(|f| f.exp == *field || f.db_field == *field);
            let db_field = found.map(|f| f.db_field.clone()).unwrap_or_else(|| field.clone());
            let exp = found.map(|f| f.exp.clone()).unwrap_or_else(|| field.clone());
            let label = self.field_friendly_map.get(db_field.as_str())
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
            self.columns = cols;
            self.sync_session_fields();
            // Apply sort from layout (resolve to dbField)
            if !layout.sort_field.is_empty() {
                let sort_db = self.all_fields.iter()
                    .find(|f| f.exp == layout.sort_field || f.db_field == layout.sort_field)
                    .map(|f| f.db_field.clone())
                    .unwrap_or_else(|| layout.sort_field.clone());
                if let Some(idx) = self.columns.iter().position(|c| c.field == sort_db) {
                    self.sort_column = idx;
                    self.sort_desc = layout.sort_dir == "desc";
                }
            }
        }
    }

    /// Build column_editor_available from all_fields + current columns
    pub fn build_column_editor(&mut self) {
        let enabled: std::collections::HashSet<String> = self.columns.iter().map(|c| c.field.clone()).collect();
        let mut items: Vec<ColumnEditorItem> = Vec::new();
        // Add enabled columns first, in order
        for col in &self.columns {
            let friendly = self.field_friendly_map.get(col.field.as_str())
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
        let mut remaining: Vec<&ArkimeField> = self.all_fields.iter()
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
        self.column_editor_available = items;
        self.column_editor_selected = 0;
        self.column_editor_mode = ColumnEditorMode::Browse;
        self.column_editor_filter.clear();
    }

    /// Apply column editor selections back to columns
    pub fn apply_column_editor(&mut self) {
        let mut new_cols = Vec::new();
        for item in &self.column_editor_available {
            if !item.enabled { continue; }
            let width = default_columns().iter()
                .find(|c| c.field == item.db_field)
                .map(|c| c.width)
                .unwrap_or_else(|| {
                    self.all_fields.iter()
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
            self.columns = new_cols;
            self.sync_session_fields();
        }
    }

    pub async fn fetch_layouts(&mut self) {
        match self.client.get_layouts().await {
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
                self.saved_layouts = layouts;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching layouts: {e}");
            }
        }
    }

    pub fn remove_enabled(&self) -> bool {
        self.user.get("removeEnabled").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub async fn fetch_user(&mut self) {
        match self.client.get_user().await {
            Ok(user) => {
                self.user = user;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching user: {e}");
            }
        }
    }

    pub async fn fetch_fields(&mut self) {
        match self.client.get_fields().await {
            Ok((fields, date_fields, field_exp_map, field_friendly_map)) => {
                self.all_fields = fields;
                self.date_fields = date_fields;
                self.field_exp_map = field_exp_map;
                self.field_friendly_map = field_friendly_map;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching fields: {e}");
            }
        }
    }

    async fn refresh_for_active_tab(&mut self) {
        if self.active_tab == Tab::Arkime {
            self.request_summary_fetch();
        } else {
            self.fetch_sessions().await;
        }
    }

    pub async fn fetch_sessions(&mut self) {
        self.status_msg = "Fetching sessions...".into();
        let sort_field = self.session_fields.get(self.sort_column)
            .cloned()
            .unwrap_or_else(|| "firstPacket".into());
        match self.client.get_sessions(&self.session_fields, &self.expression, self.time_range.date_value(), &sort_field, self.sort_desc, self.graph_size.is_visible(), self.page_start, self.page_size, &self.active_view).await {
            Ok(response) => {
                if let Some(err) = response.bsq_err.as_ref().or(response.error.as_ref()) {
                    self.status_msg = format!("Expression error: {err}");
                    return;
                }
                self.sessions = response.data;
                self.sessions_total = response.records_total;
                self.sessions_filtered = response.records_filtered;
                self.graph_data = response.graph;
                self.selected_session = 0;
                self.table_state.select(Some(0));
                let end = (self.page_start + self.sessions.len() as u64).min(self.sessions_filtered);
                self.status_msg = format!(
                    "Showing {}-{} of {} sessions",
                    self.page_start + 1, end, self.sessions_filtered
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub async fn open_session_detail(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_session) {
            let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                self.status_msg = "No session id".into();
                return;
            }
            self.status_msg = "Fetching session detail...".into();
            match self.client.get_session(id).await {
                Ok(data) => {
                    let total_rows = data.as_object()
                        .map(|o| o.keys().filter(|k| !is_hidden_detail_field(k)).count())
                        .unwrap_or(0);
                    self.session_detail = Some(SessionDetail { data, scroll: 0, selected: 0, total_rows, filter: String::new() });
                    self.session_view = SessionView::Detail;
                    self.status_msg = "Session detail loaded".into();
                }
                Err(e) => {
                    self.status_msg = format!("Error: {e}");
                }
            }
        }
    }

    pub async fn fetch_stats(&mut self) {
        self.status_msg = format!("Fetching {}...", self.stats_tab.name());
        let columns = self.stats_tab.columns();
        let sort_field = columns.get(self.stats_sort_column)
            .map(|(f, _, _)| *f)
            .unwrap_or(columns[0].0);

        let result = match self.stats_tab {
            StatsTab::Capture => self.client.get_stats(&self.stats_filter, sort_field, self.stats_sort_desc).await,
            StatsTab::DBStats => self.client.get_esstats(&self.stats_filter, sort_field, self.stats_sort_desc).await,
            StatsTab::DBIndices => self.client.get_esindices(&self.stats_filter, sort_field, self.stats_sort_desc).await,
        };

        match result {
            Ok(value) => {
                self.stats_data = value.get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.stats_total = value.get("recordsTotal")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.stats_filtered = value.get("recordsFiltered")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.stats_selected = 0;
                self.stats_table_state.select(Some(0));
                self.stats_last_refresh = std::time::Instant::now();
                self.status_msg = format!(
                    "{}: {} items",
                    self.stats_tab.name(), self.stats_data.len()
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    pub fn open_stats_detail(&mut self) {
        if let Some(item) = self.stats_data.get(self.stats_selected) {
            self.stats_detail = Some(StatsDetail { data: item.clone(), scroll: 0, filter: String::new() });
            self.stats_view = StatsView::Detail;
        }
    }

    pub fn request_summary_fetch(&mut self) {
        if self.summary_field.is_empty() {
            return;
        }
        self.show_loading = true;
        self.pending_summary_fetch = true;
    }

    pub fn sort_summary_data(&mut self) {
        let desc = self.summary_sort_desc;
        match self.summary_sort {
            SummarySort::Value => self.summary_data.sort_by(|a, b| {
                let a_str = a.item.as_str().unwrap_or("");
                let b_str = b.item.as_str().unwrap_or("");
                if desc { b_str.cmp(a_str) } else { a_str.cmp(b_str) }
            }),
            SummarySort::Sessions => self.summary_data.sort_by(|a, b| {
                if desc { b.sessions.cmp(&a.sessions) } else { a.sessions.cmp(&b.sessions) }
            }),
            SummarySort::Packets => self.summary_data.sort_by(|a, b| {
                if desc { b.packets.cmp(&a.packets) } else { a.packets.cmp(&b.packets) }
            }),
            SummarySort::Bytes => self.summary_data.sort_by(|a, b| {
                if desc { b.bytes.cmp(&a.bytes) } else { a.bytes.cmp(&b.bytes) }
            }),
        }
        self.summary_selected = 0;
        self.summary_table_state.select(Some(0));
    }

    pub fn filtered_fields(&self) -> Vec<&ArkimeField> {
        if self.field_filter.is_empty() {
            self.all_fields.iter().filter(|f| f.is_visible()).collect()
        } else {
            let filter = self.field_filter.to_lowercase();
            self.all_fields.iter()
                .filter(|f| f.is_visible())
                .filter(|f| f.exp.to_lowercase().contains(&filter) || f.friendly_name.to_lowercase().contains(&filter))
                .collect()
        }
    }
}
