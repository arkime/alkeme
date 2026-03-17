pub mod cont3xt;
pub mod parliament;
pub mod wise;
pub mod viewer;

pub use cont3xt::*;
pub use parliament::*;
pub use wise::*;
pub use viewer::*;

use serde_json::Value;

pub fn is_hidden_detail_field(key: &str) -> bool {
    key == "packetPos" || key == "packetRange" || key == "packetLen" || key.ends_with("Cnt")
}

pub fn is_non_actionable_field(key: &str) -> bool {
    key == "@timestamp"
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
    Nanos,
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

pub fn estasks_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("action", "action", "Action", 30, String),
        StatsColumnDef::new("description", "description", "Description", 50, String),
        StatsColumnDef::new("start_time_in_millis", "start_time_in_millis", "Start Time", 20, EpochMs),
        StatsColumnDef::new("running_time_in_nanos", "running_time_in_nanos", "Running Time", 14, Nanos),
        StatsColumnDef::new("childrenCount", "childrenCount", "Children", 10, Number),
        StatsColumnDef::new("user", "user", "User", 16, String),
        // non-default
        StatsColumnDef::new("cancellable", "cancellable", "Cancellable", 12, Boolean),
        StatsColumnDef::new("id", "id", "ID", 24, String),
        StatsColumnDef::new("node", "node", "Node", 20, String),
        StatsColumnDef::new("taskId", "taskId", "Task ID", 30, String),
        StatsColumnDef::new("type", "type", "Type", 14, String),
    ]
}

pub fn estasks_default_fields() -> Vec<&'static str> {
    vec![
        "action", "description", "start_time_in_millis", "running_time_in_nanos",
        "childrenCount", "user",
    ]
}

pub fn esrecovery_all_columns() -> Vec<StatsColumnDef> {
    use StatsFormat::*;
    vec![
        StatsColumnDef::new("index", "index", "Index", 30, String),
        StatsColumnDef::new("shard", "shard", "Shard", 6, Number),
        StatsColumnDef::new("time", "time", "Time", 10, String),
        StatsColumnDef::new("type", "type", "Type", 14, String),
        StatsColumnDef::new("stage", "stage", "Stage", 12, String),
        StatsColumnDef::new("source_host", "source_host", "Source Host", 16, String),
        StatsColumnDef::new("source_node", "source_node", "Source Node", 18, String),
        StatsColumnDef::new("target_host", "target_host", "Target Host", 16, String),
        StatsColumnDef::new("target_node", "target_node", "Target Node", 18, String),
        StatsColumnDef::new("files", "files", "Files", 8, Number),
        StatsColumnDef::new("files_recovered", "files_recovered", "Files Recov", 12, Number),
        StatsColumnDef::new("files_percent", "files_percent", "Files %", 8, PercentSuffix),
        StatsColumnDef::new("files_total", "files_total", "Files Total", 12, Number),
        StatsColumnDef::new("bytes", "bytes", "Bytes", 12, Bytes),
        StatsColumnDef::new("bytes_recovered", "bytes_recovered", "Bytes Recov", 12, Bytes),
        StatsColumnDef::new("bytes_percent", "bytes_percent", "Bytes %", 8, PercentSuffix),
        StatsColumnDef::new("bytes_total", "bytes_total", "Bytes Total", 12, Bytes),
        StatsColumnDef::new("translog_ops", "translog_ops", "Translog Ops", 12, Number),
        StatsColumnDef::new("translog_ops_recovered", "translog_ops_recovered", "TLog Recov", 12, Number),
        StatsColumnDef::new("translog_ops_percent", "translog_ops_percent", "TLog %", 8, PercentSuffix),
    ]
}

pub fn esrecovery_default_fields() -> Vec<&'static str> {
    vec![
        "index", "shard", "time", "type", "stage",
        "source_node", "target_node",
        "files_percent", "bytes_percent", "translog_ops_percent",
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
        StatsTab::DBTasks => estasks_all_columns(),
        StatsTab::DBRecovery => esrecovery_all_columns(),
        StatsTab::CaptureGraphs | StatsTab::DBShards => Vec::new(),
    }
}

pub fn stats_tab_default_fields(tab: StatsTab) -> Vec<&'static str> {
    match tab {
        StatsTab::Capture => capture_default_fields(),
        StatsTab::DBStats => esnodes_default_fields(),
        StatsTab::DBIndices => esindices_default_fields(),
        StatsTab::DBTasks => estasks_default_fields(),
        StatsTab::DBRecovery => esrecovery_default_fields(),
        StatsTab::CaptureGraphs | StatsTab::DBShards => Vec::new(),
    }
}

pub fn stats_tab_shareable_type(tab: StatsTab) -> &'static str {
    match tab {
        StatsTab::Capture => "capture-columns",
        StatsTab::DBStats => "esnodes-columns",
        StatsTab::DBIndices => "esindices-columns",
        StatsTab::DBTasks => "estasks-columns",
        StatsTab::DBRecovery => "esrecovery-columns",
        StatsTab::CaptureGraphs => "capturegraphs-columns",
        StatsTab::DBShards => "esshards-columns",
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewPopupMode {
    List,
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
    Users,
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
            Tab::Users => "Users",
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
        if !num_str.is_empty()
            && let Ok(val) = num_str.parse::<f64>()
                && val > 0.0 {
                    let hours = val * multiplier;
                    return Ok(TimeRange::Custom {
                        label: format!("{val}{suffix}"),
                        date_value: hours.to_string(),
                    });
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
    CaptureGraphs,
    Capture,
    DBStats,
    DBIndices,
    DBTasks,
    DBShards,
    DBRecovery,
}

impl StatsTab {
    pub const ALL: [StatsTab; 7] = [StatsTab::CaptureGraphs, StatsTab::Capture, StatsTab::DBStats, StatsTab::DBIndices, StatsTab::DBTasks, StatsTab::DBShards, StatsTab::DBRecovery];

    pub fn name(&self) -> &'static str {
        match self {
            StatsTab::CaptureGraphs => "Capture Graphs",
            StatsTab::Capture => "Capture Stats",
            StatsTab::DBStats => "DB Nodes",
            StatsTab::DBIndices => "DB Indices",
            StatsTab::DBTasks => "DB Tasks",
            StatsTab::DBShards => "DB Shards",
            StatsTab::DBRecovery => "DB Recovery",
        }
    }

    /// Array index for column-based tabs (stats_columns/stats_state_loaded arrays).
    /// DBShards and CaptureGraphs are not column-based and return None.
    pub fn col_index(&self) -> Option<usize> {
        match self {
            StatsTab::Capture => Some(0),
            StatsTab::DBStats => Some(1),
            StatsTab::DBIndices => Some(2),
            StatsTab::DBTasks => Some(3),
            StatsTab::DBRecovery => Some(4),
            StatsTab::CaptureGraphs | StatsTab::DBShards => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ShardsShow {
    All,
    NotStarted,
    Initializing,
    Relocating,
    Unassigned,
}

impl ShardsShow {
    pub const ALL: [ShardsShow; 5] = [
        ShardsShow::All, ShardsShow::NotStarted, ShardsShow::Initializing,
        ShardsShow::Relocating, ShardsShow::Unassigned,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ShardsShow::All => "All",
            ShardsShow::NotStarted => "Not Started",
            ShardsShow::Initializing => "Initializing",
            ShardsShow::Relocating => "Relocating",
            ShardsShow::Unassigned => "Unassigned",
        }
    }

    pub fn api_value(&self) -> &'static str {
        match self {
            ShardsShow::All => "all",
            ShardsShow::NotStarted => "notstarted",
            ShardsShow::Initializing => "INITIALIZING",
            ShardsShow::Relocating => "RELOCATING",
            ShardsShow::Unassigned => "UNASSIGNED",
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|&s| s == *self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|&s| s == *self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Capture graph metric definition
#[derive(Clone, Copy, PartialEq)]
pub struct CaptureGraphMetric {
    pub field: &'static str,
    pub label: &'static str,
}

pub const CAPTURE_GRAPH_METRICS: &[CaptureGraphMetric] = &[
    CaptureGraphMetric { field: "deltaPacketsPerSec", label: "Packet/s" },
    CaptureGraphMetric { field: "deltaBytesPerSec", label: "Bytes/s" },
    CaptureGraphMetric { field: "deltaBitsPerSec", label: "Bits/Sec" },
    CaptureGraphMetric { field: "deltaSessionsPerSec", label: "Sessions/s" },
    CaptureGraphMetric { field: "deltaDroppedPerSec", label: "Packet Drops/s" },
    CaptureGraphMetric { field: "monitoring", label: "Sessions" },
    CaptureGraphMetric { field: "tcpSessions", label: "Active TCP Sessions" },
    CaptureGraphMetric { field: "udpSessions", label: "Active UDP Sessions" },
    CaptureGraphMetric { field: "icmpSessions", label: "Active ICMP Sessions" },
    CaptureGraphMetric { field: "sctpSessions", label: "Active SCTP Sessions" },
    CaptureGraphMetric { field: "espSessions", label: "Active ESP Sessions" },
    CaptureGraphMetric { field: "usedSpaceM", label: "Used Space" },
    CaptureGraphMetric { field: "freeSpaceM", label: "Free Space" },
    CaptureGraphMetric { field: "freeSpaceP", label: "Free Space %" },
    CaptureGraphMetric { field: "memory", label: "Memory" },
    CaptureGraphMetric { field: "memoryP", label: "Memory %" },
    CaptureGraphMetric { field: "cpu", label: "CPU" },
    CaptureGraphMetric { field: "diskQueue", label: "Disk Q" },
    CaptureGraphMetric { field: "esQueue", label: "ES Q" },
    CaptureGraphMetric { field: "deltaESDroppedPerSec", label: "ES Drops/s" },
    CaptureGraphMetric { field: "esHealthMS", label: "ES Health Response MS" },
    CaptureGraphMetric { field: "packetQueue", label: "Packet Q" },
    CaptureGraphMetric { field: "closeQueue", label: "Closing Q" },
    CaptureGraphMetric { field: "needSave", label: "Waiting Q" },
    CaptureGraphMetric { field: "frags", label: "Active Fragments" },
    CaptureGraphMetric { field: "deltaFragsDroppedPerSec", label: "Fragments Dropped/Sec" },
    CaptureGraphMetric { field: "deltaOverloadDroppedPerSec", label: "Overload Drops/s" },
    CaptureGraphMetric { field: "deltaDupDroppedPerSec", label: "Dup Drops/s" },
    CaptureGraphMetric { field: "deltaTotalDroppedPerSec", label: "Total Dropped/Sec" },
    CaptureGraphMetric { field: "deltaSessionBytesPerSec", label: "ES Session Bytes/Sec" },
    CaptureGraphMetric { field: "sessionSizePerSec", label: "ES Session Size/Sec" },
    CaptureGraphMetric { field: "deltaWrittenBytesPerSec", label: "Written Bytes/s" },
    CaptureGraphMetric { field: "deltaUnwrittenBytesPerSec", label: "Unwritten Bytes/s" },
];

#[derive(Clone, Copy, PartialEq)]
pub enum CaptureGraphInterval {
    FiveSec,
    OneMin,
    TenMin,
}

impl CaptureGraphInterval {
    #[allow(dead_code)]
    pub const ALL: [CaptureGraphInterval; 3] = [
        CaptureGraphInterval::FiveSec,
        CaptureGraphInterval::OneMin,
        CaptureGraphInterval::TenMin,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            CaptureGraphInterval::FiveSec => "5 sec",
            CaptureGraphInterval::OneMin => "1 min",
            CaptureGraphInterval::TenMin => "10 min",
        }
    }

    pub fn seconds(&self) -> u64 {
        match self {
            CaptureGraphInterval::FiveSec => 5,
            CaptureGraphInterval::OneMin => 60,
            CaptureGraphInterval::TenMin => 600,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            CaptureGraphInterval::FiveSec => CaptureGraphInterval::OneMin,
            CaptureGraphInterval::OneMin => CaptureGraphInterval::TenMin,
            CaptureGraphInterval::TenMin => CaptureGraphInterval::FiveSec,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CaptureGraphHide {
    None,
    Old,
    NoSessions,
    Both,
}

impl CaptureGraphHide {
    pub fn label(&self) -> &'static str {
        match self {
            CaptureGraphHide::None => "None",
            CaptureGraphHide::Old => "Old",
            CaptureGraphHide::NoSessions => "No Sessions",
            CaptureGraphHide::Both => "Both",
        }
    }

    pub fn api_value(&self) -> &'static str {
        match self {
            CaptureGraphHide::None => "none",
            CaptureGraphHide::Old => "old",
            CaptureGraphHide::NoSessions => "nosession",
            CaptureGraphHide::Both => "both",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            CaptureGraphHide::None => CaptureGraphHide::Old,
            CaptureGraphHide::Old => CaptureGraphHide::NoSessions,
            CaptureGraphHide::NoSessions => CaptureGraphHide::Both,
            CaptureGraphHide::Both => CaptureGraphHide::None,
        }
    }
}

/// Per-node graph data from /api/dstats
#[derive(Clone)]
pub struct CaptureGraphNodeData {
    pub node_name: String,
    pub values: Vec<f64>,
}

/// Result from a background stats fetch
pub enum StatsFetchResult {
    /// Column-based tab result (Capture, DBStats, DBIndices, DBTasks, DBRecovery)
    Table(StatsTab, Value),
    /// DB Shards grid result
    Shards(Value),
    /// Capture Graphs per-node data
    CaptureGraphs(Vec<CaptureGraphNodeData>),
    /// Error
    Error(String),
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
    pub filter_cursor: usize,
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
    pub input_cursor: usize,
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
    pub filter_cursor: usize,
}

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub action: String, // identifier matched by the confirm handler
}
