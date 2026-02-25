use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct PlCluster {
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "type", default)]
    pub cluster_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlGroup {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub clusters: Vec<PlCluster>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlParliament {
    #[serde(default)]
    pub groups: Vec<PlGroup>,
}

#[derive(Clone, Debug)]
pub struct PlClusterStats {
    pub status: String,        // green/yellow/red
    pub health_error: String,
    pub stats_error: String,
    pub es_version: String,
    pub delta_bps: f64,
    pub delta_tdps: f64,
    pub monitoring: u64,
    pub arkime_nodes: u64,
    pub data_nodes: u64,
    pub total_nodes: u64,
}

impl PlClusterStats {
    pub fn from_value(val: &Value) -> Self {
        Self {
            status: val.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            health_error: val.get("healthError").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            stats_error: val.get("statsError").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            es_version: val.get("esVersion").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            delta_bps: val.get("deltaBPS").and_then(|v| v.as_f64()).unwrap_or(0.0),
            delta_tdps: val.get("deltaTDPS").and_then(|v| v.as_f64()).unwrap_or(0.0),
            monitoring: val.get("monitoring").and_then(|v| v.as_u64()).unwrap_or(0),
            arkime_nodes: val.get("arkimeNodes").and_then(|v| v.as_u64()).unwrap_or(0),
            data_nodes: val.get("dataNodes").and_then(|v| v.as_u64()).unwrap_or(0),
            total_nodes: val.get("totalNodes").and_then(|v| v.as_u64()).unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct PlIssue {
    #[serde(rename = "clusterId", default)]
    pub cluster_id: String,
    #[serde(default)]
    pub cluster: String,
    #[serde(rename = "type", default)]
    pub issue_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub node: String,
    #[serde(rename = "firstNoticed", default)]
    pub first_noticed: u64,
    #[serde(rename = "lastNoticed", default)]
    pub last_noticed: u64,
    #[serde(default)]
    pub acknowledged: Option<u64>,
    #[serde(rename = "ignoreUntil", default)]
    pub ignore_until: Option<serde_json::Value>,
}

impl PlIssue {
    pub fn is_ignored(&self) -> bool {
        match &self.ignore_until {
            Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
            _ => false,
        }
    }
}

impl super::ArkimeClient {
    pub async fn pl_get_parliament(&self) -> Result<PlParliament> {
        let url = format!("{}/parliament/api/parliament", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let parliament: PlParliament = serde_json::from_str(&body)?;
        Ok(parliament)
    }

    pub async fn pl_get_stats(&self) -> Result<std::collections::HashMap<String, PlClusterStats>> {
        let url = format!("{}/parliament/api/parliament/stats", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let val: Value = serde_json::from_str(&body)?;
        let mut map = std::collections::HashMap::new();
        if let Some(results) = val.get("results").and_then(|v| v.as_object()) {
            for (k, v) in results {
                map.insert(k.clone(), PlClusterStats::from_value(v));
            }
        }
        Ok(map)
    }

    pub async fn pl_get_issues(&self) -> Result<Vec<PlIssue>> {
        let url = format!("{}/parliament/api/issues", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let val: Value = serde_json::from_str(&body)?;
        let issues: Vec<PlIssue> = if let Some(arr) = val.get("issues") {
            serde_json::from_value(arr.clone())?
        } else {
            Vec::new()
        };
        Ok(issues)
    }

    /// Get issues grouped by cluster id (for dashboard inline display)
    pub async fn pl_get_issues_map(&self) -> Result<std::collections::HashMap<String, Vec<PlIssue>>> {
        let url = format!("{}/parliament/api/issues?map=true", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let val: Value = serde_json::from_str(&body)?;
        let mut map: std::collections::HashMap<String, Vec<PlIssue>> = std::collections::HashMap::new();
        if let Some(results) = val.get("results").and_then(|v| v.as_object()) {
            for (k, v) in results {
                if let Ok(issues) = serde_json::from_value::<Vec<PlIssue>>(v.clone()) {
                    map.insert(k.clone(), issues);
                }
            }
        }
        Ok(map)
    }
}
