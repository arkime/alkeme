mod types;
mod keys;
mod keys_viewer;
mod keys_cont3xt;
mod keys_parliament;
mod keys_wise;

pub use types::*;

use crate::api::{ArkimeClient, ArkimeField, ArkimeView, Cont3xtIntegration, Cont3xtLinkGroup, Cont3xtOverview, Cont3xtResult, Cont3xtView, GraphData, HttpLog, PlCluster, PlClusterStats, PlGroup, PlIssue, SummaryItem, WsQueryResult, WsSourceStats, WsStats, WsTypeStats, parse_card};
use chrono::{Datelike, Duration, Timelike, Utc};
use crossterm::event::KeyCode;
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::HashMap;

/// Handle common text input key events (char insert, backspace, cursor movement).
/// Returns true if the key was handled.
pub fn handle_text_input_key(key: KeyCode, text: &mut String, cursor: &mut usize) -> bool {
    match key {
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                text.remove(*cursor);
            }
            true
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        KeyCode::Right => {
            *cursor = (*cursor + 1).min(text.len());
            true
        }
        KeyCode::Char(c) => {
            text.insert(*cursor, c);
            *cursor += 1;
            true
        }
        _ => false,
    }
}

pub struct App {
    pub client: ArkimeClient,
    pub app_mode: AppMode,
    pub title_name: String,
    pub user: Value,
    pub active_tab: Tab,
    pub time_range: TimeRange,
    pub expression: String,
    pub expression_edit: String,
    pub expression_cursor: usize,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub confirm_dialog: Option<ConfirmDialog>,
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
    pub debug_selected: usize,
    pub debug_expanded: bool,
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
    pub c3_overviews: Vec<Cont3xtOverview>,
    pub c3_results: Vec<Cont3xtResult>,
    pub c3_selected: usize,           // index into c3_tree_order
    pub c3_tree_order: Vec<C3TreeItem>, // tree items in display order
    pub c3_tree_roots: Vec<usize>,    // indices into c3_tree_order where each root indicator starts
    pub c3_detail_scroll: u16,        // scroll in detail pane
    pub c3_detail_hscroll: u16,       // horizontal scroll in detail pane
    pub c3_detail_filter: String,     // filter string for detail pane
    pub c3_search_total: u64,
    pub c3_search_sent: u64,
    pub c3_search_itype: String,
    // indicator parent map: (child_indicator, child_itype) -> [(parent_query, parent_itype), ...]
    pub c3_indicator_parents: HashMap<(String, String), Vec<(String, String)>>,
    // init-ordered indicators: (itype, query) in the order from the init response
    pub c3_init_indicators: Vec<(String, String)>,
    pub c3_focus: Cont3xtFocus,       // which pane has focus
    pub c3_raw_view: bool,            // show raw JSON instead of card
    pub c3_show_card_popup: bool,     // show card definition popup
    pub c3_card_popup_scroll: u16,    // scroll offset for card popup
    pub c3_show_overview_popup: bool,  // overview selector popup
    pub c3_overview_popup_selected: usize,
    pub c3_overview_popup_filter: String,
    pub c3_overview_popup_filtering: bool,
    pub c3_selected_overviews: HashMap<String, String>, // itype -> overview id
    pub c3_disabled_integrations: std::collections::HashSet<String>, // user-toggled off
    pub c3_show_integration_popup: bool,
    pub c3_integration_popup_selected: usize,
    pub c3_integration_popup_filter: String,
    pub c3_integration_popup_filtering: bool,
    pub c3_integration_popup_mode: IntegrationPopupMode, // which sub-view of integration popup
    pub c3_views: Vec<Cont3xtView>,
    pub c3_view_selected: usize,
    pub c3_view_save_name: String,
    pub c3_active_view_id: Option<String>,
    pub c3_active_view_name: Option<String>,
    pub c3_searching: bool,           // streaming search in progress
    pub c3_pending_search: bool,
    pub c3_no_cache: bool,
    pub c3_tags: Vec<String>,         // tags sent with search query
    pub c3_tags_edit: String,         // edit buffer for tags popup
    pub c3_show_tags_popup: bool,     // tag editor popup visible
    pub c3_save_json_prompt: Option<String>, // filename prompt for JSON export
    pub c3_save_json_path: Option<String>,   // headless: save JSON to file and quit when search completes
    // Cont3xt date range
    pub c3_start_date: chrono::DateTime<Utc>,
    pub c3_stop_date: chrono::DateTime<Utc>,
    pub c3_show_date_popup: bool,
    pub c3_date_start_edit: String,   // edit buffer for start date
    pub c3_date_stop_edit: String,    // edit buffer for stop date
    pub c3_date_field: u8,            // 0 = start, 1 = stop
    // Cont3xt link groups
    pub c3_link_groups: Vec<Cont3xtLinkGroup>,
    pub c3_show_link_popup: bool,
    pub c3_link_popup_selected: usize,
    pub c3_link_popup_filter: String,
    pub c3_link_popup_filtering: bool,
    pub c3_link_flat: Vec<(String, String, String, String)>, // (group_name, link_name, url, info) filtered by itype
    // Cont3xt stats
    pub c3_stats_tab: C3StatsTab,
    pub c3_stats_data: Vec<serde_json::Value>,       // integration stats
    pub c3_itype_stats_data: Vec<serde_json::Value>, // itype stats
    pub c3_stats_selected: usize,
    pub c3_stats_table_state: ratatui::widgets::TableState,
    pub c3_stats_filter: String,
    pub c3_stats_filtering: bool,
    pub c3_stats_sort_col: usize,
    pub c3_stats_sort_desc: bool,
    // Cont3xt history
    pub c3_history_data: Vec<serde_json::Value>,
    pub c3_history_total: usize,
    pub c3_history_page: usize,  // 1-indexed
    pub c3_history_selected: usize,
    pub c3_history_table_state: ratatui::widgets::TableState,
    pub c3_history_filter: String,
    pub c3_history_filtering: bool,
    pub c3_history_sort_col: usize,
    pub c3_history_sort_desc: bool,
    pub c3_history_loaded: bool,
    // Parliament state
    pub pl_groups: Vec<PlGroup>,
    pub pl_stats: HashMap<String, PlClusterStats>,
    pub pl_issues_map: HashMap<String, Vec<PlIssue>>,
    pub pl_issues: Vec<PlIssue>,
    pub pl_issues_filter: String,
    pub pl_issues_filter_edit: String,
    pub pl_issues_sort: PlIssueSort,
    pub pl_issues_sort_desc: bool,
    pub pl_issues_selected: usize,
    pub pl_issues_table_state: TableState,
    pub pl_selected_group: usize,
    pub pl_selected_cluster: usize,
    pub pl_dashboard_scroll: u16,
    pub pl_last_refresh: std::time::Instant,
    pub pl_show_detail: bool,
    pub pl_detail_scroll: u16,
    // Flat list of (group_idx, cluster_idx) for dashboard navigation
    pub pl_cluster_list: Vec<(usize, usize)>,
    // Saved parliament client for returning from viewer/cont3xt mode (Ctrl+P)
    pub pl_saved_client: Option<ArkimeClient>,
    pub pl_cont3xt_url: String,
    pub pl_wise_url: String,
    pub pl_saved_viewer_expression: String,
    pub pl_saved_c3_expression: String,
    pub force_clear: bool, // force terminal clear after okta redirect

    // WISE mode fields (ws_ prefix)
    pub ws_stats: Option<WsStats>,
    pub ws_stats_tab: WsStatsTab,
    pub ws_stats_filter: String,
    pub ws_stats_filter_edit: String,
    pub ws_stats_selected: usize,
    pub ws_last_refresh: std::time::Instant,
    pub ws_sources: Vec<String>,
    pub ws_types: Vec<String>,
    pub ws_query_source: String,  // selected source for query ("any" = all)
    pub ws_query_type: String,    // selected type for query (default "ip")
    pub ws_query_value: String,
    pub ws_query_value_edit: String,
    pub ws_query_results: Vec<WsQueryResult>,
    pub ws_query_selected: usize,

    // Cached background buffer for popup double-buffering
    pub popup_bg_cache: Option<ratatui::buffer::Buffer>,
}

impl App {
    pub fn new(base_url: &str, auth_mode: crate::api::AuthMode, username: Option<String>, password: Option<String>, app_mode: AppMode) -> Self {
        let client = ArkimeClient::new(base_url, auth_mode, username, password, None);
        let http_log = client.http_log();
        let active_tab = app_mode.default_tab();
        Self {
            client,
            app_mode,
            title_name: base_url.to_string(),
            user: Value::Null,
            active_tab,
            time_range: TimeRange::Hours1,
            expression: String::new(),
            expression_edit: String::new(),
            expression_cursor: 0,
            input_mode: InputMode::Normal,
            show_help: false,
            confirm_dialog: None,
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
            debug_selected: 0,
            debug_expanded: false,
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
            c3_overviews: Vec::new(),
            c3_results: Vec::new(),
            c3_selected: 0,
            c3_tree_order: Vec::new(),
            c3_tree_roots: Vec::new(),
            c3_detail_scroll: 0,
            c3_detail_hscroll: 0,
            c3_detail_filter: String::new(),
            c3_search_total: 0,
            c3_search_sent: 0,
            c3_search_itype: String::new(),
            c3_indicator_parents: HashMap::new(),
            c3_init_indicators: Vec::new(),
            c3_focus: Cont3xtFocus::Results,
            c3_raw_view: false,
            c3_show_card_popup: false,
            c3_card_popup_scroll: 0,
            c3_show_overview_popup: false,
            c3_overview_popup_selected: 0,
            c3_overview_popup_filter: String::new(),
            c3_overview_popup_filtering: false,
            c3_selected_overviews: HashMap::new(),
            c3_disabled_integrations: std::collections::HashSet::new(),
            c3_show_integration_popup: false,
            c3_integration_popup_selected: 0,
            c3_integration_popup_filter: String::new(),
            c3_integration_popup_filtering: false,
            c3_integration_popup_mode: IntegrationPopupMode::Integrations,
            c3_views: Vec::new(),
            c3_view_selected: 0,
            c3_view_save_name: String::new(),
            c3_active_view_id: None,
            c3_active_view_name: None,
            c3_searching: false,
            c3_pending_search: false,
            c3_no_cache: false,
            c3_tags: Vec::new(),
            c3_tags_edit: String::new(),
            c3_show_tags_popup: false,
            c3_save_json_prompt: None,
            c3_save_json_path: None,
            c3_start_date: Utc::now() - Duration::days(7),
            c3_stop_date: Utc::now(),
            c3_show_date_popup: false,
            c3_date_start_edit: String::from("-7d"),
            c3_date_stop_edit: String::from("now"),
            c3_date_field: 0,
            c3_link_groups: Vec::new(),
            c3_show_link_popup: false,
            c3_link_popup_selected: 0,
            c3_link_popup_filter: String::new(),
            c3_link_popup_filtering: false,
            c3_link_flat: Vec::new(),
            c3_stats_tab: C3StatsTab::Integrations,
            c3_stats_data: Vec::new(),
            c3_itype_stats_data: Vec::new(),
            c3_stats_selected: 0,
            c3_stats_table_state: ratatui::widgets::TableState::default(),
            c3_stats_filter: String::new(),
            c3_stats_filtering: false,
            c3_stats_sort_col: 0,
            c3_stats_sort_desc: false,
            c3_history_data: Vec::new(),
            c3_history_total: 0,
            c3_history_page: 1,
            c3_history_selected: 0,
            c3_history_table_state: ratatui::widgets::TableState::default(),
            c3_history_filter: String::new(),
            c3_history_filtering: false,
            c3_history_sort_col: 0,
            c3_history_sort_desc: true,
            c3_history_loaded: false,
            // Parliament state
            pl_groups: Vec::new(),
            pl_stats: HashMap::new(),
            pl_issues_map: HashMap::new(),
            pl_issues: Vec::new(),
            pl_issues_filter: String::new(),
            pl_issues_filter_edit: String::new(),
            pl_issues_sort: PlIssueSort::LastNoticed,
            pl_issues_sort_desc: true,
            pl_issues_selected: 0,
            pl_issues_table_state: TableState::default().with_selected(0),
            pl_selected_group: 0,
            pl_selected_cluster: 0,
            pl_dashboard_scroll: 0,
            pl_last_refresh: std::time::Instant::now(),
            pl_show_detail: false,
            pl_detail_scroll: 0,
            pl_cluster_list: Vec::new(),
            pl_saved_client: None,
            pl_cont3xt_url: String::new(),
            pl_wise_url: String::new(),
            pl_saved_viewer_expression: String::new(),
            pl_saved_c3_expression: String::new(),
            force_clear: false,

            // WISE state
            ws_stats: None,
            ws_stats_tab: WsStatsTab::Sources,
            ws_stats_filter: String::new(),
            ws_stats_filter_edit: String::new(),
            ws_stats_selected: 0,
            ws_last_refresh: std::time::Instant::now(),
            ws_sources: Vec::new(),
            ws_types: Vec::new(),
            ws_query_source: "any".into(),
            ws_query_type: "ip".into(),
            ws_query_value: String::new(),
            ws_query_value_edit: String::new(),
            ws_query_results: Vec::new(),
            ws_query_selected: 0,

            popup_bg_cache: None,
        }
    }

    pub fn is_detail_view(&self) -> bool {
        self.vr_session_view == SessionView::Detail || self.vr_stats_view == StatsView::Detail
    }

    /// Returns true if any popup overlay is open that could use background caching
    pub fn has_popup_open(&self) -> bool {
        self.confirm_dialog.is_some()
            || self.c3_show_card_popup
            || self.c3_show_overview_popup
            || self.c3_show_link_popup
            || self.c3_show_integration_popup
            || self.c3_show_tags_popup
            || self.c3_show_date_popup
            || self.c3_save_json_prompt.is_some()
            || self.show_help
            || self.show_debug
    }

    /// Returns true if q should close a popup instead of quitting the app
    pub fn q_closes_popup(&self) -> bool {
        self.confirm_dialog.is_some()
            || self.show_help || self.show_debug || self.pl_show_detail
            || self.vr_show_column_editor || self.vr_show_layout_popup || self.vr_show_view_popup
            || self.c3_show_integration_popup || self.c3_show_overview_popup
            || self.c3_show_link_popup || self.c3_show_card_popup
            || self.c3_show_tags_popup || self.c3_show_date_popup
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

    pub async fn handle_confirm(&mut self, action: String) {
        if let Some(index) = action.strip_prefix("delete_esindex:") {
            match self.client.vr_delete_esindex(index).await {
                Ok(_) => {
                    self.status_msg = format!("Deleted index '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error deleting index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("optimize_esindex:") {
            match self.client.vr_optimize_esindex(index).await {
                Ok(_) => {
                    self.status_msg = format!("Force merge started for '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error force merging index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("close_esindex:") {
            match self.client.vr_close_esindex(index).await {
                Ok(_) => {
                    self.status_msg = format!("Closed index '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error closing index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("open_esindex:") {
            match self.client.vr_open_esindex(index).await {
                Ok(_) => {
                    self.status_msg = format!("Opened index '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error opening index: {e}"),
            }
        } else if let Some(rest) = action.strip_prefix("esshards:") {
            // format: "esshards:kind:value:action" where action is "exclude" or "include"
            // Use rfind to handle IPv6 colons in value
            if let Some(last_colon) = rest.rfind(':') {
                let op = &rest[last_colon+1..];
                let before_op = &rest[..last_colon];
                if let Some(first_colon) = before_op.find(':') {
                    let kind = &before_op[..first_colon];
                    let value = &before_op[first_colon+1..];
                    match self.client.vr_esshards_toggle(kind, value, op).await {
                        Ok(_) => {
                            let label = if kind == "ip" { "IP" } else { "node" };
                            self.status_msg = format!("{} {label} '{value}'", if op == "exclude" { "Excluded" } else { "Included" });
                            let detail_name = self.vr_stats_detail.as_ref()
                                .map(|d| crate::api::str_val(&d.data, "name"));
                            let detail_scroll = self.vr_stats_detail.as_ref().map(|d| d.scroll).unwrap_or(0);
                            let detail_filter = self.vr_stats_detail.as_ref().map(|d| d.filter.clone()).unwrap_or_default();
                            self.vr_fetch_stats().await;
                            if let Some(name) = detail_name {
                                if let Some(row) = self.vr_stats_data.iter().find(|r| crate::api::str_val(r, "name") == name) {
                                    self.vr_stats_detail = Some(StatsDetail { data: row.clone(), scroll: detail_scroll, filter: detail_filter });
                                }
                            }
                        }
                        Err(e) => self.status_msg = format!("Error: {e}"),
                    }
                }
            }
        }
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
        for result in &self.c3_results {
            if result.data.is_null() { continue; }
            let indicator_obj = combined.entry(&result.indicator)
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let serde_json::Value::Object(map) = indicator_obj {
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

    /// Build the flat list of links filtered by selected result's itype
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
                self.c3_link_flat.push((group.name.clone(), link.name.clone(), url, link.info.clone()));
            }
        }
        if self.c3_link_popup_selected >= self.c3_link_flat.len() {
            self.c3_link_popup_selected = self.c3_link_flat.len().saturating_sub(1);
        }
    }

    // Parliament methods

    pub async fn pl_fetch_data(&mut self) {
        match self.client.pl_get_parliament().await {
            Ok(parliament) => {
                self.pl_cont3xt_url = parliament.settings.general.cont3xt_url.clone();
                self.pl_wise_url = parliament.settings.general.wise_url.clone();
                self.pl_groups = parliament.groups;
                self.pl_rebuild_cluster_list();
                self.status_msg = format!("{} groups loaded", self.pl_groups.len());
            }
            Err(e) => self.status_msg = format!("Error fetching parliament: {e}"),
        }
        match self.client.pl_get_stats().await {
            Ok(stats) => self.pl_stats = stats,
            Err(e) => self.status_msg = format!("Error fetching stats: {e}"),
        }
        match self.client.pl_get_issues_map().await {
            Ok(issues) => self.pl_issues_map = issues,
            Err(e) => self.status_msg = format!("Error fetching issues: {e}"),
        }
        self.pl_last_refresh = std::time::Instant::now();
    }

    pub async fn pl_fetch_issues(&mut self) {
        match self.client.pl_get_issues().await {
            Ok(issues) => {
                let count = issues.len();
                self.pl_issues = issues;
                self.pl_sort_issues();
                self.status_msg = format!("{} issues", count);
            }
            Err(e) => self.status_msg = format!("Error fetching issues: {e}"),
        }
    }

    pub(crate) fn pl_rebuild_cluster_list(&mut self) {
        self.pl_cluster_list.clear();
        for (gi, group) in self.pl_groups.iter().enumerate() {
            for (ci, _cluster) in group.clusters.iter().enumerate() {
                self.pl_cluster_list.push((gi, ci));
            }
        }
    }

    pub(crate) fn pl_sort_issues(&mut self) {
        let sort = self.pl_issues_sort;
        let desc = self.pl_issues_sort_desc;
        self.pl_issues.sort_by(|a, b| {
            let cmp = match sort {
                PlIssueSort::Cluster => a.cluster.to_lowercase().cmp(&b.cluster.to_lowercase()),
                PlIssueSort::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                PlIssueSort::Severity => a.severity.cmp(&b.severity),
                PlIssueSort::FirstNoticed => a.first_noticed.cmp(&b.first_noticed),
                PlIssueSort::LastNoticed => a.last_noticed.cmp(&b.last_noticed),
            };
            if desc { cmp.reverse() } else { cmp }
        });
    }

    /// Get the currently selected cluster on the dashboard
    pub(crate) fn pl_selected_cluster_ref(&self) -> Option<&PlCluster> {
        if self.pl_cluster_list.is_empty() {
            return None;
        }
        let nav_idx = self.pl_dashboard_nav_index();
        if nav_idx < self.pl_cluster_list.len() {
            let (gi, ci) = self.pl_cluster_list[nav_idx];
            self.pl_groups.get(gi).and_then(|g| g.clusters.get(ci))
        } else {
            None
        }
    }

    /// Get flat index from current group/cluster selection
    pub(crate) fn pl_dashboard_nav_index(&self) -> usize {
        self.pl_cluster_list.iter().position(|&(gi, ci)| gi == self.pl_selected_group && ci == self.pl_selected_cluster).unwrap_or(0)
    }

    /// Get filtered issues list
    pub(crate) fn pl_filtered_issues(&self) -> Vec<&PlIssue> {
        let filter = self.pl_issues_filter.to_lowercase();
        self.pl_issues.iter().filter(|issue| {
            if filter.is_empty() {
                return true;
            }
            issue.cluster.to_lowercase().contains(&filter)
                || issue.title.to_lowercase().contains(&filter)
                || issue.message.to_lowercase().contains(&filter)
                || issue.node.to_lowercase().contains(&filter)
                || issue.severity.to_lowercase().contains(&filter)
        }).collect()
    }

    // --- WISE methods ---

    pub async fn ws_fetch_stats(&mut self) {
        match self.client.ws_get_stats(&self.ws_stats_filter).await {
            Ok(stats) => {
                self.status_msg = format!("{} sources, {} types", stats.sources.len(), stats.types.len());
                self.ws_stats = Some(stats);
            }
            Err(e) => self.status_msg = format!("Error fetching WISE stats: {e}"),
        }
        self.ws_last_refresh = std::time::Instant::now();
    }

    pub async fn ws_fetch_sources_types(&mut self) {
        match self.client.ws_get_sources().await {
            Ok(s) => self.ws_sources = s,
            Err(e) => self.status_msg = format!("Error fetching sources: {e}"),
        }
        match self.client.ws_get_types("").await {
            Ok(t) => self.ws_types = t,
            Err(e) => self.status_msg = format!("Error fetching types: {e}"),
        }
    }

    pub async fn ws_run_query(&mut self) {
        if self.ws_query_value.is_empty() {
            self.status_msg = "Enter a value to query".into();
            return;
        }
        match self.client.ws_query(&self.ws_query_source, &self.ws_query_type, &self.ws_query_value).await {
            Ok(results) => {
                let count = results.len();
                self.ws_query_results = results;
                self.ws_query_selected = 0;
                self.status_msg = if count == 0 {
                    "No results found".into()
                } else {
                    format!("{} results", count)
                };
            }
            Err(e) => self.status_msg = format!("Query error: {e}"),
        }
    }

    pub fn ws_filtered_sources(&self) -> Vec<&WsSourceStats> {
        let Some(stats) = &self.ws_stats else { return vec![] };
        stats.sources.iter().collect()
    }

    pub fn ws_filtered_types(&self) -> Vec<&WsTypeStats> {
        let Some(stats) = &self.ws_stats else { return vec![] };
        stats.types.iter().collect()
    }

    pub fn enter_expression_mode(&mut self) {
        self.expression_edit = self.expression.clone();
        self.expression_cursor = self.expression_edit.len();
        self.input_mode = InputMode::Expression;
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

/// Decode percent-encoded characters in a URL (e.g., %20 → space, %22 → ").
/// Used on macOS because `open` re-encodes the URL, causing double-encoding.
#[cfg(target_os = "macos")]
pub fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_val(h), hex_val(l)) {
                    out.push((hv << 4 | lv) as char);
                    continue;
                }
                // Not valid hex, keep as-is
                out.push('%');
                out.push(h as char);
                out.push(l as char);
            } else {
                out.push('%');
                if let Some(h) = hi { out.push(h as char); }
            }
        } else {
            out.push(b as char);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse a date input string (Splunk-style relative or absolute) into a DateTime<Utc>.
/// Supports: "now", relative like "-5h", "+1d", "-1w", "-3M", "-1y", "@day",
/// "+2h@day", and absolute ISO 8601 / YYYY/MM/DDTHH:mm:ss formats.
fn parse_date_input(s: &str) -> Option<chrono::DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() || s == "now" {
        return Some(Utc::now());
    }

    // Relative: +/-N unit[@snap]
    if s.starts_with('+') || s.starts_with('-') {
        let sign: i64 = if s.starts_with('-') { -1 } else { 1 };
        let rest = &s[1..];

        // Split on @ for optional snap
        let (time_part, _snap_part) = if let Some(at) = rest.find('@') {
            (&rest[..at], Some(&rest[at + 1..]))
        } else {
            (rest, None)
        };

        // Parse number + unit
        let num_end = time_part.find(|c: char| !c.is_ascii_digit()).unwrap_or(time_part.len());
        let num: i64 = if num_end == 0 { 1 } else { time_part[..num_end].parse().ok()? };
        let unit = &time_part[num_end..];

        let dur = match unit {
            "s" | "sec" | "secs" | "second" | "seconds" => Duration::seconds(num),
            "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(num),
            "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(num),
            "d" | "day" | "days" => Duration::days(num),
            "w" | "week" | "weeks" => Duration::weeks(num),
            "M" | "mon" | "mons" | "month" | "months" => Duration::days(num * 30),
            "q" | "qtr" | "qtrs" | "quarter" | "quarters" => Duration::days(num * 91),
            "y" | "yr" | "yrs" | "year" | "years" => Duration::days(num * 365),
            _ => return None,
        };

        return Some(Utc::now() + dur * sign as i32);
    }

    // @snap only (e.g., "@day" = start of today)
    if s.starts_with('@') {
        let snap = &s[1..];
        let now = Utc::now();
        return match snap {
            "s" | "sec" | "second" | "seconds" => Some(now.with_nanosecond(0)?),
            "m" | "min" | "minute" | "minutes" => Some(now.with_nanosecond(0)?.with_second(0)?),
            "h" | "hr" | "hour" | "hours" => Some(now.with_nanosecond(0)?.with_second(0)?.with_minute(0)?),
            "d" | "day" | "days" => Some(now.date_naive().and_hms_opt(0, 0, 0)?.and_utc()),
            "w" | "week" | "weeks" => {
                let weekday = now.weekday().num_days_from_sunday();
                Some((now.date_naive() - Duration::days(weekday as i64)).and_hms_opt(0, 0, 0)?.and_utc())
            },
            "M" | "mon" | "month" | "months" => {
                Some(now.date_naive().with_day(1)?.and_hms_opt(0, 0, 0)?.and_utc())
            },
            "y" | "yr" | "year" | "years" => {
                Some(chrono::NaiveDate::from_ymd_opt(now.year(), 1, 1)?.and_hms_opt(0, 0, 0)?.and_utc())
            },
            _ => None,
        };
    }

    // Absolute: try ISO 8601 and YYYY/MM/DDTHH:mm:ss
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y/%m/%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }

    None
}

/// Refang an indicator (reverse of defang: hXXp→http, [.]→.)
fn refang(s: &str) -> String {
    s.replace("hXXp", "http").replace("[.]", ".")
}

/// Convert a YYYY-MM-DD style format string to chrono strftime format
fn convert_date_format(fmt: &str) -> String {
    fmt.replace("YYYY", "%Y")
        .replace("YY", "%y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("hh", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

/// Parse a timeSnap string like "1d", "-1w", "2h" into a chrono Duration
fn parse_time_snap(snap: &str) -> Option<Duration> {
    let snap = snap.trim();
    if snap.is_empty() { return None; }
    let (negative, rest) = if let Some(s) = snap.strip_prefix('-') { (true, s) } else { (false, snap) };
    let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let num: i64 = rest[..num_end].parse().ok()?;
    let unit = &rest[num_end..];
    let dur = match unit {
        "s" => Duration::seconds(num),
        "m" => Duration::minutes(num),
        "h" => Duration::hours(num),
        "d" => Duration::days(num),
        "w" => Duration::weeks(num),
        _ => return None,
    };
    Some(if negative { -dur } else { dur })
}

/// Substitute all placeholders in a link URL
fn substitute_link_url(
    url: &str,
    indicator: &str,
    itype: &str,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    indicators_by_itype: &HashMap<String, Vec<String>>,
    top_indicators_by_itype: &HashMap<String, Vec<String>>,
) -> String {
    let refanged = refang(indicator);
    let num_days = (end - start).num_days();
    let num_hours = (end - start).num_hours();

    // Simple placeholders
    let result = url
        .replace("${indicator}", &refanged)
        .replace("${type}", itype)
        .replace("${numDays}", &num_days.to_string())
        .replace("${numHours}", &num_hours.to_string())
        .replace("${startDate}", &start.format("%Y-%m-%d").to_string())
        .replace("${endDate}", &end.format("%Y-%m-%d").to_string())
        .replace("${startTS}", &start.format("%Y-%m-%dT%H.%M.%SZ").to_string())
        .replace("${endTS}", &end.format("%Y-%m-%dT%H.%M.%SZ").to_string())
        .replace("${startEpoch}", &start.timestamp().to_string())
        .replace("${endEpoch}", &end.timestamp().to_string())
        .replace("${startSplunk}", &start.format("%m/%d/%Y:%H:%M:%S").to_string())
        .replace("${endSplunk}", &end.format("%m/%d/%Y:%H:%M:%S").to_string());

    // ${start,...}, ${end,...}, and ${array,...} — scan for remaining ${ placeholders
    let mut out = String::with_capacity(result.len());
    let mut rest = result.as_str();
    while let Some(pos) = rest.find("${") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 2..];
        // Find matching closing brace, accounting for nested JSON braces and escaped chars
        if let Some(close) = find_matching_brace(after) {
            let inner = &after[..close];
            if let Some(replacement) = process_advanced_placeholder(inner, start, end, indicators_by_itype, top_indicators_by_itype) {
                out.push_str(&replacement);
            }
            // On parse failure, placeholder is removed
            rest = &after[close + 1..];
        } else {
            out.push_str("${");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Find the position of the closing `}` for a `${...}` placeholder,
/// accounting for nested JSON braces and backslash-escaped characters.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1; // we're already inside the opening ${
    let mut chars = s.char_indices();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '\\' => { chars.next(); } // skip escaped char
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

/// Process ${start,...}, ${end,...}, and ${array,...} placeholders
fn process_advanced_placeholder(
    inner: &str,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    indicators_by_itype: &HashMap<String, Vec<String>>,
    top_indicators_by_itype: &HashMap<String, Vec<String>>,
) -> Option<String> {
    let comma_pos = inner.find(',')?;
    let keyword = inner[..comma_pos].trim();
    let json_str = inner[comma_pos + 1..].trim();

    match keyword {
        "start" | "end" => {
            let obj: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let fmt = obj.get("format").and_then(|v| v.as_str()).unwrap_or("YYYY-MM-DD");
            let chrono_fmt = convert_date_format(fmt);
            let mut dt = if keyword == "start" { start } else { end };
            if let Some(snap_str) = obj.get("timeSnap").and_then(|v| v.as_str()) {
                if let Some(dur) = parse_time_snap(snap_str) {
                    dt = dt + dur;
                }
            }
            Some(dt.format(&chrono_fmt).to_string())
        }
        "array" => {
            let obj: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let target_itype = obj.get("iType").and_then(|v| v.as_str())?;
            let include = obj.get("include").and_then(|v| v.as_str()).unwrap_or("all");
            let sep = obj.get("sep").and_then(|v| v.as_str()).unwrap_or(",");
            let quote = obj.get("quote").and_then(|v| v.as_str()).unwrap_or("");

            let source = if include == "top" { top_indicators_by_itype } else { indicators_by_itype };
            let values = source.get(target_itype)?;
            let formatted: Vec<String> = values.iter()
                .map(|v| format!("{quote}{v}{quote}"))
                .collect();
            Some(formatted.join(sep))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_array_with_quotes_and_sep() {
        let mut all = HashMap::new();
        all.insert("domain".to_string(), vec!["threathole.com".to_string(), "threattroll.com".to_string()]);
        let top = all.clone();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = r#"str_representation:%20(%20${array,{"iType":"domain","include":"all","sep":" OR ","quote":"\""}}%20)"#;
        let result = substitute_link_url(url, "threathole.com", "domain", start, now, &all, &top);
        assert_eq!(result, r#"str_representation:%20(%20"threathole.com" OR "threattroll.com"%20)"#);
    }

    #[test]
    fn test_substitute_simple_placeholders() {
        let all = HashMap::new();
        let top = HashMap::new();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = "https://example.com?q=${indicator}&t=${type}&d=${numDays}&h=${numHours}";
        let result = substitute_link_url(url, "test[.]com", "domain", start, now, &all, &top);
        assert_eq!(result, "https://example.com?q=test.com&t=domain&d=7&h=168");
    }

    #[test]
    fn test_substitute_custom_date_format() {
        let all = HashMap::new();
        let top = HashMap::new();
        let end = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:00Z").unwrap().with_timezone(&Utc);
        let start = end - Duration::days(7);

        let url = r#"https://example.com?s=${start,{"format":"DD.MM.YYYY"}}&e=${end,{"format":"YYYY-MM-DDThh:mm:ssZ"}}"#;
        let result = substitute_link_url(url, "test", "ip", start, end, &all, &top);
        assert_eq!(result, "https://example.com?s=08.06.2024&e=2024-06-15T12:30:00Z");
    }

    #[test]
    fn test_find_matching_brace() {
        assert_eq!(find_matching_brace("simple}"), Some(6));
        assert_eq!(find_matching_brace(r#"array,{"iType":"ip"}}"#), Some(20));
        assert_eq!(find_matching_brace(r#"array,{"quote":"\""}}"#), Some(20));
        assert_eq!(find_matching_brace("no close"), None);
    }

    #[test]
    fn test_refang() {
        assert_eq!(refang("hXXps://evil[.]com"), "https://evil.com");
        assert_eq!(refang("normal.com"), "normal.com");
    }

    #[test]
    fn test_substitute_array_full_url() {
        let mut all = HashMap::new();
        all.insert("domain".to_string(), vec!["threathole.com".to_string(), "threattroll.com".to_string()]);
        let top = all.clone();
        let now = Utc::now();
        let start = now - Duration::days(7);

        let url = r#"https://HOST:9999/app/dashboards#/view/123?_a=(query:(query:'str_representation:%20(%20${array,{"iType":"domain","include":"all","sep":" OR ","quote":"\""}}%20)'))"#;
        let result = substitute_link_url(url, "threathole.com", "domain", start, now, &all, &top);
        // %20 in template preserved as-is, quotes and spaces from sep are literal
        assert_eq!(result, r#"https://HOST:9999/app/dashboards#/view/123?_a=(query:(query:'str_representation:%20(%20"threathole.com" OR "threattroll.com"%20)'))"#);
    }

    #[test]
    fn test_parse_time_snap() {
        assert_eq!(parse_time_snap("1d"), Some(Duration::days(1)));
        assert_eq!(parse_time_snap("-1w"), Some(Duration::weeks(-1)));
        assert_eq!(parse_time_snap("2h"), Some(Duration::hours(2)));
        assert_eq!(parse_time_snap(""), None);
    }

    #[test]
    fn test_parse_date_input() {
        // "now" should be close to current time
        let now = Utc::now();
        let parsed = parse_date_input("now").unwrap();
        assert!((parsed - now).num_seconds().abs() < 2);

        // Empty string = now
        let parsed = parse_date_input("").unwrap();
        assert!((parsed - now).num_seconds().abs() < 2);

        // Relative: -7d
        let parsed = parse_date_input("-7d").unwrap();
        let expected = now - Duration::days(7);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Relative: -1h
        let parsed = parse_date_input("-1h").unwrap();
        let expected = now - Duration::hours(1);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Relative: +30m
        let parsed = parse_date_input("+30m").unwrap();
        let expected = now + Duration::minutes(30);
        assert!((parsed - expected).num_seconds().abs() < 2);

        // Absolute ISO 8601
        let parsed = parse_date_input("2024-06-15T12:30:00Z").unwrap();
        assert_eq!(parsed.year(), 2024);
        assert_eq!(parsed.month(), 6);
        assert_eq!(parsed.day(), 15);

        // Absolute date only
        let parsed = parse_date_input("2024-01-01").unwrap();
        assert_eq!(parsed.year(), 2024);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 1);
        assert_eq!(parsed.hour(), 0);

        // Invalid
        assert!(parse_date_input("garbage").is_none());
        assert!(parse_date_input("-7z").is_none());
    }
}
