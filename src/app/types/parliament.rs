use ratatui::widgets::TableState;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub enum PlIssueSort {
    Cluster,
    Title,
    Severity,
    FirstNoticed,
    LastNoticed,
}

impl PlIssueSort {
    pub const ALL: [PlIssueSort; 5] = [
        PlIssueSort::Cluster, PlIssueSort::Title, PlIssueSort::Severity,
        PlIssueSort::FirstNoticed, PlIssueSort::LastNoticed,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PlIssueSort::Cluster => "Cluster",
            PlIssueSort::Title => "Title",
            PlIssueSort::Severity => "Severity",
            PlIssueSort::FirstNoticed => "First Noticed",
            PlIssueSort::LastNoticed => "Last Noticed",
        }
    }

    pub fn next(&self) -> PlIssueSort {
        let idx = PlIssueSort::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        PlIssueSort::ALL[(idx + 1) % PlIssueSort::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlSettingsTab {
    Groups,
    General,
    Notifiers,
}

impl PlSettingsTab {
    pub const ALL: [PlSettingsTab; 3] = [PlSettingsTab::Groups, PlSettingsTab::General, PlSettingsTab::Notifiers];

    pub fn label(&self) -> &'static str {
        match self {
            PlSettingsTab::Groups => "Groups",
            PlSettingsTab::General => "General",
            PlSettingsTab::Notifiers => "Notifiers",
        }
    }
}

/// What level of the groups settings tree we're at
#[derive(Clone, Copy, PartialEq)]
pub enum PlSettingsLevel {
    GroupList,
    GroupEditor,
    ClusterEditor,
}

/// Which field is selected in the group editor
#[derive(Clone, Copy, PartialEq)]
pub enum PlGroupEditorField {
    Title,
    Description,
}

/// Which field is selected in the cluster editor
#[derive(Clone, Copy, PartialEq)]
pub enum PlClusterEditorField {
    Title,
    Url,
    LocalUrl,
    Description,
    Type,
    HideDeltaBPS,
    HideDeltaTDPS,
    HideMonitoring,
    HideArkimeNodes,
    HideDataNodes,
    HideTotalNodes,
}

impl PlClusterEditorField {
    pub const ALL: [PlClusterEditorField; 11] = [
        PlClusterEditorField::Title,
        PlClusterEditorField::Url,
        PlClusterEditorField::LocalUrl,
        PlClusterEditorField::Description,
        PlClusterEditorField::Type,
        PlClusterEditorField::HideDeltaBPS,
        PlClusterEditorField::HideDeltaTDPS,
        PlClusterEditorField::HideMonitoring,
        PlClusterEditorField::HideArkimeNodes,
        PlClusterEditorField::HideDataNodes,
        PlClusterEditorField::HideTotalNodes,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PlClusterEditorField::Title => "Title",
            PlClusterEditorField::Url => "URL",
            PlClusterEditorField::LocalUrl => "Local URL",
            PlClusterEditorField::Description => "Description",
            PlClusterEditorField::Type => "Type",
            PlClusterEditorField::HideDeltaBPS => "Hide Δ BPS",
            PlClusterEditorField::HideDeltaTDPS => "Hide Δ Drops/s",
            PlClusterEditorField::HideMonitoring => "Hide Monitoring",
            PlClusterEditorField::HideArkimeNodes => "Hide Arkime Nodes",
            PlClusterEditorField::HideDataNodes => "Hide Data Nodes",
            PlClusterEditorField::HideTotalNodes => "Hide Total Nodes",
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, PlClusterEditorField::HideDeltaBPS
            | PlClusterEditorField::HideDeltaTDPS
            | PlClusterEditorField::HideMonitoring
            | PlClusterEditorField::HideArkimeNodes
            | PlClusterEditorField::HideDataNodes
            | PlClusterEditorField::HideTotalNodes)
    }
}

/// Which field is selected in the general settings editor
#[derive(Clone, Copy, PartialEq)]
pub enum PlGeneralField {
    OutOfDate,
    EsQueryTimeout,
    NoPackets,
    NoPacketsLength,
    LowDiskSpace,
    LowDiskSpaceType,
    LowDiskSpaceES,
    LowDiskSpaceESType,
    RemoveIssuesAfter,
    RemoveAcknowledgedAfter,
    WiseUrl,
    Cont3xtUrl,
}

impl PlGeneralField {
    pub const ALL: [PlGeneralField; 12] = [
        PlGeneralField::OutOfDate,
        PlGeneralField::EsQueryTimeout,
        PlGeneralField::NoPackets,
        PlGeneralField::NoPacketsLength,
        PlGeneralField::LowDiskSpace,
        PlGeneralField::LowDiskSpaceType,
        PlGeneralField::LowDiskSpaceES,
        PlGeneralField::LowDiskSpaceESType,
        PlGeneralField::RemoveIssuesAfter,
        PlGeneralField::RemoveAcknowledgedAfter,
        PlGeneralField::WiseUrl,
        PlGeneralField::Cont3xtUrl,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PlGeneralField::OutOfDate => "Out of Date (sec)",
            PlGeneralField::EsQueryTimeout => "ES Query Timeout (sec)",
            PlGeneralField::NoPackets => "No Packets Threshold",
            PlGeneralField::NoPacketsLength => "No Packets Duration (sec)",
            PlGeneralField::LowDiskSpace => "Low Disk Space",
            PlGeneralField::LowDiskSpaceType => "Disk Space Unit",
            PlGeneralField::LowDiskSpaceES => "Low Disk Space ES",
            PlGeneralField::LowDiskSpaceESType => "ES Disk Space Unit",
            PlGeneralField::RemoveIssuesAfter => "Remove Issues After (min)",
            PlGeneralField::RemoveAcknowledgedAfter => "Remove Ack'd After (min)",
            PlGeneralField::WiseUrl => "WISE URL",
            PlGeneralField::Cont3xtUrl => "Cont3xt URL",
        }
    }

    pub fn is_select(&self) -> bool {
        matches!(self, PlGeneralField::LowDiskSpaceType | PlGeneralField::LowDiskSpaceESType)
    }
}

pub struct ParliamentState {
    pub groups: Vec<crate::api::PlGroup>,
    pub stats: HashMap<String, crate::api::PlClusterStats>,
    pub issues_map: HashMap<String, Vec<crate::api::PlIssue>>,
    pub issues: Vec<crate::api::PlIssue>,
    pub issues_filter: String,
    pub issues_filter_edit: String,
    pub issues_sort: PlIssueSort,
    pub issues_sort_desc: bool,
    pub issues_selected: usize,
    pub issues_table_state: TableState,
    pub selected_group: usize,
    pub selected_cluster: usize,
    pub dashboard_scroll: u16,
    pub last_refresh: std::time::Instant,
    pub show_detail: bool,
    pub detail_scroll: u16,
    /// Flat list of (group_idx, cluster_idx) for dashboard navigation
    pub cluster_list: Vec<(usize, usize)>,
    /// Saved parliament client for returning from viewer/cont3xt mode (Ctrl+P)
    pub saved_client: Option<crate::api::ArkimeClient>,
    pub cont3xt_url: String,
    pub wise_url: String,
    pub saved_viewer_expression: String,
    pub saved_c3_expression: String,

    // --- Settings tab state ---
    pub settings_tab: PlSettingsTab,
    pub settings_level: PlSettingsLevel,
    pub settings_general: crate::api::PlGeneralSettings,

    // Groups sub-tab
    /// Flat list for navigation: (group_idx, Option<cluster_idx>)
    pub settings_items: Vec<(usize, Option<usize>)>,
    pub settings_selected: usize,
    pub settings_table_state: TableState,

    // Group editor
    pub group_editor_idx: usize,
    pub group_editor_is_new: bool,
    pub group_editor_title: String,
    pub group_editor_title_cursor: usize,
    pub group_editor_desc: String,
    pub group_editor_desc_cursor: usize,
    pub group_editor_field: PlGroupEditorField,

    // Cluster editor
    pub cluster_editor_group_idx: usize,
    pub cluster_editor_cluster_idx: usize,
    pub cluster_editor_is_new: bool,
    pub cluster_editor_field: PlClusterEditorField,
    pub cluster_editor_title: String,
    pub cluster_editor_title_cursor: usize,
    pub cluster_editor_url: String,
    pub cluster_editor_url_cursor: usize,
    pub cluster_editor_local_url: String,
    pub cluster_editor_local_url_cursor: usize,
    pub cluster_editor_desc: String,
    pub cluster_editor_desc_cursor: usize,
    pub cluster_editor_type: String,
    pub cluster_editor_hide_delta_bps: bool,
    pub cluster_editor_hide_delta_tdps: bool,
    pub cluster_editor_hide_monitoring: bool,
    pub cluster_editor_hide_arkime_nodes: bool,
    pub cluster_editor_hide_data_nodes: bool,
    pub cluster_editor_hide_total_nodes: bool,

    // General sub-tab
    pub general_selected: usize,
    pub general_editing: bool,
    pub general_edit_value: String,
    pub general_edit_cursor: usize,

    // Backup
    pub backup_prompt: Option<String>,
    pub backup_cursor: usize,
}

impl Default for ParliamentState {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            stats: HashMap::new(),
            issues_map: HashMap::new(),
            issues: Vec::new(),
            issues_filter: String::new(),
            issues_filter_edit: String::new(),
            issues_sort: PlIssueSort::LastNoticed,
            issues_sort_desc: true,
            issues_selected: 0,
            issues_table_state: TableState::default().with_selected(0),
            selected_group: 0,
            selected_cluster: 0,
            dashboard_scroll: 0,
            last_refresh: std::time::Instant::now(),
            show_detail: false,
            detail_scroll: 0,
            cluster_list: Vec::new(),
            saved_client: None,
            cont3xt_url: String::new(),
            wise_url: String::new(),
            saved_viewer_expression: String::new(),
            saved_c3_expression: String::new(),

            settings_tab: PlSettingsTab::Groups,
            settings_level: PlSettingsLevel::GroupList,
            settings_general: crate::api::PlGeneralSettings::default(),
            settings_items: Vec::new(),
            settings_selected: 0,
            settings_table_state: TableState::default().with_selected(0),

            group_editor_idx: 0,
            group_editor_is_new: false,
            group_editor_title: String::new(),
            group_editor_title_cursor: 0,
            group_editor_desc: String::new(),
            group_editor_desc_cursor: 0,
            group_editor_field: PlGroupEditorField::Title,

            cluster_editor_group_idx: 0,
            cluster_editor_cluster_idx: 0,
            cluster_editor_is_new: false,
            cluster_editor_field: PlClusterEditorField::Title,
            cluster_editor_title: String::new(),
            cluster_editor_title_cursor: 0,
            cluster_editor_url: String::new(),
            cluster_editor_url_cursor: 0,
            cluster_editor_local_url: String::new(),
            cluster_editor_local_url_cursor: 0,
            cluster_editor_desc: String::new(),
            cluster_editor_desc_cursor: 0,
            cluster_editor_type: String::new(),
            cluster_editor_hide_delta_bps: false,
            cluster_editor_hide_delta_tdps: false,
            cluster_editor_hide_monitoring: false,
            cluster_editor_hide_arkime_nodes: false,
            cluster_editor_hide_data_nodes: false,
            cluster_editor_hide_total_nodes: false,

            general_selected: 0,
            general_editing: false,
            general_edit_value: String::new(),
            general_edit_cursor: 0,

            backup_prompt: None,
            backup_cursor: 0,
        }
    }
}
