use crate::api::{ArkimeClient, GraphData};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::HashMap;

pub fn is_hidden_detail_field(key: &str) -> bool {
    key == "packetPos" || key == "packetRange" || key == "packetLen" || key.ends_with("Cnt")
}

pub fn is_non_actionable_field(key: &str) -> bool {
    key == "@timestamp"
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Arkime,
    Sessions,
    Stats,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Arkime, Tab::Sessions, Tab::Stats, Tab::Settings];

    pub fn name(&self) -> &'static str {
        match self {
            Tab::Arkime => "Arkime",
            Tab::Sessions => "Sessions",
            Tab::Stats => "Stats",
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
pub enum StatsTab {
    Capture,
    DBStats,
    DBIndices,
}

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
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Expression,
    ActionPrompt,
    DetailFilter,
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
    pub page_start: u64,
    pub page_size: u64,
    pub selected_session: usize,
    pub table_state: TableState,
    pub session_view: SessionView,
    pub session_detail: Option<SessionDetail>,
    pub detail_action_menu: Option<DetailActionMenu>,
    pub sort_column: usize,
    pub sort_desc: bool,
    pub graph_size: GraphSize,
    pub graph_type: GraphType,
    pub graph_data: Option<GraphData>,
    pub status_msg: String,
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
        Self {
            client: ArkimeClient::new(base_url, auth_mode, username, password),
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
            page_start: 0,
            page_size: 100,
            selected_session: 0,
            table_state: TableState::default().with_selected(0),
            session_view: SessionView::List,
            session_detail: None,
            detail_action_menu: None,
            sort_column: 2,
            sort_desc: true,
            graph_size: GraphSize::Off,
            graph_type: GraphType::Sessions,
            graph_data: None,
            status_msg: String::new(),
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
            Ok((_fields, date_fields, field_exp_map, field_friendly_map)) => {
                self.date_fields = date_fields;
                self.field_exp_map = field_exp_map;
                self.field_friendly_map = field_friendly_map;
            }
            Err(e) => {
                self.status_msg = format!("Error fetching fields: {e}");
            }
        }
    }

    pub async fn fetch_sessions(&mut self) {
        self.status_msg = "Fetching sessions...".into();
        let sort_field = self.session_fields.get(self.sort_column)
            .cloned()
            .unwrap_or_else(|| "firstPacket".into());
        match self.client.get_sessions(&self.session_fields, &self.expression, self.time_range.date_value(), &sort_field, self.sort_desc, self.graph_size.is_visible(), self.page_start, self.page_size).await {
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
            self.stats_detail = Some(StatsDetail { data: item.clone(), scroll: 0 });
            self.stats_view = StatsView::Detail;
        }
    }

    fn open_action_menu(&mut self, target: ActionTarget) {
        let (session_id, session_node) = match target {
            ActionTarget::Single => {
                let (id, node) = if self.session_view == SessionView::Detail {
                    let detail = self.session_detail.as_ref();
                    (
                        detail.and_then(|d| d.data.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        detail.and_then(|d| d.data.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                } else {
                    let session = self.sessions.get(self.selected_session);
                    (
                        session.and_then(|s| s.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                        session.and_then(|s| s.get("node")).and_then(|v| v.as_str()).map(|s| s.to_string()),
                    )
                };
                if id.is_none() {
                    self.status_msg = "No session selected".into();
                    return;
                }
                (id, node)
            }
            ActionTarget::All => (None, None),
        };
        self.action_menu = Some(ActionMenu {
            target,
            selected: 0,
            session_id,
            session_node,
            scope: None,
            pending_kind: None,
        });
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.action_menu.is_some() {
            self.handle_action_menu_key(key);
            return;
        }
        if self.input_mode == InputMode::ActionPrompt {
            self.handle_action_prompt_key(key).await;
            return;
        }
        if self.input_mode == InputMode::DetailFilter {
            self.handle_detail_filter_key(key);
            return;
        }
        if self.detail_action_menu.is_some() {
            self.handle_detail_action_key(key);
            return;
        }
        if self.input_mode == InputMode::Expression {
            self.handle_expression_key(key).await;
            return;
        }
        match self.active_tab {
            Tab::Stats => {
                match self.stats_view {
                    StatsView::List => self.handle_stats_key(key).await,
                    StatsView::Detail => self.handle_stats_detail_key(key),
                }
            }
            _ => {
                match self.session_view {
                    SessionView::List => self.handle_list_key(key).await,
                    SessionView::Detail => self.handle_detail_key(key),
                }
            }
        }
    }

    async fn handle_expression_key(&mut self, key: KeyEvent) {
        let is_stats = self.active_tab == Tab::Stats;
        let edit = if is_stats { &mut self.stats_filter_edit } else { &mut self.expression_edit };
        match key.code {
            KeyCode::Enter => {
                if is_stats {
                    self.stats_filter = self.stats_filter_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.fetch_stats().await;
                } else {
                    self.expression = self.expression_edit.clone();
                    self.input_mode = InputMode::Normal;
                    self.page_start = 0;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Esc => {
                if is_stats {
                    self.stats_filter_edit = self.stats_filter.clone();
                } else {
                    self.expression_edit = self.expression.clone();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Left => {
                if self.expression_cursor > 0 {
                    self.expression_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.expression_cursor < edit.len() {
                    self.expression_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.expression_cursor = 0;
            }
            KeyCode::End => {
                self.expression_cursor = edit.len();
            }
            KeyCode::Char(c) => {
                edit.insert(self.expression_cursor, c);
                self.expression_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.expression_cursor > 0 {
                    self.expression_cursor -= 1;
                    edit.remove(self.expression_cursor);
                }
            }
            KeyCode::Delete => {
                if self.expression_cursor < edit.len() {
                    edit.remove(self.expression_cursor);
                }
            }
            _ => {}
        }
    }

    async fn handle_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + 1) % Tab::ALL.len()];
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::BackTab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()];
                if self.active_tab == Tab::Stats && self.stats_data.is_empty() {
                    self.fetch_stats().await;
                }
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !self.sessions.is_empty() {
                    self.selected_session = (self.selected_session + self.visible_rows).min(self.sessions.len() - 1);
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.selected_session = self.selected_session.saturating_sub(self.visible_rows);
                self.table_state.select(Some(self.selected_session));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.sessions.is_empty() {
                    self.selected_session = (self.selected_session + 1).min(self.sessions.len() - 1);
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_session > 0 {
                    self.selected_session -= 1;
                    self.table_state.select(Some(self.selected_session));
                }
            }
            KeyCode::Enter => {
                self.open_session_detail().await;
            }
            KeyCode::Char('r') => {
                self.fetch_sessions().await;
            }
            KeyCode::Char('/') => {
                self.expression_edit = self.expression.clone();
                self.expression_cursor = self.expression_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('t') => {
                self.time_range = self.time_range.next();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('T') => {
                self.time_range = self.time_range.prev();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('s') => {
                self.sort_column = (self.sort_column + 1) % self.session_fields.len();
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Char('S') => {
                self.sort_desc = !self.sort_desc;
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.sessions_filtered > self.page_size {
                    let last_page = (self.sessions_filtered - 1) / self.page_size * self.page_size;
                    if self.page_start != last_page {
                        self.page_start = last_page;
                        self.fetch_sessions().await;
                    }
                }
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.page_start > 0 {
                    self.page_start = 0;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Right => {
                let next = self.page_start + self.page_size;
                if next < self.sessions_filtered {
                    self.page_start = next;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Left => {
                if self.page_start > 0 {
                    self.page_start = self.page_start.saturating_sub(self.page_size);
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Home => {
                if self.page_start > 0 {
                    self.page_start = 0;
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Char('g') => {
                let was_off = !self.graph_size.is_visible();
                self.graph_size = self.graph_size.next();
                if was_off && self.graph_size.is_visible() {
                    self.fetch_sessions().await;
                }
            }
            KeyCode::Char('G') => {
                if self.graph_size.is_visible() {
                    self.graph_type = self.graph_type.next();
                }
            }
            KeyCode::Char('h') => {
                self.show_help = true;
            }
            KeyCode::Char('a') => {
                self.open_action_menu(ActionTarget::Single);
            }
            KeyCode::Char('A') => {
                self.open_action_menu(ActionTarget::All);
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.session_view = SessionView::List;
                self.session_detail = None;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.session_detail {
                    if detail.total_rows > 0 && detail.selected < detail.total_rows - 1 {
                        detail.selected += 1;
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.session_detail {
                    if detail.selected > 0 {
                        detail.selected -= 1;
                    }
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = (detail.selected + self.visible_rows).min(detail.total_rows.saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.selected = detail.selected.saturating_sub(self.visible_rows);
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.session_detail {
                    if let Some(obj) = detail.data.as_object() {
                        let filter_lower = detail.filter.to_lowercase();
                        let mut keys: Vec<&String> = obj.keys()
                            .filter(|k| !is_hidden_detail_field(k))
                            .filter(|k| {
                                if filter_lower.is_empty() {
                                    return true;
                                }
                                let friendly = self.field_friendly_map.get(k.as_str())
                                    .map(|s| s.as_str())
                                    .unwrap_or(k.as_str());
                                k.to_lowercase().contains(&filter_lower)
                                    || friendly.to_lowercase().contains(&filter_lower)
                            })
                            .collect();
                        keys.sort();
                        if let Some(db_field) = keys.get(detail.selected) {
                            if is_non_actionable_field(db_field) {
                                return;
                            }
                            let val = &obj[*db_field];
                            let (val_str, values) = match val {
                                serde_json::Value::String(s) => (s.clone(), None),
                                serde_json::Value::Array(arr) => {
                                    let items: Vec<String> = arr.iter()
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            other => other.to_string(),
                                        })
                                        .collect();
                                    if items.len() == 1 {
                                        (items[0].clone(), None)
                                    } else {
                                        (items[0].clone(), Some(items))
                                    }
                                }
                                serde_json::Value::Null => ("-".into(), None),
                                other => (other.to_string(), None),
                            };
                            let exp_name = self.field_exp_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            let friendly = self.field_friendly_map.get(db_field.as_str())
                                .cloned()
                                .unwrap_or_else(|| (*db_field).clone());
                            self.detail_action_menu = Some(DetailActionMenu {
                                field: exp_name,
                                display: friendly,
                                value: val_str,
                                selected: 0,
                                values,
                                value_selected: 0,
                            });
                        }
                    }
                }
            }
            KeyCode::Char('a') => {
                self.open_action_menu(ActionTarget::Single);
            }
            KeyCode::Char('A') => {
                self.open_action_menu(ActionTarget::All);
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::DetailFilter;
            }
            _ => {}
        }
    }

    fn handle_detail_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.filter.clear();
                    self.recalc_detail_rows();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.filter.push(c);
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.filter.pop();
                    detail.selected = 0;
                    detail.scroll = 0;
                    self.recalc_detail_rows();
                }
            }
            _ => {}
        }
    }

    fn recalc_detail_rows(&mut self) {
        if let Some(ref mut detail) = self.session_detail {
            if let Some(obj) = detail.data.as_object() {
                let filter_lower = detail.filter.to_lowercase();
                detail.total_rows = obj.keys()
                    .filter(|k| !is_hidden_detail_field(k))
                    .filter(|k| {
                        if filter_lower.is_empty() {
                            return true;
                        }
                        let friendly = self.field_friendly_map.get(k.as_str())
                            .map(|s| s.as_str())
                            .unwrap_or(k.as_str());
                        k.to_lowercase().contains(&filter_lower)
                            || friendly.to_lowercase().contains(&filter_lower)
                    })
                    .count();
            }
        }
    }

    fn handle_action_menu_key(&mut self, key: KeyEvent) {
        let remove_enabled = self.remove_enabled();
        let menu = match &mut self.action_menu {
            Some(m) => m,
            None => return,
        };
        let in_scope = menu.scope.is_some();
        match key.code {
            KeyCode::Esc => {
                if in_scope {
                    let menu = self.action_menu.as_mut().unwrap();
                    menu.scope = None;
                    menu.selected = 0;
                } else {
                    self.action_menu = None;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if in_scope {
                    menu.selected = (menu.selected + 1).min(1);
                } else {
                    let len = menu.options(remove_enabled).len();
                    menu.selected = (menu.selected + 1).min(len - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if menu.selected > 0 {
                    menu.selected -= 1;
                }
            }
            KeyCode::Enter => {
                if in_scope {
                    let scope = if menu.selected == 0 { ActionScope::Visible } else { ActionScope::Matching };
                    let kind = menu.pending_kind.unwrap();
                    let target = menu.target;
                    let session_id = menu.session_id.clone();
                    let session_node = menu.session_node.clone();
                    let default_input = match kind {
                        ActionKind::DownloadPcap => "sessions.pcap".to_string(),
                        ActionKind::ExportCsv => "sessions.csv".to_string(),
                        _ => String::new(),
                    };
                    self.action_menu = None;
                    self.action_prompt = Some(ActionPrompt {
                        kind,
                        target,
                        scope,
                        session_id,
                        session_node,
                        input: default_input,
                    });
                    self.input_mode = InputMode::ActionPrompt;
                    return;
                }

                let options = menu.options(remove_enabled);
                let kind = options[menu.selected];
                let target = menu.target;
                let session_id = menu.session_id.clone();
                let session_node = menu.session_node.clone();

                // For ALL PCAP/CSV, show scope selector first
                if target == ActionTarget::All
                    && (kind == ActionKind::DownloadPcap || kind == ActionKind::ExportCsv)
                {
                    let menu = self.action_menu.as_mut().unwrap();
                    menu.pending_kind = Some(kind);
                    menu.scope = Some(ActionScope::Visible);
                    menu.selected = 0;
                    return;
                }

                let default_input = match kind {
                    ActionKind::DownloadPcap => {
                        match target {
                            ActionTarget::Single => {
                                format!("{}.pcap", session_id.as_deref().unwrap_or("session"))
                            }
                            ActionTarget::All => "sessions.pcap".to_string(),
                        }
                    }
                    ActionKind::ExportCsv => "sessions.csv".to_string(),
                    ActionKind::AddTags | ActionKind::RemoveTags => String::new(),
                };
                self.action_menu = None;
                self.action_prompt = Some(ActionPrompt {
                    kind,
                    target,
                    scope: ActionScope::Matching,
                    session_id,
                    session_node,
                    input: default_input,
                });
                self.input_mode = InputMode::ActionPrompt;
            }
            _ => {}
        }
    }

    async fn handle_action_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.action_prompt = None;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                let prompt = match self.action_prompt.take() {
                    Some(p) => p,
                    None => return,
                };
                self.input_mode = InputMode::Normal;
                if prompt.input.is_empty() {
                    self.status_msg = "No input provided".into();
                    return;
                }
                self.execute_action(prompt).await;
            }
            KeyCode::Char(c) => {
                if let Some(ref mut prompt) = self.action_prompt {
                    prompt.input.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut prompt) = self.action_prompt {
                    prompt.input.pop();
                }
            }
            _ => {}
        }
    }

    fn visible_session_ids(&self) -> Vec<String> {
        self.sessions.iter()
            .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect()
    }

    async fn execute_action(&mut self, prompt: ActionPrompt) {
        let date = self.time_range.date_value();
        match (prompt.kind, prompt.target) {
            (ActionKind::DownloadPcap, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                let node = prompt.session_node.as_deref().unwrap_or("");
                self.status_msg = "Downloading PCAP...".into();
                match self.client.download_session_pcap(node, id).await {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::DownloadPcap, ActionTarget::All) => {
                self.status_msg = "Downloading PCAP...".into();
                let result = if prompt.scope == ActionScope::Visible {
                    let ids = self.visible_session_ids();
                    self.client.download_sessions_pcap_ids(&ids).await
                } else {
                    self.client.download_sessions_pcap(&self.expression, date).await
                };
                match result {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::ExportCsv, ActionTarget::All) => {
                self.status_msg = "Exporting CSV...".into();
                let result = if prompt.scope == ActionScope::Visible {
                    let ids = self.visible_session_ids();
                    self.client.export_sessions_csv_ids(&ids, &self.session_fields).await
                } else {
                    self.client.export_sessions_csv(&self.expression, date, &self.session_fields).await
                };
                match result {
                    Ok(data) => {
                        match std::fs::write(&prompt.input, &data) {
                            Ok(_) => self.status_msg = format!("Saved {} ({} bytes)", prompt.input, data.len()),
                            Err(e) => self.status_msg = format!("Error writing file: {e}"),
                        }
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::AddTags, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                self.status_msg = "Adding tags...".into();
                match self.client.add_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::AddTags, ActionTarget::All) => {
                self.status_msg = "Adding tags...".into();
                match self.client.add_sessions_tags(&self.expression, date, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags added: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::Single) => {
                let id = prompt.session_id.as_deref().unwrap_or("");
                self.status_msg = "Removing tags...".into();
                match self.client.remove_session_tags(id, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            (ActionKind::RemoveTags, ActionTarget::All) => {
                self.status_msg = "Removing tags...".into();
                match self.client.remove_sessions_tags(&self.expression, date, &prompt.input).await {
                    Ok(_) => {
                        self.status_msg = format!("Tags removed: {}", prompt.input);
                        self.fetch_sessions().await;
                    }
                    Err(e) => self.status_msg = format!("Error: {e}"),
                }
            }
            _ => {}
        }
    }

    fn handle_detail_action_key(&mut self, key: KeyEvent) {
        let in_values = self.detail_action_menu.as_ref()
            .map(|m| m.values.is_some()).unwrap_or(false);

        match key.code {
            KeyCode::Esc => {
                self.detail_action_menu = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut menu) = self.detail_action_menu {
                    if in_values {
                        let len = menu.values.as_ref().unwrap().len();
                        menu.value_selected = (menu.value_selected + 1).min(len - 1);
                    } else {
                        menu.selected = (menu.selected + 1).min(DetailActionMenu::OPTIONS.len() - 1);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut menu) = self.detail_action_menu {
                    if in_values {
                        if menu.value_selected > 0 {
                            menu.value_selected -= 1;
                        }
                    } else {
                        if menu.selected > 0 {
                            menu.selected -= 1;
                        }
                    }
                }
            }
            KeyCode::Enter => {
                if in_values {
                    // Pick the selected value, then show AND/OR options
                    if let Some(ref mut menu) = self.detail_action_menu {
                        let chosen = menu.values.as_ref().unwrap()[menu.value_selected].clone();
                        menu.value = chosen;
                        menu.values = None;
                        menu.selected = 0;
                    }
                } else if let Some(menu) = self.detail_action_menu.take() {
                    let needs_quotes = menu.value.parse::<f64>().is_err();
                    let quoted_val = if needs_quotes {
                        format!("\"{}\"", menu.value)
                    } else {
                        menu.value.clone()
                    };

                    let (connector, op) = match menu.selected {
                        0 => ("&&", "=="),
                        1 => ("&&", "!="),
                        2 => ("||", "=="),
                        3 => ("||", "!="),
                        _ => ("&&", "=="),
                    };

                    let clause = format!("{} {} {}", menu.field, op, quoted_val);

                    if self.expression.is_empty() {
                        self.expression = clause;
                    } else {
                        self.expression = format!("{} {} {}", self.expression, connector, clause);
                    }
                    self.expression_edit = self.expression.clone();
                }
            }
            _ => {}
        }
    }

    async fn handle_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + 1) % Tab::ALL.len()];
            }
            KeyCode::BackTab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()];
            }
            KeyCode::Char('1') => {
                if self.stats_tab != StatsTab::Capture {
                    self.stats_tab = StatsTab::Capture;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Char('2') => {
                if self.stats_tab != StatsTab::DBStats {
                    self.stats_tab = StatsTab::DBStats;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Char('3') => {
                if self.stats_tab != StatsTab::DBIndices {
                    self.stats_tab = StatsTab::DBIndices;
                    self.stats_sort_column = 0;
                    self.stats_sort_desc = false;
                    self.fetch_stats().await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.stats_data.is_empty() {
                    self.stats_selected = (self.stats_selected + 1).min(self.stats_data.len() - 1);
                    self.stats_table_state.select(Some(self.stats_selected));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.stats_selected > 0 {
                    self.stats_selected -= 1;
                    self.stats_table_state.select(Some(self.stats_selected));
                }
            }
            KeyCode::Enter => {
                self.open_stats_detail();
            }
            KeyCode::Char('r') => {
                self.fetch_stats().await;
            }
            KeyCode::Char('/') => {
                self.stats_filter_edit = self.stats_filter.clone();
                self.expression_cursor = self.stats_filter_edit.len();
                self.input_mode = InputMode::Expression;
            }
            KeyCode::Char('s') => {
                let num_cols = self.stats_tab.columns().len();
                self.stats_sort_column = (self.stats_sort_column + 1) % num_cols;
                self.fetch_stats().await;
            }
            KeyCode::Char('S') => {
                self.stats_sort_desc = !self.stats_sort_desc;
                self.fetch_stats().await;
            }
            KeyCode::Char('h') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn handle_stats_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.stats_view = StatsView::List;
                self.stats_detail = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_add(20);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.stats_detail {
                    detail.scroll = detail.scroll.saturating_sub(20);
                }
            }
            _ => {}
        }
    }
}
