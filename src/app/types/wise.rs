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
