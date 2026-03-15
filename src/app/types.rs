use chrono::{Duration, Utc};
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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

// --- Stats column configuration ---

#[derive(Clone, Copy, PartialEq)]
pub enum StatsFormat {
    String,
    Number,
    Bytes,
    BytesPerSec,
    MegaBytes,
    Percent,
    EpochSecs,
    EpochMs,
    SizeString,
    Boolean,
    PercentSuffix,
}

#[derive(Clone)]
pub struct StatsColumnDef {
    pub field: String,
    pub sort: String,
    pub label: String,
    pub width: u16,
    pub format: StatsFormat,
}

impl StatsColumnDef {
    pub fn new(field: &str, sort: &str, label: &str, width: u16, format: StatsFormat) -> Self {
        Self {
            field: field.into(),
            sort: sort.into(),
            label: label.into(),
            width,
            format,
        }
    }

    pub fn is_numeric(&self) -> bool {
        !matches!(self.format, StatsFormat::String)
    }
}

#[derive(Clone)]
pub struct StatsColumnEditorItem {
    pub field: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct SavedShareable {
    pub id: String,
    pub name: String,
    pub columns: Vec<String>,
    pub sort_field: String,
    pub sort_dir: String,
    pub shared: bool,
}

pub fn capture_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("nodeName", "nodeName", "Node", 20, String),
        StatsColumnDef::new("currentTime", "currentTime", "Time", 20, EpochSecs),
        StatsColumnDef::new("monitoring", "monitoring", "Sessions", 10, Number),
        StatsColumnDef::new("freeSpaceM", "freeSpaceM", "Free Space", 16, MegaBytes),
        StatsColumnDef::new("cpu", "cpu", "CPU", 8, Percent),
        StatsColumnDef::new("memory", "memory", "Memory", 16, Bytes),
        StatsColumnDef::new("packetQueue", "packetQueue", "Pkt Queue", 10, Number),
        StatsColumnDef::new("diskQueue", "diskQueue", "Disk Queue", 10, Number),
        StatsColumnDef::new("esQueue", "esQueue", "ES Queue", 10, Number),
        StatsColumnDef::new("deltaPackets", "deltaPacketsPerSec", "ΔPackets", 10, Number),
        StatsColumnDef::new("deltaBytesPerSec", "deltaBytesPerSec", "Bytes/Sec", 12, BytesPerSec),
        StatsColumnDef::new("deltaSessions", "deltaSessionsPerSec", "ΔSessions", 10, Number),
        StatsColumnDef::new("deltaDropped", "deltaDroppedPerSec", "ΔDropped", 10, Number),
        // non-default
        StatsColumnDef::new("deltaBitsPerSec", "deltaBitsPerSec", "Bits/Sec", 12, Number),
        StatsColumnDef::new("deltaWrittenBytesPerSec", "deltaWrittenBytesPerSec", "Written/Sec", 12, BytesPerSec),
        StatsColumnDef::new("deltaUnwrittenBytesPerSec", "deltaUnwrittenBytesPerSec", "Unwritten/Sec", 14, BytesPerSec),
        StatsColumnDef::new("tcpSessions", "tcpSessions", "TCP Sess", 10, Number),
        StatsColumnDef::new("udpSessions", "udpSessions", "UDP Sess", 10, Number),
        StatsColumnDef::new("icmpSessions", "icmpSessions", "ICMP Sess", 10, Number),
        StatsColumnDef::new("sctpSessions", "sctpSessions", "SCTP Sess", 10, Number),
        StatsColumnDef::new("espSessions", "espSessions", "ESP Sess", 10, Number),
        StatsColumnDef::new("usedSpaceM", "usedSpaceM", "Used Space", 12, MegaBytes),
        StatsColumnDef::new("esHealthMS", "esHealthMS", "ES Health MS", 12, Number),
        StatsColumnDef::new("closeQueue", "closeQueue", "Close Queue", 12, Number),
        StatsColumnDef::new("needSave", "needSave", "Need Save", 10, Number),
        StatsColumnDef::new("frags", "frags", "Frags", 10, Number),
        StatsColumnDef::new("deltaFragsDroppedPerSec", "deltaFragsDroppedPerSec", "ΔFragsDropped", 14, Number),
        StatsColumnDef::new("deltaTotalDroppedPerSec", "deltaTotalDroppedPerSec", "ΔTotalDropped", 14, Number),
        StatsColumnDef::new("deltaSessionBytesPerSec", "deltaSessionBytesPerSec", "SessBytes/Sec", 14, BytesPerSec),
        StatsColumnDef::new("deltaOverloadDropped", "deltaOverloadDropped", "ΔOverloadDrop", 14, Number),
        StatsColumnDef::new("deltaDupDroppedPerSec", "deltaDupDroppedPerSec", "ΔDupDropped", 12, Number),
        StatsColumnDef::new("deltaESDroppedPerSec", "deltaESDroppedPerSec", "ΔESDropped", 12, Number),
        StatsColumnDef::new("sessionSizePerSec", "sessionSizePerSec", "SessSize/Sec", 12, Number),
        StatsColumnDef::new("retention", "retention", "Retention", 10, Number),
        StatsColumnDef::new("startTime", "startTime", "Start Time", 20, EpochSecs),
        StatsColumnDef::new("runningTime", "runningTime", "Running Time", 20, Number),
        StatsColumnDef::new("ver", "ver", "Version", 14, String),
    ]
}

pub fn capture_default_fields() -> Vec<&'static str> {
    vec![
        "nodeName", "currentTime", "monitoring", "freeSpaceM", "cpu", "memory",
        "packetQueue", "diskQueue", "esQueue", "deltaPackets", "deltaBytesPerSec",
        "deltaSessions", "deltaDropped",
    ]
}

pub fn esnodes_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("name", "nodeName", "Node", 20, String),
        StatsColumnDef::new("docs", "docs", "Docs", 12, Number),
        StatsColumnDef::new("storeSize", "storeSize", "Disk Used", 12, Bytes),
        StatsColumnDef::new("freeSize", "freeSize", "Free Size", 12, Bytes),
        StatsColumnDef::new("heapSize", "heapSize", "Heap Size", 12, Bytes),
        StatsColumnDef::new("load", "load", "Load", 10, Number),
        StatsColumnDef::new("cpu", "cpu", "CPU", 8, Percent),
        StatsColumnDef::new("read", "read", "Read", 10, Bytes),
        StatsColumnDef::new("write", "write", "Write", 10, Bytes),
        StatsColumnDef::new("searches", "searches", "Searches", 10, Number),
        // non-default
        StatsColumnDef::new("scrolls", "scrolls", "Scrolls", 10, Number),
        StatsColumnDef::new("ip", "ip", "IP", 16, String),
        StatsColumnDef::new("ipExcluded", "ipExcluded", "IP Excluded", 12, String),
        StatsColumnDef::new("nodeExcluded", "nodeExcluded", "Node Excluded", 14, String),
        StatsColumnDef::new("nonHeapSize", "nonHeapSize", "Non-Heap", 12, Bytes),
        StatsColumnDef::new("searchesTime", "searchesTime", "Search Time", 12, Number),
        StatsColumnDef::new("writesRejectedDelta", "writesRejectedDelta", "Writes Rej Δ", 12, Number),
        StatsColumnDef::new("writesCompleted", "writesCompleted", "Writes Done", 12, Number),
        StatsColumnDef::new("writesCompletedDelta", "writesCompletedDelta", "Writes Done Δ", 14, Number),
        StatsColumnDef::new("writesQueueSize", "writesQueueSize", "Write Queue", 12, Number),
        StatsColumnDef::new("molochtype", "molochtype", "Type", 10, String),
        StatsColumnDef::new("molochzone", "molochzone", "Zone", 10, String),
        StatsColumnDef::new("shards", "shards", "Shards", 8, Number),
        StatsColumnDef::new("segments", "segments", "Segments", 10, Number),
        StatsColumnDef::new("uptime", "uptime", "Uptime", 10, Number),
        StatsColumnDef::new("version", "version", "Version", 12, String),
        StatsColumnDef::new("writesRejected", "writesRejected", "Writes Rejected", 14, Number),
    ]
}

pub fn esnodes_default_fields() -> Vec<&'static str> {
    vec![
        "name", "docs", "storeSize", "freeSize", "heapSize", "load", "cpu",
        "read", "write", "searches",
    ]
}

pub fn esindices_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("index", "index", "Index", 40, String),
        StatsColumnDef::new("docs.count", "docs.count", "Docs", 14, Number),
        StatsColumnDef::new("store.size", "store.size", "Disk Size", 14, SizeString),
        StatsColumnDef::new("pri", "pri", "Shards", 8, Number),
        StatsColumnDef::new("segmentsCount", "segmentsCount", "Segments", 10, Number),
        StatsColumnDef::new("rep", "rep", "Replicas", 10, Number),
        StatsColumnDef::new("memoryTotal", "memoryTotal", "Memory", 12, Bytes),
        StatsColumnDef::new("health", "health", "Health", 10, String),
        StatsColumnDef::new("status", "status", "Status", 10, String),
        // non-default
        StatsColumnDef::new("cd", "cd", "Created", 20, EpochSecs),
        StatsColumnDef::new("pri.search.query_current", "pri.search.query_current", "Queries", 10, Number),
        StatsColumnDef::new("uuid", "uuid", "UUID", 12, String),
        StatsColumnDef::new("molochtype", "molochtype", "Type", 10, String),
        StatsColumnDef::new("shardsPerNode", "shardsPerNode", "Shards/Node", 12, String),
        StatsColumnDef::new("versionCreated", "versionCreated", "Version", 12, String),
        StatsColumnDef::new("docSize", "docSize", "Doc Size", 10, Number),
    ]
}

pub fn esindices_default_fields() -> Vec<&'static str> {
    vec![
        "index", "docs.count", "store.size", "pri", "segmentsCount", "rep",
        "memoryTotal", "health", "status",
    ]
}

pub fn files_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("num", "num", "File #", 8, Number),
        StatsColumnDef::new("node", "node", "Node", 14, String),
        StatsColumnDef::new("name", "name", "Name", 50, String),
        StatsColumnDef::new("locked", "locked", "Locked", 8, Boolean),
        StatsColumnDef::new("first", "first", "First Date", 20, EpochSecs),
        StatsColumnDef::new("filesize", "filesize", "File Size", 12, Number),
        // non-default
        StatsColumnDef::new("lastTimestamp", "lastTimestamp", "Last Date", 20, EpochMs),
        StatsColumnDef::new("encoding", "encoding", "Encoding", 14, String),
        StatsColumnDef::new("packetPosEncoding", "packetPosEncoding", "Pkt Pos Enc", 14, String),
        StatsColumnDef::new("packets", "packets", "Packets", 12, Number),
        StatsColumnDef::new("packetsSize", "packetsSize", "Packets Size", 14, Number),
        StatsColumnDef::new("uncompressedBits", "uncompressedBits", "UC Bits", 10, Number),
        StatsColumnDef::new("cratio", "cratio", "C Ratio", 10, PercentSuffix),
        StatsColumnDef::new("compression", "compression", "Compression", 14, String),
        StatsColumnDef::new("startTimestamp", "startTimestamp", "Start Date", 20, EpochMs),
        StatsColumnDef::new("finishTimestamp", "finishTimestamp", "Finish Date", 20, EpochMs),
        StatsColumnDef::new("sessionsStarted", "sessionsStarted", "Sess Started", 14, Number),
        StatsColumnDef::new("sessionsPresent", "sessionsPresent", "Sess Present", 14, Number),
    ]
}

pub fn files_default_fields() -> Vec<&'static str> {
    vec!["num", "node", "name", "locked", "first", "filesize"]
}

/// Build active columns from a list of field names and the all-columns definition
pub fn stats_columns_from_fields(field_names: &[&str], all_columns: &[StatsColumnDef]) -> Vec<StatsColumnDef> {
    let mut result = Vec::new();
    for &name in field_names {
        if let Some(col) = all_columns.iter().find(|c| c.field == name) {
            result.push(col.clone());
        }
    }
    result
}

/// Get all-columns and default fields for a given StatsTab
pub fn stats_tab_all_columns(tab: StatsTab) -> Vec<StatsColumnDef> {
    match tab {
        StatsTab::Capture => capture_all_columns(),
        StatsTab::DBStats => esnodes_all_columns(),
        StatsTab::DBIndices => esindices_all_columns(),
    }
}

pub fn stats_tab_default_fields(tab: StatsTab) -> Vec<&'static str> {
    match tab {
        StatsTab::Capture => capture_default_fields(),
        StatsTab::DBStats => esnodes_default_fields(),
        StatsTab::DBIndices => esindices_default_fields(),
    }
}

pub fn stats_tab_shareable_type(tab: StatsTab) -> &'static str {
    match tab {
        StatsTab::Capture => "capture-columns",
        StatsTab::DBStats => "esnodes-columns",
        StatsTab::DBIndices => "esindices-columns",
    }
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
            AppMode::Viewer => &[Tab::Arkime, Tab::Sessions, Tab::Stats, Tab::Files, Tab::Settings],
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
    Files,
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
            Tab::Files => "Files",
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

#[derive(Clone, PartialEq)]
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
    Custom { label: String, date_value: String },
}

impl TimeRange {
    pub fn defaults() -> Vec<TimeRange> {
        vec![
            TimeRange::Minutes15, TimeRange::Minutes30, TimeRange::Hours1,
            TimeRange::Hours6, TimeRange::Hours24, TimeRange::Week1,
            TimeRange::Weeks2, TimeRange::Month1, TimeRange::All,
        ]
    }

    pub fn label(&self) -> &str {
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
            TimeRange::Custom { label, .. } => label,
        }
    }

    pub fn date_value(&self) -> &str {
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
            TimeRange::Custom { date_value, .. } => date_value,
        }
    }

    /// Hours as f64 for sorting (All = infinity)
    fn sort_hours(&self) -> f64 {
        match self {
            TimeRange::All => f64::INFINITY,
            other => other.date_value().parse().unwrap_or(0.0),
        }
    }

    /// Parse a CLI time range string. Returns the matching built-in or a Custom entry.
    /// Supports: built-in labels (15m, 30m, 1h, etc.), -1 (All), {float}h (custom hours).
    pub fn parse(s: &str) -> Result<TimeRange, String> {
        if s == "-1" {
            return Ok(TimeRange::All);
        }
        // Check built-in labels (case-insensitive)
        for t in Self::defaults() {
            if t.label().eq_ignore_ascii_case(s) {
                return Ok(t);
            }
        }
        // Try {float}h, {float}w, {float}m format
        let last = s.as_bytes().last().copied().unwrap_or(0);
        let (num_str, suffix, multiplier) = match last {
            b'h' | b'H' => (&s[..s.len()-1], "h", 1.0),
            b'w' | b'W' => (&s[..s.len()-1], "w", 168.0),
            b'm' | b'M' => (&s[..s.len()-1], "m", 730.5),
            _ => ("", "", 0.0),
        };
        if !num_str.is_empty() {
            if let Ok(val) = num_str.parse::<f64>() {
                if val > 0.0 {
                    let hours = val * multiplier;
                    return Ok(TimeRange::Custom {
                        label: format!("{val}{suffix}"),
                        date_value: hours.to_string(),
                    });
                }
            }
        }
        Err(format!("invalid time range '{s}': use a label (15m, 1h, 24h, 1w, All), -1, or {{num}}h/w/m (e.g. 72h, 2w, 3m)"))
    }

    /// Insert a time range into a sorted list. If it matches an existing entry, does nothing.
    /// Returns the index of the (possibly newly inserted) entry.
    pub fn insert_sorted(list: &mut Vec<TimeRange>, entry: TimeRange) -> usize {
        // Check if already present
        if let Some(idx) = list.iter().position(|t| t == &entry) {
            return idx;
        }
        let hours = entry.sort_hours();
        let pos = list.iter().position(|t| t.sort_hours() > hours).unwrap_or(list.len());
        list.insert(pos, entry);
        pos
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
            StatsTab::DBStats => "DB Nodes",
            StatsTab::DBIndices => "DB Indices",
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


pub struct WiseState {
    pub stats: Option<crate::api::WsStats>,
    pub stats_tab: WsStatsTab,
    pub stats_filter: String,
    pub stats_filter_edit: String,
    pub stats_selected: usize,
    pub last_refresh: std::time::Instant,
    pub sources: Vec<String>,
    pub types: Vec<String>,
    pub query_source: String,
    pub query_type: String,
    pub query_value: String,
    pub query_value_edit: String,
    pub query_results: Vec<crate::api::WsQueryResult>,
    pub query_selected: usize,
}

impl Default for WiseState {
    fn default() -> Self {
        Self {
            stats: None,
            stats_tab: WsStatsTab::Sources,
            stats_filter: String::new(),
            stats_filter_edit: String::new(),
            stats_selected: 0,
            last_refresh: std::time::Instant::now(),
            sources: Vec::new(),
            types: Vec::new(),
            query_source: "any".into(),
            query_type: "ip".into(),
            query_value: String::new(),
            query_value_edit: String::new(),
            query_results: Vec::new(),
            query_selected: 0,
        }
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
        }
    }
}
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
    pub overview_popup_filtering: bool,
    /// itype -> overview id
    pub selected_overviews: HashMap<String, String>,
    /// user-toggled off
    pub disabled_integrations: HashSet<String>,
    pub show_integration_popup: bool,
    pub integration_popup_selected: usize,
    pub integration_popup_filter: String,
    pub integration_popup_filtering: bool,
    /// which sub-view of integration popup
    pub integration_popup_mode: IntegrationPopupMode,
    pub views: Vec<crate::api::Cont3xtView>,
    pub view_selected: usize,
    pub view_save_name: String,
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
    /// tag editor popup visible
    pub show_tags_popup: bool,
    /// filename prompt for JSON export
    pub save_json_prompt: Option<String>,
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
    /// edit buffer for stop date
    pub date_stop_edit: String,
    /// 0 = start, 1 = stop
    pub date_field: u8,
    /// Cont3xt link groups
    pub link_groups: Vec<crate::api::Cont3xtLinkGroup>,
    pub show_link_popup: bool,
    pub link_popup_selected: usize,
    pub link_popup_filter: String,
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
    pub role_popup_filtering: bool,
    /// (action, message)
    pub settings_confirm: Option<(String, String)>,
    /// Integration settings
    pub int_settings: Vec<crate::api::IntegrationSettings>,
    pub int_settings_selected: usize,
    pub int_settings_table_state: ratatui::widgets::TableState,
    pub int_settings_filter: String,
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
    pub lg_links_filtering: bool,
    /// Link editor
    pub lg_editor_field: C3LinkEditorField,
    pub lg_editor_link: crate::api::Cont3xtLink,
    pub lg_editor_link_idx: usize,
    pub lg_editor_cursor: usize,
    pub lg_editor_itype_selected: usize,
    /// Link group backup prompt
    pub backup_prompt: Option<String>,
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
            overview_popup_filtering: false,
            selected_overviews: HashMap::new(),
            disabled_integrations: HashSet::new(),
            show_integration_popup: false,
            integration_popup_selected: 0,
            integration_popup_filter: String::new(),
            integration_popup_filtering: false,
            integration_popup_mode: IntegrationPopupMode::Integrations,
            views: Vec::new(),
            view_selected: 0,
            view_save_name: String::new(),
            active_view_id: None,
            active_view_name: None,
            searching: false,
            pending_search: false,
            no_cache: false,
            tags: Vec::new(),
            tags_edit: String::new(),
            show_tags_popup: false,
            save_json_prompt: None,
            save_json_path: None,
            loaded_file: None,
            start_date: Utc::now() - Duration::days(7),
            stop_date: Utc::now(),
            show_date_popup: false,
            date_start_edit: String::from("-7d"),
            date_stop_edit: String::from("now"),
            date_field: 0,
            link_groups: Vec::new(),
            show_link_popup: false,
            link_popup_selected: 0,
            link_popup_filter: String::new(),
            link_popup_filtering: false,
            link_flat: Vec::new(),
            stats_tab: C3StatsTab::Integrations,
            stats_data: Vec::new(),
            itype_stats_data: Vec::new(),
            stats_selected: 0,
            stats_table_state: ratatui::widgets::TableState::default(),
            stats_filter: String::new(),
            stats_filtering: false,
            stats_sort_col: 0,
            stats_sort_desc: false,
            history_data: Vec::new(),
            history_total: 0,
            history_page: 1,
            history_selected: 0,
            history_table_state: ratatui::widgets::TableState::default(),
            history_filter: String::new(),
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
            view_editor_integration_filtering: false,
            view_editor_view_roles: Vec::new(),
            view_editor_edit_roles: Vec::new(),
            view_editor_field: C3ViewEditorField::Name,
            role_popup_open: false,
            role_popup_for_edit: false,
            role_popup_selected: 0,
            role_popup_filter: String::new(),
            role_popup_filtering: false,
            settings_confirm: None,
            // Integration settings
            int_settings: Vec::new(),
            int_settings_selected: 0,
            int_settings_table_state: ratatui::widgets::TableState::default(),
            int_settings_filter: String::new(),
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
            lg_filtering: false,
            lg_sort_col: 0,
            lg_sort_desc: false,
            lg_table_state: ratatui::widgets::TableState::default(),
            lg_loaded: false,
            lg_links_selected: 0,
            lg_links_table_state: ratatui::widgets::TableState::default(),
            lg_editing_group_idx: 0,
            lg_links_filter: String::new(),
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
            ov_fe_popup_filtering: false,
        }
    }
}
pub struct ViewerState {
    /// dbField -> type ("seconds" or "date")
    pub date_fields: HashMap<String, String>,
    /// dbField -> exp (expression field name)
    pub field_exp_map: HashMap<String, String>,
    /// dbField -> friendlyName
    pub field_friendly_map: HashMap<String, String>,
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
    /// view id for API calls
    pub active_view: Option<String>,
    /// view name for display
    pub active_view_name: Option<String>,
    pub saved_views: Vec<crate::api::ArkimeView>,
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
    pub pending_packets_fetch: bool,
    pub pending_summary_fetch: bool,
    pub packets_node_pending: String,
    pub packets_id_pending: String,
    pub packets_total_pending: u64,
    pub sort_column: usize,
    pub sort_desc: bool,
    pub graph_size: GraphSize,
    pub graph_type: GraphType,
    pub graph_data: Option<crate::api::GraphData>,
    /// Stats tab state
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
    /// Per-tab dynamic stats columns
    pub stats_columns: [Vec<StatsColumnDef>; 3],
    /// Stats column editor
    pub stats_show_column_editor: bool,
    pub stats_column_editor_selected: usize,
    pub stats_column_editor_mode: ColumnEditorMode,
    pub stats_column_editor_items: Vec<StatsColumnEditorItem>,
    pub stats_column_editor_filter: String,
    /// Stats layout popup (shareables)
    pub stats_show_layout_popup: bool,
    pub stats_layout_popup_mode: LayoutPopupMode,
    pub stats_layout_popup_selected: usize,
    pub stats_saved_shareables: Vec<SavedShareable>,
    pub stats_layout_save_name: String,
    pub stats_layout_save_cursor: usize,
    pub stats_layout_delete_name: String,
    pub stats_layout_filter: String,
    /// Files tab state
    pub files_data: Vec<Value>,
    pub files_total: u64,
    pub files_filtered: u64,
    pub files_filter: String,
    pub files_filter_edit: String,
    pub files_selected: usize,
    pub files_table_state: TableState,
    pub files_sort_column: usize,
    pub files_sort_desc: bool,
    pub files_page_start: usize,
    pub files_page_size: usize,
    pub files_columns: Vec<StatsColumnDef>,
    /// Files column editor
    pub files_show_column_editor: bool,
    pub files_column_editor_selected: usize,
    pub files_column_editor_mode: ColumnEditorMode,
    pub files_column_editor_items: Vec<StatsColumnEditorItem>,
    pub files_column_editor_filter: String,
    /// Files layout popup (shareables)
    pub files_show_layout_popup: bool,
    pub files_layout_popup_mode: LayoutPopupMode,
    pub files_layout_popup_selected: usize,
    pub files_saved_shareables: Vec<SavedShareable>,
    pub files_layout_save_name: String,
    pub files_layout_save_cursor: usize,
    pub files_layout_delete_name: String,
    pub files_layout_filter: String,
    pub files_view: StatsView,
    pub files_detail: Option<StatsDetail>,
    /// Arkime (Summary) tab state
    pub all_fields: Vec<crate::api::ArkimeField>,
    pub summary_field: String,
    pub summary_data: Vec<crate::api::SummaryItem>,
    pub summary_metric: SummaryMetric,
    pub summary_selected: usize,
    pub summary_table_state: TableState,
    pub summary_sort: SummarySort,
    pub summary_sort_desc: bool,
    pub field_filter: String,
    pub field_filter_selected: usize,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
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
            stats_columns: [
            stats_columns_from_fields(&capture_default_fields(), &capture_all_columns()),
            stats_columns_from_fields(&esnodes_default_fields(), &esnodes_all_columns()),
            stats_columns_from_fields(&esindices_default_fields(), &esindices_all_columns()),
            ],
            stats_show_column_editor: false,
            stats_column_editor_selected: 0,
            stats_column_editor_mode: ColumnEditorMode::Browse,
            stats_column_editor_items: Vec::new(),
            stats_column_editor_filter: String::new(),
            stats_show_layout_popup: false,
            stats_layout_popup_mode: LayoutPopupMode::List,
            stats_layout_popup_selected: 0,
            stats_saved_shareables: Vec::new(),
            stats_layout_save_name: String::new(),
            stats_layout_save_cursor: 0,
            stats_layout_delete_name: String::new(),
            stats_layout_filter: String::new(),
            // Files tab
            files_data: Vec::new(),
            files_total: 0,
            files_filtered: 0,
            files_filter: String::new(),
            files_filter_edit: String::new(),
            files_selected: 0,
            files_table_state: TableState::default(),
            files_sort_column: 0,
            files_sort_desc: false,
            files_page_start: 0,
            files_page_size: 100,
            files_columns: stats_columns_from_fields(&files_default_fields(), &files_all_columns()),
            files_show_column_editor: false,
            files_column_editor_selected: 0,
            files_column_editor_mode: ColumnEditorMode::Browse,
            files_column_editor_items: Vec::new(),
            files_column_editor_filter: String::new(),
            files_show_layout_popup: false,
            files_layout_popup_mode: LayoutPopupMode::List,
            files_layout_popup_selected: 0,
            files_saved_shareables: Vec::new(),
            files_layout_save_name: String::new(),
            files_layout_save_cursor: 0,
            files_layout_delete_name: String::new(),
            files_layout_filter: String::new(),
            files_view: StatsView::List,
            files_detail: None,
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
        }
    }
}
