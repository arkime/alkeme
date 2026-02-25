use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct WsSourceStats {
    pub source: String,
    #[serde(default)]
    pub request: u64,
    #[serde(rename = "cacheHit", default)]
    pub cache_hit: u64,
    #[serde(rename = "cacheMiss", default)]
    pub cache_miss: u64,
    #[serde(rename = "cacheRefresh", default)]
    pub cache_refresh: u64,
    #[serde(rename = "directHit", default)]
    pub direct_hit: u64,
    #[serde(rename = "requestDropped", default)]
    pub request_dropped: u64,
    #[serde(rename = "recentAverageMS", default)]
    pub recent_avg_ms: f64,
    #[serde(default)]
    pub items: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WsTypeStats {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub request: u64,
    #[serde(default)]
    pub found: u64,
    #[serde(rename = "cacheHit", default)]
    pub cache_hit: u64,
    #[serde(rename = "cacheSrcHit", default)]
    pub cache_src_hit: u64,
    #[serde(rename = "cacheSrcMiss", default)]
    pub cache_src_miss: u64,
    #[serde(rename = "cacheSrcRefresh", default)]
    pub cache_src_refresh: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WsStats {
    #[serde(default)]
    pub sources: Vec<WsSourceStats>,
    #[serde(default)]
    pub types: Vec<WsTypeStats>,
    #[serde(rename = "startTime", default)]
    #[allow(dead_code)]
    pub start_time: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WsQueryResult {
    pub field: String,
    pub value: serde_json::Value,
    #[serde(default)]
    #[allow(dead_code)]
    pub len: usize,
}

impl super::ArkimeClient {
    pub async fn ws_get_stats(&self, search: &str) -> Result<WsStats> {
        let url = if search.is_empty() {
            format!("{}/stats", self.base_url)
        } else {
            format!("{}/stats?search={}", self.base_url, urlencoding::encode(search))
        };
        let body = self.authenticated_get(&url).await?;
        let stats: WsStats = serde_json::from_str(&body)?;
        Ok(stats)
    }

    pub async fn ws_get_sources(&self) -> Result<Vec<String>> {
        let url = format!("{}/sources", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let sources: Vec<String> = serde_json::from_str(&body)?;
        Ok(sources)
    }

    pub async fn ws_get_types(&self, source: &str) -> Result<Vec<String>> {
        let url = if source.is_empty() || source == "any" {
            format!("{}/types", self.base_url)
        } else {
            format!("{}/types/{}", self.base_url, urlencoding::encode(source))
        };
        let body = self.authenticated_get(&url).await?;
        let types: Vec<String> = serde_json::from_str(&body)?;
        Ok(types)
    }

    pub async fn ws_query(&self, source: &str, type_name: &str, value: &str) -> Result<Vec<WsQueryResult>> {
        let url = if source.is_empty() || source == "any" {
            format!("{}/{}/{}", self.base_url, urlencoding::encode(type_name), urlencoding::encode(value))
        } else {
            format!("{}/{}/{}/{}", self.base_url, urlencoding::encode(source), urlencoding::encode(type_name), urlencoding::encode(value))
        };
        let body = self.authenticated_get(&url).await?;
        // API returns "Not found" as plain text when no results
        if body.trim() == "Not found" {
            return Ok(vec![]);
        }
        let results: Vec<WsQueryResult> = serde_json::from_str(&body)?;
        Ok(results)
    }
}
