mod types;
mod keys;

pub use types::*;

use crate::api::{ArkimeClient, ArkimeField, ArkimeView, Cont3xtIntegration, Cont3xtResult, Cont3xtView, GraphData, HttpLog, SummaryItem, parse_card};
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::HashMap;

pub struct App {
    pub client: ArkimeClient,
    pub app_mode: AppMode,
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
    pub vr_date_fields: HashMap<String, String>, // dbField -> type ("seconds" or "date")
    pub vr_field_exp_map: HashMap<String, String>, // dbField -> exp (expression field name)
    pub vr_field_friendly_map: HashMap<String, String>, // dbField -> friendlyName
    pub vr_sessions: Vec<Value>,
    pub vr_sessions_total: u64,
    pub vr_sessions_filtered: u64,
    pub vr_session_fields: Vec<String>,
    pub vr_columns: Vec<ColumnDef>,
    pub vr_saved_layouts: Vec<SavedLayout>,
    pub vr_show_column_editor: bool,
    pub vr_column_editor_selected: usize,
    pub vr_column_editor_mode: ColumnEditorMode,
    pub vr_column_editor_available: Vec<ColumnEditorItem>,
    pub vr_column_editor_filter: String,
    pub vr_show_layout_popup: bool,
    pub vr_layout_popup_mode: LayoutPopupMode,
    pub vr_layout_popup_selected: usize,
    pub vr_layout_save_name: String,
    pub vr_layout_save_cursor: usize,
    pub vr_layout_delete_name: String,
    pub vr_layout_filter: String,
    // View state
    pub vr_active_view: Option<String>,      // view id for API calls
    pub vr_active_view_name: Option<String>,  // view name for display
    pub vr_saved_views: Vec<ArkimeView>,
    pub vr_show_view_popup: bool,
    pub vr_view_popup_mode: ViewPopupMode,
    pub vr_view_popup_selected: usize,
    pub vr_view_save_name: String,
    pub vr_view_save_cursor: usize,
    pub vr_view_save_columns: bool,
    pub vr_view_delete_id: String,
    pub vr_view_delete_name: String,
    pub vr_view_filter: String,
    pub vr_view_filter_active: bool,
    pub vr_page_start: u64,
    pub vr_page_size: u64,
    pub vr_selected_session: usize,
    pub vr_table_state: TableState,
    pub vr_session_view: SessionView,
    pub vr_session_detail: Option<SessionDetail>,
    pub vr_detail_action_menu: Option<DetailActionMenu>,
    pub vr_packets_view: Option<crate::api::PacketsData>,
    pub vr_packets_scroll: u16,
    pub vr_packets_raw: bool,
    pub vr_packets_line: LineMode,
    pub show_loading: bool,
    pub loading_owl_x: u16,
    pub loading_owl_dx: i16,
    pub loading_owl_tick: std::time::Instant,
    pub vr_pending_packets_fetch: bool,
    pub vr_pending_summary_fetch: bool,
    pub vr_packets_node_pending: String,
    pub vr_packets_id_pending: String,
    pub vr_packets_total_pending: u64,
    pub vr_sort_column: usize,
    pub vr_sort_desc: bool,
    pub vr_graph_size: GraphSize,
    pub vr_graph_type: GraphType,
    pub vr_graph_data: Option<GraphData>,
    pub status_msg: String,
    pub show_debug: bool,
    pub debug_scroll: usize,
    pub http_log: HttpLog,
    // Stats tab state
    pub vr_stats_tab: StatsTab,
    pub vr_stats_data: Vec<Value>,
    pub vr_stats_total: u64,
    pub vr_stats_filtered: u64,
    pub vr_stats_filter: String,
    pub vr_stats_filter_edit: String,
    pub vr_stats_selected: usize,
    pub vr_stats_table_state: TableState,
    pub vr_stats_view: StatsView,
    pub vr_stats_detail: Option<StatsDetail>,
    pub vr_stats_sort_column: usize,
    pub vr_stats_sort_desc: bool,
    pub vr_stats_last_refresh: std::time::Instant,
    pub visible_rows: usize,
    // Arkime (Summary) tab state
    pub vr_all_fields: Vec<ArkimeField>,
    pub vr_summary_field: String,
    pub vr_summary_data: Vec<SummaryItem>,
    pub vr_summary_metric: SummaryMetric,
    pub vr_summary_selected: usize,
    pub vr_summary_table_state: TableState,
    pub vr_summary_sort: SummarySort,
    pub vr_summary_sort_desc: bool,
    pub vr_field_filter: String,
    pub vr_field_filter_selected: usize,
    // Owl animation
    pub owl_x: f32,
    pub owl_y: f32,
    pub owl_dx: f32,
    pub owl_dy: f32,
    pub owl_frame: usize,
    pub owl_tick: std::time::Instant,
    pub anim_start: std::time::Instant,
    // Cont3xt state
    pub c3_integrations: Vec<Cont3xtIntegration>,
    pub c3_results: Vec<Cont3xtResult>,
    pub c3_selected: usize,           // selected integration result
    pub c3_detail_scroll: u16,        // scroll in detail pane
    pub c3_detail_hscroll: u16,       // horizontal scroll in detail pane
    pub c3_search_total: u64,
    pub c3_search_itype: String,
    pub c3_focus: Cont3xtFocus,       // which pane has focus
    pub c3_raw_view: bool,            // show raw JSON instead of card
    pub c3_disabled_integrations: std::collections::HashSet<String>, // user-toggled off
    pub c3_show_integration_popup: bool,
    pub c3_integration_popup_selected: usize,
    pub c3_integration_popup_filter: String,
    pub c3_integration_popup_filtering: bool,
    pub c3_integration_popup_mode: IntegrationPopupMode, // which sub-view of integration popup
    pub c3_views: Vec<Cont3xtView>,
    pub c3_view_selected: usize,
    pub c3_view_save_name: String,
    pub c3_searching: bool,           // streaming search in progress
    pub c3_pending_search: bool,
    // Cont3xt stats
    pub c3_stats_tab: C3StatsTab,
    pub c3_stats_data: Vec<serde_json::Value>,       // integration stats
    pub c3_itype_stats_data: Vec<serde_json::Value>, // itype stats
    pub c3_stats_selected: usize,
    pub c3_stats_filter: String,
    pub c3_stats_filtering: bool,
    pub c3_stats_sort_col: usize,
    pub c3_stats_sort_desc: bool,
}

impl App {
    pub fn new(base_url: &str, auth_mode: crate::api::AuthMode, username: Option<String>, password: Option<String>, app_mode: AppMode) -> Self {
        let client = ArkimeClient::new(base_url, auth_mode, username, password);
        let http_log = client.http_log();
        let active_tab = app_mode.default_tab();
        Self {
            client,
            app_mode,
            user: Value::Null,
            active_tab,
            time_range: TimeRange::All,
            expression: String::new(),
            expression_edit: String::new(),
            expression_cursor: 0,
            input_mode: InputMode::Normal,
            show_help: false,
            action_menu: None,
            action_prompt: None,
            vr_date_fields: HashMap::new(),
            vr_field_exp_map: HashMap::new(),
            vr_field_friendly_map: HashMap::new(),
            vr_sessions: Vec::new(),
            vr_sessions_total: 0,
            vr_sessions_filtered: 0,
            vr_session_fields: vec![
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
            vr_columns: default_columns(),
            vr_saved_layouts: Vec::new(),
            vr_show_column_editor: false,
            vr_column_editor_selected: 0,
            vr_column_editor_mode: ColumnEditorMode::Browse,
            vr_column_editor_available: Vec::new(),
            vr_column_editor_filter: String::new(),
            vr_show_layout_popup: false,
            vr_layout_popup_mode: LayoutPopupMode::List,
            vr_layout_popup_selected: 0,
            vr_layout_save_name: String::new(),
            vr_layout_save_cursor: 0,
            vr_layout_delete_name: String::new(),
            vr_layout_filter: String::new(),
            vr_active_view: None,
            vr_active_view_name: None,
            vr_saved_views: Vec::new(),
            vr_show_view_popup: false,
            vr_view_popup_mode: ViewPopupMode::List,
            vr_view_popup_selected: 0,
            vr_view_save_name: String::new(),
            vr_view_save_cursor: 0,
            vr_view_save_columns: false,
            vr_view_delete_id: String::new(),
            vr_view_delete_name: String::new(),
            vr_view_filter: String::new(),
            vr_view_filter_active: false,
            vr_page_start: 0,
            vr_page_size: 100,
            vr_selected_session: 0,
            vr_table_state: TableState::default().with_selected(0),
            vr_session_view: SessionView::List,
            vr_session_detail: None,
            vr_detail_action_menu: None,
            vr_packets_view: None,
            vr_packets_scroll: 0,
            vr_packets_raw: false,
            vr_packets_line: LineMode::Hex,
            show_loading: false,
            loading_owl_x: 0,
            loading_owl_dx: 1,
            loading_owl_tick: std::time::Instant::now(),
            vr_pending_packets_fetch: false,
            vr_pending_summary_fetch: false,
            vr_packets_node_pending: String::new(),
            vr_packets_id_pending: String::new(),
            vr_packets_total_pending: 0,
            vr_sort_column: 2,
            vr_sort_desc: true,
            vr_graph_size: GraphSize::Off,
            vr_graph_type: GraphType::Sessions,
            vr_graph_data: None,
            status_msg: String::new(),
            show_debug: false,
            debug_scroll: 0,
            http_log,
            // Stats tab state
            vr_stats_tab: StatsTab::Capture,
            vr_stats_data: Vec::new(),
            vr_stats_total: 0,
            vr_stats_filtered: 0,
            vr_stats_filter: String::new(),
            vr_stats_filter_edit: String::new(),
            vr_stats_selected: 0,
            vr_stats_table_state: TableState::default().with_selected(0),
            vr_stats_view: StatsView::List,
            vr_stats_detail: None,
            vr_stats_sort_column: 0,
            vr_stats_sort_desc: false,
            vr_stats_last_refresh: std::time::Instant::now(),
            visible_rows: 20,
            // Arkime (Summary) tab state
            vr_all_fields: Vec::new(),
            vr_summary_field: String::new(),
            vr_summary_data: Vec::new(),
            vr_summary_metric: SummaryMetric::Sessions,
            vr_summary_selected: 0,
            vr_summary_table_state: TableState::default().with_selected(0),
            vr_summary_sort: SummarySort::Sessions,
            vr_summary_sort_desc: true,
            vr_field_filter: String::new(),
            vr_field_filter_selected: 0,
            // Owl animation
            owl_x: 5.0,
            owl_y: 3.0,
            owl_dx: 1.0,
            owl_dy: 0.5,
            owl_frame: 0,
            owl_tick: std::time::Instant::now(),
            anim_start: std::time::Instant::now(),
            // Cont3xt state
            c3_integrations: Vec::new(),
            c3_results: Vec::new(),
            c3_selected: 0,
            c3_detail_scroll: 0,
            c3_detail_hscroll: 0,
            c3_search_total: 0,
            c3_search_itype: String::new(),
            c3_focus: Cont3xtFocus::Results,
            c3_raw_view: false,
            c3_disabled_integrations: std::collections::HashSet::new(),
            c3_show_integration_popup: false,
            c3_integration_popup_selected: 0,
            c3_integration_popup_filter: String::new(),
            c3_integration_popup_filtering: false,
            c3_integration_popup_mode: IntegrationPopupMode::Integrations,
            c3_views: Vec::new(),
            c3_view_selected: 0,
            c3_view_save_name: String::new(),
            c3_searching: false,
            c3_pending_search: false,
            c3_stats_tab: C3StatsTab::Integrations,
            c3_stats_data: Vec::new(),
            c3_itype_stats_data: Vec::new(),
            c3_stats_selected: 0,
            c3_stats_filter: String::new(),
            c3_stats_filtering: false,
            c3_stats_sort_col: 0,
            c3_stats_sort_desc: false,
        }
    }

    pub fn is_detail_view(&self) -> bool {
        self.vr_session_view == SessionView::Detail || self.vr_stats_view == StatsView::Detail
    }

    pub fn tabs(&self) -> &'static [Tab] {
        self.app_mode.tabs()
    }

    pub fn next_tab(&mut self) {
        let tabs = self.tabs();
        let idx = tabs.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = self.tabs();
        let idx = tabs.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }

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

    async fn refresh_for_active_tab(&mut self) {
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
        let columns = self.vr_stats_tab.columns();
        let sort_field = columns.get(self.vr_stats_sort_column)
            .map(|(f, _, _)| *f)
            .unwrap_or(columns[0].0);

        let result = match self.vr_stats_tab {
            StatsTab::Capture => self.client.vr_get_stats(&self.vr_stats_filter, sort_field, self.vr_stats_sort_desc).await,
            StatsTab::DBStats => self.client.vr_get_esstats(&self.vr_stats_filter, sort_field, self.vr_stats_sort_desc).await,
            StatsTab::DBIndices => self.client.vr_get_esindices(&self.vr_stats_filter, sort_field, self.vr_stats_sort_desc).await,
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
        self.show_loading = true;
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
}
