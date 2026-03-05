use serde_json::Value;

pub fn is_hidden_detail_field(key: &str) -> bool {
    key == "packetPos" || key == "packetRange" || key == "packetLen" || key.ends_with("Cnt")
}

pub fn is_non_actionable_field(key: &str) -> bool {
    key == "@timestamp"
}

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

#[derive(Clone)]
pub struct ColumnDef {
    pub field: String,    // dbField name (used for API calls)
    pub exp: String,      // expression name (shown to user)
    pub label: String,
    pub width: u16,
}

impl ColumnDef {
    pub fn new(field: &str, exp: &str, label: &str, width: u16) -> Self {
        Self { field: field.into(), exp: exp.into(), label: label.into(), width }
    }
}

pub fn default_columns() -> Vec<ColumnDef> {
    vec![
        ColumnDef::new("ipProtocol", "ip.protocol", "IP", 4),
        ColumnDef::new("firstPacket", "starttime", "First Packet", 20),
        ColumnDef::new("lastPacket", "stoptime", "Last Packet", 20),
        ColumnDef::new("source.ip", "ip.src", "Src IP", 16),
        ColumnDef::new("source.port", "port.src", "SrcPort", 7),
        ColumnDef::new("destination.ip", "ip.dst", "Dst IP", 16),
        ColumnDef::new("destination.port", "port.dst", "DstPort", 7),
        ColumnDef::new("protocol", "protocols", "Protocols", 20),
        ColumnDef::new("source.packets", "packets.src", "Src Pkts", 9),
        ColumnDef::new("destination.packets", "packets.dst", "Dst Pkts", 9),
        ColumnDef::new("source.bytes", "bytes.src", "Src Bytes", 10),
        ColumnDef::new("destination.bytes", "bytes.dst", "Dst Bytes", 10),
    ]
}

#[derive(Clone)]
pub struct ColumnEditorItem {
    pub db_field: String,
    pub exp: String,
    pub friendly_name: String,
    pub enabled: bool,
}

/// Derive a reasonable column width from field type
pub fn width_for_field(field_type: &str) -> u16 {
    match field_type {
        "ip" => 16,
        "integer" => 10,
        "seconds" | "date" => 20,
        _ => 16,
    }
}

#[derive(Clone)]
pub struct SavedLayout {
    pub name: String,
    pub columns: Vec<String>,
    pub sort_field: String,
    pub sort_dir: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColumnEditorMode {
    Browse,
    Reorder,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LayoutPopupMode {
    List,
    SaveInput,
    ConfirmDelete,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewPopupMode {
    List,
    SaveInput,
    ConfirmDelete,
}

#[derive(Clone, Copy, PartialEq)]
pub enum IntegrationPopupMode {
    Integrations,
    Views,
    SaveInput,
    ConfirmDelete,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AppMode {
    Viewer,
    Cont3xt,
    Wise,
    Parliament,
}

impl AppMode {
    pub fn tabs(&self) -> &'static [Tab] {
        match self {
            AppMode::Viewer => &[Tab::Arkime, Tab::Sessions, Tab::Stats, Tab::Settings],
            AppMode::Cont3xt => &[Tab::Search, Tab::C3Stats, Tab::History, Tab::Settings],
            AppMode::Wise => &[Tab::WsStats, Tab::WsQuery, Tab::Settings],
            AppMode::Parliament => &[Tab::Dashboard, Tab::Issues, Tab::Settings],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AppMode::Viewer => "Viewer",
            AppMode::Cont3xt => "Cont3xt",
            AppMode::Wise => "WISE",
            AppMode::Parliament => "Parliament",
        }
    }

    pub fn default_tab(&self) -> Tab {
        match self {
            AppMode::Viewer => Tab::Sessions,
            AppMode::Cont3xt => Tab::Search,
            AppMode::Parliament => Tab::Dashboard,
            AppMode::Wise => Tab::WsStats,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Arkime,
    Sessions,
    Stats,
    Search,
    C3Stats,
    History,
    Dashboard,
    Issues,
    WsStats,
    WsQuery,
    Settings,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Arkime => "Arkime",
            Tab::Sessions => "Sessions",
            Tab::Stats => "Stats",
            Tab::Search => "Search",
            Tab::C3Stats => "Stats",
            Tab::History => "History",
            Tab::Dashboard => "Dashboard",
            Tab::Issues => "Issues",
            Tab::WsStats => "Stats",
            Tab::WsQuery => "Query",
            Tab::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum GraphType {
    Sessions,
    Packets,
    Bytes,
}

impl GraphType {
    pub const ALL: [GraphType; 3] = [GraphType::Sessions, GraphType::Packets, GraphType::Bytes];

    pub fn label(&self) -> &'static str {
        match self {
            GraphType::Sessions => "Sessions",
            GraphType::Packets => "Packets",
            GraphType::Bytes => "Bytes",
        }
    }

    pub fn next(&self) -> GraphType {
        let idx = GraphType::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        GraphType::ALL[(idx + 1) % GraphType::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum GraphSize {
    Off,
    Small,
    Large,
}

impl GraphSize {
    pub fn next(&self) -> GraphSize {
        match self {
            GraphSize::Off => GraphSize::Small,
            GraphSize::Small => GraphSize::Large,
            GraphSize::Large => GraphSize::Off,
        }
    }

    pub fn height(&self) -> u16 {
        match self {
            GraphSize::Off => 0,
            GraphSize::Small => 10,
            GraphSize::Large => 20,
        }
    }

    pub fn is_visible(&self) -> bool {
        *self != GraphSize::Off
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum LineMode {
    Off,
    Hex,
    Decimal,
}

impl LineMode {
    pub fn next(&self) -> LineMode {
        match self {
            LineMode::Off => LineMode::Hex,
            LineMode::Hex => LineMode::Decimal,
            LineMode::Decimal => LineMode::Off,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            LineMode::Off => "off",
            LineMode::Hex => "hex",
            LineMode::Decimal => "dec",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TimeRange {
    Minutes15,
    Minutes30,
    Hours1,
    Hours6,
    Hours24,
    Week1,
    Weeks2,
    Month1,
    All,
}

impl TimeRange {
    pub const ALL: [TimeRange; 9] = [
        TimeRange::Minutes15, TimeRange::Minutes30, TimeRange::Hours1,
        TimeRange::Hours6, TimeRange::Hours24, TimeRange::Week1,
        TimeRange::Weeks2, TimeRange::Month1, TimeRange::All,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TimeRange::Minutes15 => "15m",
            TimeRange::Minutes30 => "30m",
            TimeRange::Hours1 => "1h",
            TimeRange::Hours6 => "6h",
            TimeRange::Hours24 => "24h",
            TimeRange::Week1 => "1w",
            TimeRange::Weeks2 => "2w",
            TimeRange::Month1 => "1M",
            TimeRange::All => "All",
        }
    }

    pub fn date_value(&self) -> &'static str {
        match self {
            TimeRange::Minutes15 => "0.25",
            TimeRange::Minutes30 => "0.50",
            TimeRange::Hours1 => "1",
            TimeRange::Hours6 => "6",
            TimeRange::Hours24 => "24",
            TimeRange::Week1 => "168",
            TimeRange::Weeks2 => "336",
            TimeRange::Month1 => "720",
            TimeRange::All => "-1",
        }
    }

    pub fn next(&self) -> TimeRange {
        let idx = TimeRange::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        TimeRange::ALL[(idx + 1) % TimeRange::ALL.len()]
    }

    pub fn prev(&self) -> TimeRange {
        let idx = TimeRange::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        TimeRange::ALL[(idx + TimeRange::ALL.len() - 1) % TimeRange::ALL.len()]
    }
}

#[derive(PartialEq)]
pub enum SessionView {
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SummaryMetric {
    Sessions,
    Packets,
    Bytes,
}

impl SummaryMetric {
    pub const ALL: [SummaryMetric; 3] = [SummaryMetric::Sessions, SummaryMetric::Packets, SummaryMetric::Bytes];

    pub fn label(&self) -> &'static str {
        match self {
            SummaryMetric::Sessions => "Sessions",
            SummaryMetric::Packets => "Packets",
            SummaryMetric::Bytes => "Bytes",
        }
    }

    pub fn next(&self) -> SummaryMetric {
        let idx = SummaryMetric::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        SummaryMetric::ALL[(idx + 1) % SummaryMetric::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SummarySort {
    Value,
    Sessions,
    Packets,
    Bytes,
}

impl SummarySort {
    pub const ALL: [SummarySort; 4] = [SummarySort::Value, SummarySort::Sessions, SummarySort::Packets, SummarySort::Bytes];

    pub fn next(&self) -> SummarySort {
        let idx = SummarySort::ALL.iter().position(|&t| t == *self).unwrap_or(0);
        SummarySort::ALL[(idx + 1) % SummarySort::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum StatsTab {
    Capture,
    DBStats,
    DBIndices,
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
        match self {
            _ => &[
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
            ],
        }
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

impl StatsTab {
    pub const ALL: [StatsTab; 3] = [StatsTab::Capture, StatsTab::DBStats, StatsTab::DBIndices];

    pub fn name(&self) -> &'static str {
        match self {
            StatsTab::Capture => "Capture Stats",
            StatsTab::DBStats => "DB Stats",
            StatsTab::DBIndices => "DB Indices",
        }
    }

    pub fn columns(&self) -> &[(&str, &str, u16)] {
        // (field_name, label, width)
        match self {
            StatsTab::Capture => &[
                ("nodeName", "Node", 20),
                ("currentTime", "Time", 20),
                ("monitoring", "Sessions", 10),
                ("freeSpaceM", "Free Space", 16),
                ("deltaPackets", "ΔPackets", 10),
                ("deltaBytesPerSec", "Bytes/Sec", 12),
                ("deltaSessions", "ΔSessions", 10),
                ("deltaDropped", "ΔDropped", 10),
            ],
            StatsTab::DBStats => &[
                ("name", "Node", 20),
                ("storeSize", "Disk Used", 14),
                ("docs", "Docs", 14),
                ("searches", "Searches", 12),
                ("searchesTime", "Search Time", 12),
                ("version", "Version", 12),
            ],
            StatsTab::DBIndices => &[
                ("index", "Index", 40),
                ("status", "Status", 10),
                ("health", "Health", 10),
                ("docs.count", "Docs", 14),
                ("store.size", "Disk Size", 14),
                ("pri", "Shards", 8),
            ],
        }
    }
}

#[derive(PartialEq)]
pub enum StatsView {
    List,
    Detail,
}

pub struct StatsDetail {
    pub data: Value,
    pub scroll: u16,
    pub filter: String,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Expression,
    ActionPrompt,
    DetailFilter,
    FieldSelector,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActionTarget {
    Single,
    All,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActionScope {
    Visible,
    Matching,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActionKind {
    DownloadPcap,
    ExportCsv,
    AddTags,
    RemoveTags,
}

pub struct ActionMenu {
    pub target: ActionTarget,
    pub selected: usize,
    pub session_id: Option<String>,
    pub session_node: Option<String>,
    pub scope: Option<ActionScope>,
    pub pending_kind: Option<ActionKind>,
}

impl ActionMenu {
    pub fn options(&self, remove_enabled: bool) -> Vec<ActionKind> {
        let mut opts = match self.target {
            ActionTarget::Single => vec![ActionKind::DownloadPcap, ActionKind::AddTags],
            ActionTarget::All => vec![ActionKind::DownloadPcap, ActionKind::ExportCsv, ActionKind::AddTags],
        };
        if remove_enabled {
            opts.push(ActionKind::RemoveTags);
        }
        opts
    }
}

impl ActionKind {
    pub fn label(&self) -> &'static str {
        match self {
            ActionKind::DownloadPcap => "Download PCAP",
            ActionKind::ExportCsv => "Export CSV",
            ActionKind::AddTags => "Add Tags",
            ActionKind::RemoveTags => "Remove Tags",
        }
    }

    pub fn prompt_label(&self) -> &'static str {
        match self {
            ActionKind::DownloadPcap => "Filename: ",
            ActionKind::ExportCsv => "Filename: ",
            ActionKind::AddTags => "Tags (comma separated): ",
            ActionKind::RemoveTags => "Tags (comma separated): ",
        }
    }
}

pub struct ActionPrompt {
    pub kind: ActionKind,
    pub target: ActionTarget,
    pub scope: ActionScope,
    pub session_id: Option<String>,
    pub session_node: Option<String>,
    pub input: String,
}

pub struct DetailActionMenu {
    pub field: String,       // exp name for expression building
    pub display: String,     // friendlyName for display
    pub value: String,
    pub selected: usize,
    pub values: Option<Vec<String>>,  // populated for array fields
    pub value_selected: usize,
}

impl DetailActionMenu {
    pub const OPTIONS: [&'static str; 4] = [
        "AND value",
        "AND NOT value",
        "OR value",
        "OR NOT value",
    ];
}

pub struct SessionDetail {
    pub data: Value,
    pub scroll: u16,
    pub selected: usize,
    pub total_rows: usize,
    pub filter: String,
}

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub action: String, // identifier matched by the confirm handler
}

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
pub enum WsStatsTab {
    Sources,
    Types,
}

impl WsStatsTab {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            WsStatsTab::Sources => "Sources",
            WsStatsTab::Types => "Types",
        }
    }
}

