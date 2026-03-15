use super::str_val;
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
    #[serde(rename = "localUrl", default)]
    pub local_url: String,
    #[serde(rename = "type", default)]
    pub cluster_type: String,
    #[serde(rename = "hideDeltaBPS", default)]
    pub hide_delta_bps: bool,
    #[serde(rename = "hideDeltaTDPS", default)]
    pub hide_delta_tdps: bool,
    #[serde(rename = "hideMonitoring", default)]
    pub hide_monitoring: bool,
    #[serde(rename = "hideArkimeNodes", default)]
    pub hide_arkime_nodes: bool,
    #[serde(rename = "hideDataNodes", default)]
    pub hide_data_nodes: bool,
    #[serde(rename = "hideTotalNodes", default)]
    pub hide_total_nodes: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlGroup {
    #[serde(default)]
    pub id: String,
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
    #[serde(default)]
    pub settings: PlSettings,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PlSettings {
    #[serde(default)]
    pub general: PlGeneralSettings,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PlGeneralSettings {
    #[serde(rename = "outOfDate", default)]
    pub out_of_date: Option<u32>,
    #[serde(rename = "esQueryTimeout", default)]
    pub es_query_timeout: Option<u32>,
    #[serde(rename = "noPackets", default)]
    pub no_packets: Option<i32>,
    #[serde(rename = "noPacketsLength", default)]
    pub no_packets_length: Option<u32>,
    #[serde(rename = "lowDiskSpace", default)]
    pub low_disk_space: Option<f64>,
    #[serde(rename = "lowDiskSpaceType", default)]
    pub low_disk_space_type: Option<String>,
    #[serde(rename = "lowDiskSpaceES", default)]
    pub low_disk_space_es: Option<f64>,
    #[serde(rename = "lowDiskSpaceESType", default)]
    pub low_disk_space_es_type: Option<String>,
    #[serde(rename = "removeIssuesAfter", default)]
    pub remove_issues_after: Option<u32>,
    #[serde(rename = "removeAcknowledgedAfter", default)]
    pub remove_acknowledged_after: Option<u32>,
    #[serde(rename = "cont3xtUrl", default)]
    pub cont3xt_url: String,
    #[serde(rename = "wiseUrl", default)]
    pub wise_url: String,
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
            status: str_val(val, "status"),
            health_error: str_val(val, "healthError"),
            stats_error: str_val(val, "statsError"),
            es_version: str_val(val, "esVersion"),
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
    fn pl_base(&self) -> &str {
        self.base_url.strip_suffix("/parliament").unwrap_or(&self.base_url)
    }

    pub async fn pl_get_parliament(&self) -> Result<PlParliament> {
        let url = format!("{}/parliament/api/parliament", self.pl_base());
        let body = self.authenticated_get(&url).await?;
        let parliament: PlParliament = serde_json::from_str(&body)?;
        Ok(parliament)
    }

    pub async fn pl_get_stats(&self) -> Result<std::collections::HashMap<String, PlClusterStats>> {
        let url = format!("{}/parliament/api/parliament/stats", self.pl_base());
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
        let url = format!("{}/parliament/api/issues", self.pl_base());
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
        let url = format!("{}/parliament/api/issues?map=true", self.pl_base());
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

    // --- Group CRUD ---

    pub async fn pl_create_group(&self, title: &str, description: &str) -> Result<()> {
        let url = format!("{}/parliament/api/groups", self.pl_base());
        let body = serde_json::json!({ "title": title, "description": description });
        let result = self.authenticated_post_json(&url, &body).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    pub async fn pl_update_group(&self, group_id: &str, title: &str, description: &str) -> Result<()> {
        let url = format!("{}/parliament/api/groups/{}", self.pl_base(), group_id);
        let body = serde_json::json!({ "title": title, "description": description });
        let result = self.authenticated_put_json(&url, &body).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    pub async fn pl_delete_group(&self, group_id: &str) -> Result<()> {
        let url = format!("{}/parliament/api/groups/{}", self.pl_base(), group_id);
        let result = self.authenticated_delete(&url).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    // --- Cluster CRUD ---

    pub async fn pl_create_cluster(&self, group_id: &str, cluster: &Value) -> Result<()> {
        let url = format!("{}/parliament/api/groups/{}/clusters", self.pl_base(), group_id);
        let result = self.authenticated_post_json(&url, cluster).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    pub async fn pl_update_cluster(&self, group_id: &str, cluster_id: &str, cluster: &Value) -> Result<()> {
        let url = format!("{}/parliament/api/groups/{}/clusters/{}", self.pl_base(), group_id, cluster_id);
        let result = self.authenticated_put_json(&url, cluster).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    pub async fn pl_delete_cluster(&self, group_id: &str, cluster_id: &str) -> Result<()> {
        let url = format!("{}/parliament/api/groups/{}/clusters/{}", self.pl_base(), group_id, cluster_id);
        let result = self.authenticated_delete(&url).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }

    // --- Settings ---

    pub async fn pl_update_settings(&self, settings: &Value) -> Result<()> {
        let url = format!("{}/parliament/api/settings", self.pl_base());
        let body = serde_json::json!({ "settings": { "general": settings } });
        let result = self.authenticated_put_json(&url, &body).await?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            anyhow::bail!("{}", text);
        }
        Ok(())
    }
}
