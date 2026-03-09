use super::*;

use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct SessionsResponse {
    pub data: Vec<Value>,
    #[serde(default, rename = "recordsTotal")]
    pub records_total: u64,
    #[serde(default, rename = "recordsFiltered")]
    pub records_filtered: u64,
    #[serde(default)]
    pub graph: Option<GraphData>,
    #[serde(default, rename = "bsqErr")]
    pub bsq_err: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ArkimeClient {
    pub async fn vr_get_sessions(&self, fields: &[String], expression: &str, date: &str, sort_field: &str, sort_desc: bool, facets: bool, start: u64, length: u64, view: &Option<String>) -> Result<SessionsResponse> {
        let fields_str = fields.join(",");
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/sessions?fields={}&length={}&start={}&flatten=1&date={}&order={}:{}",
            self.base_url, fields_str, length, start, date, urlencoding::encode(sort_field), dir
        );
        if facets {
            url.push_str("&facets=1");
        }
        Self::append_expr_view(&mut url, expression, view);

        let body = self.authenticated_get(&url).await?;
        let parsed: SessionsResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn vr_get_session(&self, id: &str) -> Result<Value> {
        let url = format!(
            "{}/api/session/{}?flatten=1&date=-1",
            self.base_url,
            urlencoding::encode(id)
        );
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    async fn vr_get_sorted_filtered(&self, endpoint: &str, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/{}?sortField={}&desc={}",
            self.base_url, endpoint, urlencoding::encode(sort_field), dir
        );
        if !filter.is_empty() {
            url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
        }
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn vr_get_stats(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        self.vr_get_sorted_filtered("stats", filter, sort_field, sort_desc).await
    }

    pub async fn vr_get_esstats(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        self.vr_get_sorted_filtered("esstats", filter, sort_field, sort_desc).await
    }

    pub async fn vr_get_esindices(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        self.vr_get_sorted_filtered("esindices", filter, sort_field, sort_desc).await
    }

    pub async fn vr_delete_esindex(&self, index: &str) -> Result<Value> {
        let url = format!("{}/api/esindices/{}", self.base_url, urlencoding::encode(index));
        self.authenticated_delete(&url).await
    }

    pub async fn vr_esindex_action(&self, index: &str, action: &str) -> Result<Value> {
        let url = format!("{}/api/esindices/{}/{}", self.base_url, urlencoding::encode(index), action);
        self.authenticated_post_json(&url, &serde_json::json!({})).await
    }

    /// Exclude or include a node by name or IP
    /// kind: "name" or "ip", action: "exclude" or "include"
    pub async fn vr_esshards_toggle(&self, kind: &str, value: &str, action: &str) -> Result<Value> {
        let url = format!("{}/api/esshards/{}/{}/{}", self.base_url, urlencoding::encode(kind), urlencoding::encode(value), action);
        self.authenticated_post_json(&url, &serde_json::json!({})).await
    }

    pub async fn vr_get_fields(&self) -> Result<(Vec<ArkimeField>, HashMap<String, String>, HashMap<String, String>, HashMap<String, String>)> {
        let url = format!("{}/api/fields?array=true", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let fields: Vec<ArkimeField> = serde_json::from_str(&body)?;
        let date_fields: HashMap<String, String> = fields
            .iter()
            .filter(|f| f.field_type == "seconds" || f.field_type == "date")
            .map(|f| (f.db_field.clone(), f.field_type.clone()))
            .collect();
        let field_exp_map: HashMap<String, String> = fields
            .iter()
            .filter(|f| !f.exp.is_empty())
            .map(|f| (f.db_field.clone(), f.exp.clone()))
            .collect();
        let field_friendly_map: HashMap<String, String> = fields
            .iter()
            .filter(|f| !f.friendly_name.is_empty())
            .map(|f| (f.db_field.clone(), f.friendly_name.clone()))
            .collect();
        Ok((fields, date_fields, field_exp_map, field_friendly_map))
    }

    pub async fn vr_download_session_pcap(&self, node: &str, id: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/api/session/{}/{}.pcap?date=-1",
            self.base_url, urlencoding::encode(node), urlencoding::encode(id)
        );
        self.authenticated_get_bytes(&url).await
    }

    pub async fn vr_download_sessions_pcap(&self, expression: &str, date: &str, view: &Option<String>) -> Result<Vec<u8>> {
        let mut url = format!("{}/api/sessions.pcap?date={}", self.base_url, urlencoding::encode(date));
        Self::append_expr_view(&mut url, expression, view);
        self.authenticated_get_bytes(&url).await
    }

    pub async fn vr_download_sessions_pcap_ids(&self, ids: &[String]) -> Result<Vec<u8>> {
        let ids_str = ids.join(",");
        let url = format!("{}/api/sessions.pcap?date=-1&ids={}", self.base_url, urlencoding::encode(&ids_str));
        self.authenticated_get_bytes(&url).await
    }

    pub async fn vr_export_sessions_csv(&self, expression: &str, date: &str, fields: &[String], view: &Option<String>) -> Result<Vec<u8>> {
        let fields_str = fields.join(",");
        let mut url = format!("{}/api/sessions/csv?date={}&fields={}", self.base_url, urlencoding::encode(date), urlencoding::encode(&fields_str));
        Self::append_expr_view(&mut url, expression, view);
        self.authenticated_get_bytes(&url).await
    }

    pub async fn vr_export_sessions_csv_ids(&self, ids: &[String], fields: &[String]) -> Result<Vec<u8>> {
        let fields_str = fields.join(",");
        let ids_str = ids.join(",");
        let url = format!("{}/api/sessions/csv?date=-1&fields={}&ids={}", self.base_url, urlencoding::encode(&fields_str), urlencoding::encode(&ids_str));
        self.authenticated_get_bytes(&url).await
    }

    pub async fn vr_add_session_tags(&self, id: &str, tags: &str) -> Result<String> {
        let url = format!("{}/api/sessions/addtags", self.base_url);
        self.authenticated_post(&url, &[("tags", tags), ("ids", id)]).await
    }

    pub async fn vr_remove_session_tags(&self, id: &str, tags: &str) -> Result<String> {
        let url = format!("{}/api/sessions/removetags", self.base_url);
        self.authenticated_post(&url, &[("tags", tags), ("ids", id)]).await
    }

    async fn vr_bulk_tag_op(&self, endpoint: &str, expression: &str, date: &str, tags: &str, view: &Option<String>) -> Result<String> {
        let mut url = format!("{}/api/sessions/{}?date={}", self.base_url, endpoint, urlencoding::encode(date));
        Self::append_expr_view(&mut url, expression, view);
        self.authenticated_post(&url, &[("tags", tags)]).await
    }

    pub async fn vr_add_sessions_tags(&self, expression: &str, date: &str, tags: &str, view: &Option<String>) -> Result<String> {
        self.vr_bulk_tag_op("addtags", expression, date, tags, view).await
    }

    pub async fn vr_remove_sessions_tags(&self, expression: &str, date: &str, tags: &str, view: &Option<String>) -> Result<String> {
        self.vr_bulk_tag_op("removetags", expression, date, tags, view).await
    }

    pub fn vr_summary_url(&self, expression: &str, date: &str, view: &Option<String>) -> String {
        let mut url = format!("{}/api/sessions/summary?date={}", self.base_url, urlencoding::encode(date));
        Self::append_expr_view(&mut url, expression, view);
        url
    }

    pub fn vr_packets_url(&self, node: &str, id: &str, raw: bool) -> String {
        format!("{}/api/session/{}/{}/packets?base=hex&ts=true&packets=10000&showFrames={}",
            self.base_url, urlencoding::encode(node), urlencoding::encode(id), raw)
    }

    // Layout API methods
    pub async fn vr_get_layouts(&self) -> Result<Value> {
        let url = format!("{}/api/user/layouts/sessionstable", self.base_url);
        let body = self.authenticated_get_with_cookie(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn vr_create_layout(&self, name: &str, columns: &[String], sort_field: &str, sort_dir: &str) -> Result<Value> {
        let url = format!("{}/api/user/layouts/sessionstable", self.base_url);
        let body = serde_json::json!({
            "name": name,
            "columns": columns,
            "order": [[sort_field, sort_dir]]
        });
        self.authenticated_post_json(&url, &body).await
    }

    pub async fn vr_update_layout(&self, name: &str, columns: &[String], sort_field: &str, sort_dir: &str) -> Result<Value> {
        let url = format!("{}/api/user/layouts/sessionstable", self.base_url);
        let body = serde_json::json!({
            "name": name,
            "columns": columns,
            "order": [[sort_field, sort_dir]]
        });
        self.authenticated_put_json(&url, &body).await
    }

    pub async fn vr_delete_layout(&self, name: &str) -> Result<Value> {
        let url = format!("{}/api/user/layouts/sessionstable/{}", self.base_url, urlencoding::encode(name));
        self.authenticated_delete(&url).await
    }

    pub async fn vr_get_views(&self) -> Result<Vec<ArkimeView>> {
        let url = format!("{}/api/views?length=1000", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        let mut views = Vec::new();
        if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
            let current_user = self.username.as_deref().unwrap_or("");
            for item in data {
                let id = str_val(item, "id");
                let name = str_val(item, "name");
                let expression = str_val(item, "expression");
                let user = str_val(item, "user");
                let shared = user != current_user;
                views.push(ArkimeView { id, name, expression, user, shared });
            }
        }
        Ok(views)
    }

    pub async fn vr_create_view(&self, name: &str, expression: &str, col_config: Option<(&[String], &str, &str)>) -> Result<Value> {
        let url = format!("{}/api/view", self.base_url);
        let mut body = serde_json::json!({
            "name": name,
            "expression": expression,
        });
        if let Some((columns, sort_field, sort_dir)) = col_config {
            body["sessionsColConfig"] = serde_json::json!({
                "visibleHeaders": columns,
                "order": [[sort_field, sort_dir]],
            });
        }
        self.authenticated_post_json(&url, &body).await
    }

    pub async fn vr_delete_view(&self, id: &str) -> Result<Value> {
        let url = format!("{}/api/view/{}", self.base_url, urlencoding::encode(id));
        self.authenticated_delete(&url).await
    }

    // Shareable API methods

    pub async fn get_shareables(&self, shareable_type: &str) -> Result<Vec<crate::app::SavedShareable>> {
        let url = format!("{}/api/shareables?type={}", self.base_url, urlencoding::encode(shareable_type));
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        let mut result = Vec::new();
        if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
            for item in data {
                let id = str_val(item, "id");
                let name = str_val(item, "name");
                let shared = item.get("shared").and_then(|v| v.as_bool()).unwrap_or(false);
                let data_obj = item.get("data").unwrap_or(&Value::Null);
                let columns: Vec<String> = data_obj.get("columns")
                    .and_then(|c| c.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let (sort_field, sort_dir) = data_obj.get("order")
                    .and_then(|o| o.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|pair| pair.as_array())
                    .map(|pair| {
                        let f = pair.first().and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let d = pair.get(1).and_then(|x| x.as_str()).unwrap_or("asc").to_string();
                        (f, d)
                    })
                    .unwrap_or_default();
                result.push(crate::app::SavedShareable { id, name, columns, sort_field, sort_dir, shared });
            }
        }
        Ok(result)
    }

    pub async fn create_shareable(&self, name: &str, shareable_type: &str, columns: &[String], sort_field: &str, sort_dir: &str) -> Result<Value> {
        let url = format!("{}/api/shareable", self.base_url);
        let body = serde_json::json!({
            "name": name,
            "type": shareable_type,
            "data": {
                "columns": columns,
                "order": [[sort_field, sort_dir]]
            }
        });
        self.authenticated_post_json(&url, &body).await
    }

    pub async fn update_shareable(&self, id: &str, name: &str, shareable_type: &str, columns: &[String], sort_field: &str, sort_dir: &str) -> Result<Value> {
        let url = format!("{}/api/shareable/{}", self.base_url, urlencoding::encode(id));
        let body = serde_json::json!({
            "name": name,
            "type": shareable_type,
            "data": {
                "columns": columns,
                "order": [[sort_field, sort_dir]]
            }
        });
        self.authenticated_put_json(&url, &body).await
    }

    pub async fn delete_shareable(&self, id: &str) -> Result<Value> {
        let url = format!("{}/api/shareable/{}", self.base_url, urlencoding::encode(id));
        self.authenticated_delete(&url).await
    }
}
