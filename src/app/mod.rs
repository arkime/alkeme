mod types;
mod keys;
mod keys_shared;
mod keys_viewer;
mod keys_cont3xt;
mod keys_parliament;
mod keys_wise;
mod viewer;
mod cont3xt;

pub use types::*;

use crate::api::{ArkimeClient, ArkimeField, ArkimeView, Cont3xtIntegration, Cont3xtLinkGroup, Cont3xtOverview, Cont3xtResult, Cont3xtView, GraphData, HttpLog, IntegrationSettings, PlCluster, PlClusterStats, PlGroup, PlIssue, SummaryItem, WsQueryResult, WsSourceStats, WsStats, WsTypeStats};
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
    pub time_ranges: Vec<TimeRange>,
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
    // Per-tab dynamic stats columns
    pub vr_stats_columns: [Vec<StatsColumnDef>; 3],
    // Stats column editor
    pub vr_stats_show_column_editor: bool,
    pub vr_stats_column_editor_selected: usize,
    pub vr_stats_column_editor_mode: ColumnEditorMode,
    pub vr_stats_column_editor_items: Vec<StatsColumnEditorItem>,
    pub vr_stats_column_editor_filter: String,
    // Stats layout popup (shareables)
    pub vr_stats_show_layout_popup: bool,
    pub vr_stats_layout_popup_mode: LayoutPopupMode,
    pub vr_stats_layout_popup_selected: usize,
    pub vr_stats_saved_shareables: Vec<SavedShareable>,
    pub vr_stats_layout_save_name: String,
    pub vr_stats_layout_save_cursor: usize,
    pub vr_stats_layout_delete_name: String,
    pub vr_stats_layout_filter: String,
    pub visible_rows: usize,
    // Files tab state
    pub vr_files_data: Vec<Value>,
    pub vr_files_total: u64,
    pub vr_files_filtered: u64,
    pub vr_files_filter: String,
    pub vr_files_filter_edit: String,
    pub vr_files_selected: usize,
    pub vr_files_table_state: TableState,
    pub vr_files_sort_column: usize,
    pub vr_files_sort_desc: bool,
    pub vr_files_page_start: usize,
    pub vr_files_page_size: usize,
    pub vr_files_columns: Vec<StatsColumnDef>,
    // Files column editor
    pub vr_files_show_column_editor: bool,
    pub vr_files_column_editor_selected: usize,
    pub vr_files_column_editor_mode: ColumnEditorMode,
    pub vr_files_column_editor_items: Vec<StatsColumnEditorItem>,
    pub vr_files_column_editor_filter: String,
    // Files layout popup (shareables)
    pub vr_files_show_layout_popup: bool,
    pub vr_files_layout_popup_mode: LayoutPopupMode,
    pub vr_files_layout_popup_selected: usize,
    pub vr_files_saved_shareables: Vec<SavedShareable>,
    pub vr_files_layout_save_name: String,
    pub vr_files_layout_save_cursor: usize,
    pub vr_files_layout_delete_name: String,
    pub vr_files_layout_filter: String,
    pub vr_files_view: StatsView,
    pub vr_files_detail: Option<StatsDetail>,
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
    pub c3_loaded_file: Option<String>,      // file path when results loaded from --cont3xt-read-json or similar
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
    // Cont3xt settings
    pub c3_settings_tab: C3SettingsTab,
    pub c3_settings_views: Vec<Cont3xtView>,
    pub c3_settings_views_selected: usize,
    pub c3_settings_views_table_state: ratatui::widgets::TableState,
    pub c3_settings_views_filter: String,
    pub c3_settings_views_filtering: bool,
    pub c3_settings_views_loaded: bool,
    pub c3_settings_views_sort: u8,
    pub c3_settings_views_sort_desc: bool,
    pub c3_all_roles: Vec<String>,
    // View editor state
    pub c3_view_editor_open: bool,
    pub c3_view_editor_id: Option<String>, // None = new, Some(id) = editing
    pub c3_view_editor_name: String,
    pub c3_view_editor_name_cursor: usize,
    pub c3_view_editor_integrations: Vec<(String, bool)>, // (name, enabled)
    pub c3_view_editor_integration_selected: usize,
    pub c3_view_editor_integration_filter: String,
    pub c3_view_editor_integration_filtering: bool,
    pub c3_view_editor_view_roles: Vec<(String, bool)>, // (role, selected)
    pub c3_view_editor_edit_roles: Vec<(String, bool)>, // (role, selected)
    pub c3_view_editor_field: C3ViewEditorField,
    // Role popup state (sub-popup within view editor)
    pub c3_role_popup_open: bool,
    pub c3_role_popup_for_edit: bool, // false = viewRoles, true = editRoles
    pub c3_role_popup_selected: usize,
    pub c3_role_popup_filter: String,
    pub c3_role_popup_filtering: bool,
    // Settings confirm dialog
    pub c3_settings_confirm: Option<(String, String)>, // (action, message)
    // Integration settings
    pub c3_int_settings: Vec<IntegrationSettings>,
    pub c3_int_settings_selected: usize,
    pub c3_int_settings_table_state: ratatui::widgets::TableState,
    pub c3_int_settings_filter: String,
    pub c3_int_settings_filtering: bool,
    pub c3_int_settings_loaded: bool,
    pub c3_int_settings_sort: u8,   // 0=Name, 1=Status
    pub c3_int_settings_sort_desc: bool,
    pub c3_int_settings_dirty: bool,
    // Integration config editor
    pub c3_int_editor_open: bool,
    pub c3_int_editor_idx: usize,
    pub c3_int_editor_values: Vec<(String, String, bool, bool, bool, String)>, // (field_name, value, is_password, is_boolean, required, help)
    pub c3_int_editor_selected: usize,
    pub c3_int_editor_cursor: usize,
    pub c3_int_editor_show_password: bool,
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
            time_ranges: TimeRange::defaults(),
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
            vr_stats_columns: [
                stats_columns_from_fields(&capture_default_fields(), &capture_all_columns()),
                stats_columns_from_fields(&esnodes_default_fields(), &esnodes_all_columns()),
                stats_columns_from_fields(&esindices_default_fields(), &esindices_all_columns()),
            ],
            vr_stats_show_column_editor: false,
            vr_stats_column_editor_selected: 0,
            vr_stats_column_editor_mode: ColumnEditorMode::Browse,
            vr_stats_column_editor_items: Vec::new(),
            vr_stats_column_editor_filter: String::new(),
            vr_stats_show_layout_popup: false,
            vr_stats_layout_popup_mode: LayoutPopupMode::List,
            vr_stats_layout_popup_selected: 0,
            vr_stats_saved_shareables: Vec::new(),
            vr_stats_layout_save_name: String::new(),
            vr_stats_layout_save_cursor: 0,
            vr_stats_layout_delete_name: String::new(),
            vr_stats_layout_filter: String::new(),
            visible_rows: 20,
            // Files tab
            vr_files_data: Vec::new(),
            vr_files_total: 0,
            vr_files_filtered: 0,
            vr_files_filter: String::new(),
            vr_files_filter_edit: String::new(),
            vr_files_selected: 0,
            vr_files_table_state: TableState::default(),
            vr_files_sort_column: 0,
            vr_files_sort_desc: false,
            vr_files_page_start: 0,
            vr_files_page_size: 100,
            vr_files_columns: stats_columns_from_fields(&files_default_fields(), &files_all_columns()),
            vr_files_show_column_editor: false,
            vr_files_column_editor_selected: 0,
            vr_files_column_editor_mode: ColumnEditorMode::Browse,
            vr_files_column_editor_items: Vec::new(),
            vr_files_column_editor_filter: String::new(),
            vr_files_show_layout_popup: false,
            vr_files_layout_popup_mode: LayoutPopupMode::List,
            vr_files_layout_popup_selected: 0,
            vr_files_saved_shareables: Vec::new(),
            vr_files_layout_save_name: String::new(),
            vr_files_layout_save_cursor: 0,
            vr_files_layout_delete_name: String::new(),
            vr_files_layout_filter: String::new(),
            vr_files_view: StatsView::List,
            vr_files_detail: None,
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
            c3_loaded_file: None,
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
            // Cont3xt settings
            c3_settings_tab: C3SettingsTab::Views,
            c3_settings_views: Vec::new(),
            c3_settings_views_selected: 0,
            c3_settings_views_table_state: ratatui::widgets::TableState::default(),
            c3_settings_views_filter: String::new(),
            c3_settings_views_filtering: false,
            c3_settings_views_loaded: false,
            c3_settings_views_sort: 0,
            c3_settings_views_sort_desc: false,
            c3_all_roles: Vec::new(),
            c3_view_editor_open: false,
            c3_view_editor_id: None,
            c3_view_editor_name: String::new(),
            c3_view_editor_name_cursor: 0,
            c3_view_editor_integrations: Vec::new(),
            c3_view_editor_integration_selected: 0,
            c3_view_editor_integration_filter: String::new(),
            c3_view_editor_integration_filtering: false,
            c3_view_editor_view_roles: Vec::new(),
            c3_view_editor_edit_roles: Vec::new(),
            c3_view_editor_field: C3ViewEditorField::Name,
            c3_role_popup_open: false,
            c3_role_popup_for_edit: false,
            c3_role_popup_selected: 0,
            c3_role_popup_filter: String::new(),
            c3_role_popup_filtering: false,
            c3_settings_confirm: None,
            // Integration settings
            c3_int_settings: Vec::new(),
            c3_int_settings_selected: 0,
            c3_int_settings_table_state: ratatui::widgets::TableState::default(),
            c3_int_settings_filter: String::new(),
            c3_int_settings_filtering: false,
            c3_int_settings_loaded: false,
            c3_int_settings_sort: 0,
            c3_int_settings_sort_desc: false,
            c3_int_settings_dirty: false,
            c3_int_editor_open: false,
            c3_int_editor_idx: 0,
            c3_int_editor_values: Vec::new(),
            c3_int_editor_selected: 0,
            c3_int_editor_cursor: 0,
            c3_int_editor_show_password: false,
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

    pub fn needs_animation(&self) -> bool {
        match self.app_mode {
            AppMode::Viewer => self.active_tab == Tab::Settings,
            AppMode::Cont3xt => self.active_tab == Tab::Settings && self.c3_settings_tab != C3SettingsTab::Views && self.c3_settings_tab != C3SettingsTab::Integrations,
            AppMode::Wise => self.active_tab == Tab::Settings,
            AppMode::Parliament => self.active_tab == Tab::Settings,
        }
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
            || self.c3_view_editor_open
            || self.c3_role_popup_open
            || self.c3_int_editor_open
            || self.show_help
            || self.show_debug
    }

    /// Returns true if q should close a popup instead of quitting the app
    pub fn q_closes_popup(&self) -> bool {
        self.confirm_dialog.is_some()
            || self.show_help || self.show_debug || self.pl_show_detail
            || self.vr_show_column_editor || self.vr_show_layout_popup || self.vr_show_view_popup
            || self.vr_stats_show_column_editor || self.vr_stats_show_layout_popup
            || self.c3_show_integration_popup || self.c3_show_overview_popup
            || self.c3_show_link_popup || self.c3_show_card_popup
            || self.c3_show_tags_popup || self.c3_show_date_popup
            || self.c3_view_editor_open || self.c3_role_popup_open
            || self.c3_int_editor_open
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
            match self.client.vr_esindex_action(index, "optimize").await {
                Ok(_) => {
                    self.status_msg = format!("Force merge started for '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error force merging index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("close_esindex:") {
            match self.client.vr_esindex_action(index, "close").await {
                Ok(_) => {
                    self.status_msg = format!("Closed index '{index}'");
                    self.vr_fetch_stats().await;
                }
                Err(e) => self.status_msg = format!("Error closing index: {e}"),
            }
        } else if let Some(index) = action.strip_prefix("open_esindex:") {
            match self.client.vr_esindex_action(index, "open").await {
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

    pub fn time_range_next(&mut self) {
        let idx = self.time_ranges.iter().position(|t| t == &self.time_range).unwrap_or(0);
        self.time_range = self.time_ranges[(idx + 1) % self.time_ranges.len()].clone();
    }

    pub fn time_range_prev(&mut self) {
        let idx = self.time_ranges.iter().position(|t| t == &self.time_range).unwrap_or(0);
        let len = self.time_ranges.len();
        self.time_range = self.time_ranges[(idx + len - 1) % len].clone();
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
