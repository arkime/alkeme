use super::*;

impl App {
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
}
