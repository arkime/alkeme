use crate::api::{ArkimeClient, GraphData};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use serde_json::Value;
use std::collections::HashMap;

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

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Expression,
}

pub struct SessionDetail {
    pub data: Value,
    pub scroll: u16,
}

pub struct App {
    pub client: ArkimeClient,
    pub active_tab: Tab,
    pub time_range: TimeRange,
    pub expression: String,
    pub expression_edit: String,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub date_fields: HashMap<String, String>, // dbField -> type ("seconds" or "date")
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
    pub sort_column: usize,
    pub sort_desc: bool,
    pub graph_size: GraphSize,
    pub graph_type: GraphType,
    pub graph_data: Option<GraphData>,
    pub status_msg: String,
}

impl App {
    pub fn new(base_url: &str, auth_mode: crate::api::AuthMode, username: Option<String>, password: Option<String>) -> Self {
        Self {
            client: ArkimeClient::new(base_url, auth_mode, username, password),
            active_tab: Tab::Sessions,
            time_range: TimeRange::All,
            expression: String::new(),
            expression_edit: String::new(),
            input_mode: InputMode::Normal,
            show_help: false,
            date_fields: HashMap::new(),
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
            sort_column: 0,
            sort_desc: true,
            graph_size: GraphSize::Off,
            graph_type: GraphType::Sessions,
            graph_data: None,
            status_msg: String::new(),
        }
    }

    pub fn is_detail_view(&self) -> bool {
        self.session_view == SessionView::Detail
    }

    pub async fn fetch_fields(&mut self) {
        match self.client.get_fields().await {
            Ok((_fields, date_fields)) => {
                self.date_fields = date_fields;
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
                    self.session_detail = Some(SessionDetail { data, scroll: 0 });
                    self.session_view = SessionView::Detail;
                    self.status_msg = "Session detail loaded".into();
                }
                Err(e) => {
                    self.status_msg = format!("Error: {e}");
                }
            }
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.input_mode == InputMode::Expression {
            self.handle_expression_key(key).await;
            return;
        }
        match self.session_view {
            SessionView::List => self.handle_list_key(key).await,
            SessionView::Detail => self.handle_detail_key(key),
        }
    }

    async fn handle_expression_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.expression = self.expression_edit.clone();
                self.input_mode = InputMode::Normal;
                self.page_start = 0;
                self.fetch_sessions().await;
            }
            KeyCode::Esc => {
                self.expression_edit = self.expression.clone();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) => {
                self.expression_edit.push(c);
            }
            KeyCode::Backspace => {
                self.expression_edit.pop();
            }
            _ => {}
        }
    }

    async fn handle_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + 1) % Tab::ALL.len()];
            }
            KeyCode::BackTab => {
                let idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
                self.active_tab = Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()];
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
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.session_view = SessionView::List;
                self.session_detail = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.scroll = detail.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.scroll = detail.scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.scroll = detail.scroll.saturating_add(20);
                }
            }
            KeyCode::PageUp => {
                if let Some(ref mut detail) = self.session_detail {
                    detail.scroll = detail.scroll.saturating_sub(20);
                }
            }
            _ => {}
        }
    }
}
