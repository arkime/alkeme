use anyhow::Result;
use reqwest::Client;
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
}

#[derive(Deserialize, Clone, Default)]
pub struct GraphData {
    #[serde(default, rename = "sessionsHisto")]
    pub sessions_histo: Vec<(f64, f64)>,
    #[serde(default, rename = "source.packetsHisto")]
    pub src_packets_histo: Vec<(f64, f64)>,
    #[serde(default, rename = "destination.packetsHisto")]
    pub dst_packets_histo: Vec<(f64, f64)>,
    #[serde(default, rename = "source.bytesHisto")]
    pub src_bytes_histo: Vec<(f64, f64)>,
    #[serde(default, rename = "destination.bytesHisto")]
    pub dst_bytes_histo: Vec<(f64, f64)>,
}

#[derive(Deserialize)]
pub struct ArkimeField {
    #[serde(default, rename = "dbField")]
    pub db_field: String,
    #[serde(default, rename = "type")]
    pub field_type: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AuthMode {
    None,
    Basic,
    Digest,
}

pub struct ArkimeClient {
    client: Client,
    base_url: String,
    auth_mode: AuthMode,
    username: Option<String>,
    password: Option<String>,
}

impl ArkimeClient {
    pub fn new(base_url: &str, auth_mode: AuthMode, username: Option<String>, password: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_mode,
            username,
            password,
        }
    }

    async fn authenticated_get(&self, url: &str) -> Result<String> {
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                return Ok(resp.text().await?);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                Ok(resp.text().await?)
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed", resp.status());
                }
                Ok(resp.text().await?)
            }
            AuthMode::Digest => {
                // First request to get the WWW-Authenticate challenge
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return Ok(resp.text().await?);
                }

                let www_auth = resp
                    .headers()
                    .get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header in 401 response"))?
                    .to_string();

                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else {
                    parsed_url.path().to_string()
                };
                let context = digest_auth::AuthContext::new(username, password, &uri);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();

                let resp = self.client
                    .get(url)
                    .header("Authorization", auth_header)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed", resp.status());
                }

                Ok(resp.text().await?)
            }
        }
    }

    pub async fn get_sessions(&self, fields: &[String], expression: &str, date: &str, sort_field: &str, sort_desc: bool, facets: bool, start: u64, length: u64) -> Result<SessionsResponse> {
        let fields_str = fields.join(",");
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/sessions?fields={}&length={}&start={}&flatten=1&date={}&order={}:{}",
            self.base_url, fields_str, length, start, date, urlencoding::encode(sort_field), dir
        );
        if facets {
            url.push_str("&facets=1");
        }
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }

        let body = self.authenticated_get(&url).await?;
        let parsed: SessionsResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn get_session(&self, id: &str) -> Result<Value> {
        let url = format!(
            "{}/api/session/{}?flatten=1&date=-1",
            self.base_url,
            urlencoding::encode(id)
        );
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn get_fields(&self) -> Result<(Vec<ArkimeField>, HashMap<String, String>)> {
        let url = format!("{}/api/fields?array=true", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let fields: Vec<ArkimeField> = serde_json::from_str(&body)?;
        let date_fields: HashMap<String, String> = fields
            .iter()
            .filter(|f| f.field_type == "seconds" || f.field_type == "date")
            .map(|f| (f.db_field.clone(), f.field_type.clone()))
            .collect();
        Ok((fields, date_fields))
    }
}
