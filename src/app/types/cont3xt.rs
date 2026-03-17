use chrono::{Duration, Utc};
use std::collections::{HashMap, HashSet};

/// An entry in the cont3xt results tree — either an indicator header or an integration result
#[derive(Clone)]
pub enum C3TreeItem {
    Indicator(String, String), // (itype, query)
    Result(usize),             // index into c3_results
}

impl C3TreeItem {
    pub fn result_idx(&self) -> Option<usize> {
        match self {
            C3TreeItem::Result(idx) => Some(*idx),
            C3TreeItem::Indicator(_, _) => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Cont3xtFocus {
    Results,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
pub enum IntegrationPopupMode {
    Integrations,
    Views,
    SaveInput,
    ConfirmDelete,
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3SettingsTab {
    Views,
    Integrations,
    LinkGroups,
    Overviews,
}

impl C3SettingsTab {
    pub const ALL: [C3SettingsTab; 4] = [
        C3SettingsTab::Views,
        C3SettingsTab::Integrations,
        C3SettingsTab::LinkGroups,
        C3SettingsTab::Overviews,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            C3SettingsTab::Views => "(1) Views",
            C3SettingsTab::Integrations => "(2) Integrations",
            C3SettingsTab::LinkGroups => "(3) Link Groups",
            C3SettingsTab::Overviews => "(4) Overviews",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3ViewEditorField {
    Name,
    Integrations,
    ViewRoles,
    EditRoles,
}

impl C3ViewEditorField {
    pub fn next(&self) -> Self {
        match self {
            C3ViewEditorField::Name => C3ViewEditorField::Integrations,
            C3ViewEditorField::Integrations => C3ViewEditorField::ViewRoles,
            C3ViewEditorField::ViewRoles => C3ViewEditorField::EditRoles,
            C3ViewEditorField::EditRoles => C3ViewEditorField::Name,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            C3ViewEditorField::Name => C3ViewEditorField::EditRoles,
            C3ViewEditorField::Integrations => C3ViewEditorField::Name,
            C3ViewEditorField::ViewRoles => C3ViewEditorField::Integrations,
            C3ViewEditorField::EditRoles => C3ViewEditorField::ViewRoles,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3LinkGroupLevel {
    GroupList,
    GroupEditor,
    LinkList,
    LinkEditor,
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3BackupKind {
    LinkGroupsAll,
    LinkGroupSingle,
    Integrations,
    Views,
    OverviewsAll,
    OverviewSingle,
}

impl C3BackupKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::LinkGroupsAll => " Backup All Link Groups ",
            Self::LinkGroupSingle => " Backup Link Group ",
            Self::Integrations => " Backup Integration Settings ",
            Self::Views => " Backup Integration Views ",
            Self::OverviewsAll => " Backup All Overviews ",
            Self::OverviewSingle => " Backup Overview ",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3GroupEditorField {
    Name,
    ViewRoles,
    EditRoles,
}

impl C3GroupEditorField {
    pub fn next(&self) -> Self {
        match self {
            Self::Name => Self::ViewRoles,
            Self::ViewRoles => Self::EditRoles,
            Self::EditRoles => Self::Name,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::Name => Self::EditRoles,
            Self::ViewRoles => Self::Name,
            Self::EditRoles => Self::ViewRoles,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3LinkEditorField {
    Name,
    Url,
    Itypes,
    Color,
    InfoField,
    ExternalDocName,
    ExternalDocUrl,
}

impl C3LinkEditorField {
    pub fn all() -> &'static [C3LinkEditorField] {
        &[Self::Name, Self::Url, Self::Itypes, Self::Color, Self::InfoField, Self::ExternalDocName, Self::ExternalDocUrl]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Url => "URL",
            Self::Itypes => "Indicator Types",
            Self::Color => "Color",
            Self::InfoField => "Info",
            Self::ExternalDocName => "External Doc Name",
            Self::ExternalDocUrl => "External Doc URL",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3OverviewLevel {
    List,
    Editor,
    FieldList,
    FieldEditor,
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3OverviewEditorField {
    Name,
    Title,
    Itype,
    ViewRoles,
    EditRoles,
}

impl C3OverviewEditorField {
    pub fn all() -> &'static [C3OverviewEditorField] {
        &[Self::Name, Self::Title, Self::Itype, Self::ViewRoles, Self::EditRoles]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Title => "Title",
            Self::Itype => "IType",
            Self::ViewRoles => "View Roles",
            Self::EditRoles => "Edit Roles",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Name => Self::Title,
            Self::Title => Self::Itype,
            Self::Itype => Self::ViewRoles,
            Self::ViewRoles => Self::EditRoles,
            Self::EditRoles => Self::Name,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::Name => Self::EditRoles,
            Self::Title => Self::Name,
            Self::Itype => Self::Title,
            Self::ViewRoles => Self::Itype,
            Self::EditRoles => Self::ViewRoles,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3OvFieldEditorField {
    From,
    Field,
    Label,
    CustomJson,
}

impl C3OvFieldEditorField {
    pub fn next(&self, is_custom: bool) -> Self {
        if is_custom {
            match self {
                Self::From => Self::CustomJson,
                _ => Self::From,
            }
        } else {
            match self {
                Self::From => Self::Field,
                Self::Field => Self::Label,
                _ => Self::From,
            }
        }
    }
    pub fn prev(&self, is_custom: bool) -> Self {
        if is_custom {
            match self {
                Self::From => Self::CustomJson,
                _ => Self::From,
            }
        } else {
            match self {
                Self::From => Self::Label,
                Self::Field => Self::From,
                _ => Self::Field,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum C3StatsTab {
    Integrations,
    ITypes,
}

impl C3StatsTab {
    pub const ALL: [C3StatsTab; 2] = [C3StatsTab::Integrations, C3StatsTab::ITypes];

    pub fn name(&self) -> &'static str {
        match self {
            C3StatsTab::Integrations => "(1) Integrations",
            C3StatsTab::ITypes => "(2) iTypes",
        }
    }

    pub fn columns(&self) -> &[(&str, &str, u16)] {
        // Both sub-tabs share the same columns
        &[
            ("name", "Name", 20),
            ("cacheLookup", "Cache Lookup", 13),
            ("cacheFound", "Cache Found", 12),
            ("cacheGood", "Cache Good", 11),
            ("cacheRecentAvgMS", "Cache Avg MS", 13),
            ("directLookup", "Direct Lookup", 14),
            ("directFound", "Direct Found", 13),
            ("directGood", "Direct Good", 12),
            ("directError", "Direct Error", 13),
            ("directRecentAvgMS", "Direct Avg MS", 14),
            ("total", "Total", 10),
        ]
    }
}

// (api_field, label, width, sortable)
pub const C3_HISTORY_COLUMNS: &[(&str, &str, u16, bool)] = &[
    ("issuedAt", "Time", 18, true),
    ("iType", "iType", 8, true),
    ("indicator", "Indicator", 30, true),
    ("tags", "Tags", 15, false),
    ("resultCount", "Results", 8, true),
    ("took", "Took", 8, true),
];

pub struct Cont3xtState {
    /// Cont3xt state
    pub integrations: Vec<crate::api::Cont3xtIntegration>,
    pub overviews: Vec<crate::api::Cont3xtOverview>,
    pub results: Vec<crate::api::Cont3xtResult>,
    /// index into c3_tree_order
    pub selected: usize,
    /// tree items in display order
    pub tree_order: Vec<C3TreeItem>,
    /// indices into c3_tree_order where each root indicator starts
    pub tree_roots: Vec<usize>,
    /// scroll in detail pane
    pub detail_scroll: u16,
    /// horizontal scroll in detail pane
    pub detail_hscroll: u16,
    /// filter string for detail pane
    pub detail_filter: String,
    pub detail_filter_cursor: usize,
    pub search_total: u64,
    pub search_sent: u64,
    pub search_itype: String,
    /// indicator parent map: (child_indicator, child_itype) -> [(parent_query, parent_itype), ...]
    pub indicator_parents: HashMap<(String, String), Vec<(String, String)>>,
    /// init-ordered indicators: (itype, query) in the order from the init response
    pub init_indicators: Vec<(String, String)>,
    /// which pane has focus
    pub focus: Cont3xtFocus,
    /// show raw JSON instead of card
    pub raw_view: bool,
    /// show card definition popup
    pub show_card_popup: bool,
    /// scroll offset for card popup
    pub card_popup_scroll: u16,
    /// overview selector popup
    pub show_overview_popup: bool,
    pub overview_popup_selected: usize,
    pub overview_popup_filter: String,
    pub overview_popup_filter_cursor: usize,
    pub overview_popup_filtering: bool,
    /// itype -> overview id
    pub selected_overviews: HashMap<String, String>,
    /// user-toggled off
    pub disabled_integrations: HashSet<String>,
    pub show_integration_popup: bool,
    pub integration_popup_selected: usize,
    pub integration_popup_filter: String,
    pub integration_popup_filter_cursor: usize,
    pub integration_popup_filtering: bool,
    /// which sub-view of integration popup
    pub integration_popup_mode: IntegrationPopupMode,
    pub views: Vec<crate::api::Cont3xtView>,
    pub view_selected: usize,
    pub view_save_name: String,
    pub view_save_cursor: usize,
    pub active_view_id: Option<String>,
    pub active_view_name: Option<String>,
    /// streaming search in progress
    pub searching: bool,
    pub pending_search: bool,
    pub no_cache: bool,
    /// tags sent with search query
    pub tags: Vec<String>,
    /// edit buffer for tags popup
    pub tags_edit: String,
    pub tags_edit_cursor: usize,
    /// tag editor popup visible
    pub show_tags_popup: bool,
    /// filename prompt for JSON export
    pub save_json_prompt: Option<String>,
    pub save_json_cursor: usize,
    /// headless: save JSON to file and quit when search completes
    pub save_json_path: Option<String>,
    /// file path when results loaded from --cont3xt-read-json or similar
    pub loaded_file: Option<String>,
    /// Cont3xt date range
    pub start_date: chrono::DateTime<Utc>,
    pub stop_date: chrono::DateTime<Utc>,
    pub show_date_popup: bool,
    /// edit buffer for start date
    pub date_start_edit: String,
    pub date_start_edit_cursor: usize,
    /// edit buffer for stop date
    pub date_stop_edit: String,
    pub date_stop_edit_cursor: usize,
    /// 0 = start, 1 = stop
    pub date_field: u8,
    /// Cont3xt link groups
    pub link_groups: Vec<crate::api::Cont3xtLinkGroup>,
    pub show_link_popup: bool,
    pub link_popup_selected: usize,
    pub link_popup_filter: String,
    pub link_popup_filter_cursor: usize,
    pub link_popup_filtering: bool,
    /// (group_name, link_name, url, info, color) filtered by itype
    pub link_flat: Vec<(String, String, String, String, String)>,
    /// Cont3xt stats
    pub stats_tab: C3StatsTab,
    /// integration stats
    pub stats_data: Vec<serde_json::Value>,
    /// itype stats
    pub itype_stats_data: Vec<serde_json::Value>,
    pub stats_selected: usize,
    pub stats_table_state: ratatui::widgets::TableState,
    pub stats_filter: String,
    pub stats_filter_cursor: usize,
    pub stats_filtering: bool,
    pub stats_sort_col: usize,
    pub stats_sort_desc: bool,
    /// Cont3xt history
    pub history_data: Vec<serde_json::Value>,
    pub history_total: usize,
    /// 1-indexed
    pub history_page: usize,
    pub history_selected: usize,
    pub history_table_state: ratatui::widgets::TableState,
    pub history_filter: String,
    pub history_filter_cursor: usize,
    pub history_filtering: bool,
    pub history_sort_col: usize,
    pub history_sort_desc: bool,
    pub history_loaded: bool,
    /// Cont3xt settings
    pub settings_tab: C3SettingsTab,
    pub settings_views: Vec<crate::api::Cont3xtView>,
    pub settings_views_selected: usize,
    pub settings_views_table_state: ratatui::widgets::TableState,
    pub settings_views_filter: String,
    pub settings_views_filter_cursor: usize,
    pub settings_views_filtering: bool,
    pub settings_views_loaded: bool,
    pub settings_views_sort: u8,
    pub settings_views_sort_desc: bool,
    pub all_roles: Vec<String>,
    /// View editor state
    pub view_editor_open: bool,
    /// None = new, Some(id) = editing
    pub view_editor_id: Option<String>,
    pub view_editor_name: String,
    pub view_editor_name_cursor: usize,
    /// (name, enabled)
    pub view_editor_integrations: Vec<(String, bool)>,
    pub view_editor_integration_selected: usize,
    pub view_editor_integration_filter: String,
    pub view_editor_integration_filter_cursor: usize,
    pub view_editor_integration_filtering: bool,
    /// (role, selected)
    pub view_editor_view_roles: Vec<(String, bool)>,
    /// (role, selected)
    pub view_editor_edit_roles: Vec<(String, bool)>,
    pub view_editor_field: C3ViewEditorField,
    /// Role popup state (sub-popup within view editor)
    pub role_popup_open: bool,
    /// false = viewRoles, true = editRoles
    pub role_popup_for_edit: bool,
    pub role_popup_selected: usize,
    pub role_popup_filter: String,
    pub role_popup_cursor: usize,
    pub role_popup_filtering: bool,
    /// (action, message)
    pub settings_confirm: Option<(String, String)>,
    /// Integration settings
    pub int_settings: Vec<crate::api::IntegrationSettings>,
    pub int_settings_selected: usize,
    pub int_settings_table_state: ratatui::widgets::TableState,
    pub int_settings_filter: String,
    pub int_settings_filter_cursor: usize,
    pub int_settings_filtering: bool,
    pub int_settings_loaded: bool,
    /// 0=Name, 1=Status
    pub int_settings_sort: u8,
    pub int_settings_sort_desc: bool,
    pub int_settings_dirty: bool,
    /// Integration config editor
    pub int_editor_open: bool,
    pub int_editor_idx: usize,
    /// (field_name, value, is_password, is_boolean, required, help)
    pub int_editor_values: Vec<(String, String, bool, bool, bool, String)>,
    pub int_editor_selected: usize,
    pub int_editor_cursor: usize,
    pub int_editor_show_password: bool,
    /// Link group settings
    pub lg_level: C3LinkGroupLevel,
    pub lg_groups: Vec<crate::api::Cont3xtLinkGroup>,
    pub lg_selected: usize,
    pub lg_filter: String,
    pub lg_filter_cursor: usize,
    pub lg_filtering: bool,
    pub lg_sort_col: usize,
    pub lg_sort_desc: bool,
    pub lg_table_state: ratatui::widgets::TableState,
    pub lg_loaded: bool,
    /// Link list within group
    pub lg_links_selected: usize,
    pub lg_links_table_state: ratatui::widgets::TableState,
    pub lg_editing_group_idx: usize,
    pub lg_links_filter: String,
    pub lg_links_filter_cursor: usize,
    pub lg_links_filtering: bool,
    /// Link editor
    pub lg_editor_field: C3LinkEditorField,
    pub lg_editor_link: crate::api::Cont3xtLink,
    pub lg_editor_link_idx: usize,
    pub lg_editor_cursor: usize,
    pub lg_editor_itype_selected: usize,
    /// Link group backup prompt
    pub backup_prompt: Option<String>,
    pub backup_cursor: usize,
    pub backup_kind: C3BackupKind,
    /// Group editor (name, viewRoles, editRoles)
    pub lg_group_editor_field: C3GroupEditorField,
    pub lg_group_editor_name: String,
    pub lg_group_editor_cursor: usize,
    pub lg_group_editor_view_roles: Vec<String>,
    pub lg_group_editor_edit_roles: Vec<String>,
    pub lg_group_editor_idx: usize,
    /// Overview settings
    pub ov_level: C3OverviewLevel,
    pub ov_list: Vec<crate::api::Cont3xtOverview>,
    pub ov_selected: usize,
    pub ov_filter: String,
    pub ov_filter_cursor: usize,
    pub ov_filtering: bool,
    pub ov_sort_col: usize,
    pub ov_sort_desc: bool,
    pub ov_table_state: ratatui::widgets::TableState,
    pub ov_loaded: bool,
    /// Overview editor
    pub ov_editor_field: C3OverviewEditorField,
    pub ov_editor_cursor: usize,
    pub ov_editor_idx: usize,
    pub ov_editor_name: String,
    pub ov_editor_title: String,
    pub ov_editor_itype: String,
    pub ov_editor_view_roles: Vec<String>,
    pub ov_editor_edit_roles: Vec<String>,
    /// Overview field list
    pub ov_fields_selected: usize,
    pub ov_fields_table_state: ratatui::widgets::TableState,
    pub ov_fields_filter: String,
    pub ov_fields_filter_cursor: usize,
    pub ov_fields_filtering: bool,
    /// Overview field editor
    pub ov_field_editor_field: C3OvFieldEditorField,
    pub ov_field_editor_cursor: usize,
    pub ov_field_editor_idx: usize,
    pub ov_field_editor_from: String,
    pub ov_field_editor_field_name: String,
    pub ov_field_editor_label: String,
    pub ov_field_editor_is_custom: bool,
    pub ov_fe_json_lines: Vec<String>,
    pub ov_fe_json_line: usize,
    pub ov_fe_json_col: usize,
    pub ov_fe_json_scroll: usize,
    pub ov_fe_popup_open: bool,
    /// false=From, true=Field
    pub ov_fe_popup_for_field: bool,
    pub ov_fe_popup_items: Vec<String>,
    pub ov_fe_popup_selected: usize,
    pub ov_fe_popup_filter: String,
    pub ov_fe_popup_cursor: usize,
    pub ov_fe_popup_filtering: bool,
}

impl Default for Cont3xtState {
    fn default() -> Self {
        Self {
            integrations: Vec::new(),
            overviews: Vec::new(),
            results: Vec::new(),
            selected: 0,
            tree_order: Vec::new(),
            tree_roots: Vec::new(),
            detail_scroll: 0,
            detail_hscroll: 0,
            detail_filter: String::new(),
            detail_filter_cursor: 0,
            search_total: 0,
            search_sent: 0,
            search_itype: String::new(),
            indicator_parents: HashMap::new(),
            init_indicators: Vec::new(),
            focus: Cont3xtFocus::Results,
            raw_view: false,
            show_card_popup: false,
            card_popup_scroll: 0,
            show_overview_popup: false,
            overview_popup_selected: 0,
            overview_popup_filter: String::new(),
            overview_popup_filter_cursor: 0,
            overview_popup_filtering: false,
            selected_overviews: HashMap::new(),
            disabled_integrations: HashSet::new(),
            show_integration_popup: false,
            integration_popup_selected: 0,
            integration_popup_filter: String::new(),
            integration_popup_filter_cursor: 0,
            integration_popup_filtering: false,
            integration_popup_mode: IntegrationPopupMode::Integrations,
            views: Vec::new(),
            view_selected: 0,
            view_save_name: String::new(),
            view_save_cursor: 0,
            active_view_id: None,
            active_view_name: None,
            searching: false,
            pending_search: false,
            no_cache: false,
            tags: Vec::new(),
            tags_edit: String::new(),
            tags_edit_cursor: 0,
            show_tags_popup: false,
            save_json_prompt: None,
            save_json_cursor: 0,
            save_json_path: None,
            loaded_file: None,
            start_date: Utc::now() - Duration::days(7),
            stop_date: Utc::now(),
            show_date_popup: false,
            date_start_edit: String::from("-7d"),
            date_start_edit_cursor: 3,
            date_stop_edit: String::from("now"),
            date_stop_edit_cursor: 3,
            date_field: 0,
            link_groups: Vec::new(),
            show_link_popup: false,
            link_popup_selected: 0,
            link_popup_filter: String::new(),
            link_popup_filter_cursor: 0,
            link_popup_filtering: false,
            link_flat: Vec::new(),
            stats_tab: C3StatsTab::Integrations,
            stats_data: Vec::new(),
            itype_stats_data: Vec::new(),
            stats_selected: 0,
            stats_table_state: ratatui::widgets::TableState::default(),
            stats_filter: String::new(),
            stats_filter_cursor: 0,
            stats_filtering: false,
            stats_sort_col: 0,
            stats_sort_desc: false,
            history_data: Vec::new(),
            history_total: 0,
            history_page: 1,
            history_selected: 0,
            history_table_state: ratatui::widgets::TableState::default(),
            history_filter: String::new(),
            history_filter_cursor: 0,
            history_filtering: false,
            history_sort_col: 0,
            history_sort_desc: true,
            history_loaded: false,
            // Cont3xt settings
            settings_tab: C3SettingsTab::Views,
            settings_views: Vec::new(),
            settings_views_selected: 0,
            settings_views_table_state: ratatui::widgets::TableState::default(),
            settings_views_filter: String::new(),
            settings_views_filter_cursor: 0,
            settings_views_filtering: false,
            settings_views_loaded: false,
            settings_views_sort: 0,
            settings_views_sort_desc: false,
            all_roles: Vec::new(),
            view_editor_open: false,
            view_editor_id: None,
            view_editor_name: String::new(),
            view_editor_name_cursor: 0,
            view_editor_integrations: Vec::new(),
            view_editor_integration_selected: 0,
            view_editor_integration_filter: String::new(),
            view_editor_integration_filter_cursor: 0,
            view_editor_integration_filtering: false,
            view_editor_view_roles: Vec::new(),
            view_editor_edit_roles: Vec::new(),
            view_editor_field: C3ViewEditorField::Name,
            role_popup_open: false,
            role_popup_for_edit: false,
            role_popup_selected: 0,
            role_popup_filter: String::new(),
            role_popup_cursor: 0,
            role_popup_filtering: false,
            settings_confirm: None,
            // Integration settings
            int_settings: Vec::new(),
            int_settings_selected: 0,
            int_settings_table_state: ratatui::widgets::TableState::default(),
            int_settings_filter: String::new(),
            int_settings_filter_cursor: 0,
            int_settings_filtering: false,
            int_settings_loaded: false,
            int_settings_sort: 0,
            int_settings_sort_desc: false,
            int_settings_dirty: false,
            int_editor_open: false,
            int_editor_idx: 0,
            int_editor_values: Vec::new(),
            int_editor_selected: 0,
            int_editor_cursor: 0,
            int_editor_show_password: false,
            // Link group settings
            lg_level: C3LinkGroupLevel::GroupList,
            lg_groups: Vec::new(),
            lg_selected: 0,
            lg_filter: String::new(),
            lg_filter_cursor: 0,
            lg_filtering: false,
            lg_sort_col: 0,
            lg_sort_desc: false,
            lg_table_state: ratatui::widgets::TableState::default(),
            lg_loaded: false,
            lg_links_selected: 0,
            lg_links_table_state: ratatui::widgets::TableState::default(),
            lg_editing_group_idx: 0,
            lg_links_filter: String::new(),
            lg_links_filter_cursor: 0,
            lg_links_filtering: false,
            lg_editor_field: C3LinkEditorField::Name,
            lg_editor_link: crate::api::Cont3xtLink {
            name: String::new(),
            url: String::new(),
            itypes: Vec::new(),
            info: String::new(),
            color: String::new(),
            external_doc_name: String::new(),
            external_doc_url: String::new(),
            },
            lg_editor_link_idx: 0,
            lg_editor_cursor: 0,
            lg_editor_itype_selected: 0,
            backup_prompt: None,
            backup_cursor: 0,
            backup_kind: C3BackupKind::LinkGroupsAll,
            lg_group_editor_field: C3GroupEditorField::Name,
            lg_group_editor_name: String::new(),
            lg_group_editor_cursor: 0,
            lg_group_editor_view_roles: Vec::new(),
            lg_group_editor_edit_roles: Vec::new(),
            lg_group_editor_idx: 0,
            ov_level: C3OverviewLevel::List,
            ov_list: Vec::new(),
            ov_selected: 0,
            ov_filter: String::new(),
            ov_filter_cursor: 0,
            ov_filtering: false,
            ov_sort_col: 0,
            ov_sort_desc: false,
            ov_table_state: ratatui::widgets::TableState::default(),
            ov_loaded: false,
            ov_editor_field: C3OverviewEditorField::Name,
            ov_editor_cursor: 0,
            ov_editor_idx: 0,
            ov_editor_name: String::new(),
            ov_editor_title: String::new(),
            ov_editor_itype: String::new(),
            ov_editor_view_roles: Vec::new(),
            ov_editor_edit_roles: Vec::new(),
            ov_fields_selected: 0,
            ov_fields_table_state: ratatui::widgets::TableState::default(),
            ov_fields_filter: String::new(),
            ov_fields_filter_cursor: 0,
            ov_fields_filtering: false,
            ov_field_editor_field: C3OvFieldEditorField::From,
            ov_field_editor_cursor: 0,
            ov_field_editor_idx: 0,
            ov_field_editor_from: String::new(),
            ov_field_editor_field_name: String::new(),
            ov_field_editor_label: String::new(),
            ov_field_editor_is_custom: false,
            ov_fe_json_lines: Vec::new(),
            ov_fe_json_line: 0,
            ov_fe_json_col: 0,
            ov_fe_json_scroll: 0,
            ov_fe_popup_open: false,
            ov_fe_popup_for_field: false,
            ov_fe_popup_items: Vec::new(),
            ov_fe_popup_selected: 0,
            ov_fe_popup_filter: String::new(),
            ov_fe_popup_cursor: 0,
            ov_fe_popup_filtering: false,
        }
    }
}
