mod viewer;
mod cont3xt;
mod parliament;

pub use cont3xt::*;
pub use parliament::*;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct HttpLogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub method: String,
    pub url: String,
    pub post_data: Option<String>,
    pub status: u16,
    pub first_byte_ms: u64,
    pub last_byte_ms: u64,
    pub response_body: Option<String>,
}

pub type HttpLog = Arc<Mutex<Vec<HttpLogEntry>>>;

pub fn new_http_log() -> HttpLog {
    Arc::new(Mutex::new(Vec::new()))
}

pub(crate) fn log_http(log: &HttpLog, method: &str, url: &str, post_data: Option<String>, status: u16, first_byte_ms: u64, last_byte_ms: u64, response_body: Option<&str>) {
    if let Ok(mut entries) = log.lock() {
        let resp = if status >= 300 {
            response_body.map(|b| {
                let truncated = if b.len() > 200 { &b[..200] } else { b };
                truncated.to_string()
            })
        } else {
            None
        };
        entries.push(HttpLogEntry {
            timestamp: chrono::Local::now(),
            method: method.to_string(),
            url: url.to_string(),
            post_data,
            status,
            first_byte_ms,
            last_byte_ms,
            response_body: resp,
        });
    }
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
    #[serde(default)]
    pub exp: String,
    #[serde(default, rename = "friendlyName")]
    pub friendly_name: String,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default, rename = "noFacet")]
    pub no_facet: Option<String>,
}

impl ArkimeField {
    /// Fields with regex or noFacet="true" should be hidden from user selectors
    pub fn is_visible(&self) -> bool {
        self.regex.is_none() && self.no_facet.as_deref() != Some("true")
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ArkimeView {
    pub id: String,
    pub name: String,
    pub expression: String,
    pub user: String,
    pub shared: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AuthMode {
    None,
    Basic,
    Digest,
    Form,
}

#[derive(Clone)]
pub struct Packet {
    pub src: bool,
    pub bytes: u32,
    pub timestamp: Option<u64>,
    pub flags: String,
    pub lines: Vec<String>,
}

#[derive(Clone)]
pub struct PacketsData {
    pub src_label: String,
    pub dst_label: String,
    pub packets: Vec<Packet>,
    pub total: u64,
}

#[derive(Deserialize, Clone)]
pub struct SummaryItem {
    pub item: Value,
    #[serde(default)]
    pub sessions: u64,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub packets: u64,
}

/// A saved cont3xt view (integration set)
#[derive(Clone)]
pub struct Cont3xtView {
    pub id: String,
    pub name: String,
    pub integrations: Vec<String>,
    #[allow(dead_code)]
    pub creator: String,
    pub editable: bool,
}

/// A cont3xt integration definition from /api/integration
#[derive(Clone)]
pub struct Cont3xtIntegration {
    pub name: String,
    #[allow(dead_code)]
    pub doable: bool,
    pub order: u32,
    pub card: Option<Cont3xtCard>,
}

/// Card display definition for an integration
#[derive(Clone)]
pub struct Cont3xtCard {
    #[allow(dead_code)]
    pub title: String,
    pub fields: Vec<CardField>,
}

/// A field in a card definition
#[derive(Clone)]
pub struct CardField {
    pub label: String,
    pub field: String,              // dot-joined path for data traversal
    pub field_type: String,         // string, url, date, ms, seconds, array, table, json, dnsRecords
    pub join: Option<String>,       // for array type, join separator
    pub fields: Vec<CardField>,     // for table type, sub-fields
    pub defang: bool,
    pub field_root: Option<String>, // dot-joined fieldRootPath
    #[allow(dead_code)]
    pub filter_empty: bool,         // filter null/empty values from arrays/tables
}

/// A search result from one integration
#[derive(Clone)]
pub struct Cont3xtResult {
    pub name: String,
    pub indicator: String,
    #[allow(dead_code)]
    pub itype: String,
    pub data: Value,
    #[allow(dead_code)]
    pub has_data: bool,
}

pub fn parse_card_field(val: &Value) -> CardField {
    match val {
        Value::String(s) => CardField {
            label: s.clone(),
            field: s.clone(),
            field_type: "string".to_string(),
            join: None,
            fields: Vec::new(),
            defang: false,
            field_root: None,
            filter_empty: false,
        },
        Value::Object(obj) => {
            let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
            // Server normalizes "field" -> "path" (array of strings)
            let field = if let Some(path_arr) = obj.get("path").and_then(|v| v.as_array()) {
                path_arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                obj.get("field").and_then(|v| v.as_str()).unwrap_or(&label).to_string()
            };
            let field_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("string").to_string();
            let join = obj.get("join").and_then(|v| v.as_str()).map(|s| s.to_string());
            let defang = obj.get("defang").and_then(|v| v.as_bool()).unwrap_or(false);
            // Server normalizes "fieldRoot" -> "fieldRootPath" (array of strings)
            let field_root = if let Some(path_arr) = obj.get("fieldRootPath").and_then(|v| v.as_array()) {
                Some(path_arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("."))
            } else {
                obj.get("fieldRoot").and_then(|v| v.as_str()).map(|s| s.to_string())
            };
            let filter_empty = obj.get("filterEmpty").and_then(|v| v.as_bool()).unwrap_or(false);
            let fields = obj.get("fields")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().map(parse_card_field).collect())
                .unwrap_or_default();
            CardField { label, field, field_type, join, fields, defang, field_root, filter_empty }
        }
        _ => CardField {
            label: String::new(),
            field: String::new(),
            field_type: "string".to_string(),
            join: None,
            fields: Vec::new(),
            defang: false,
            field_root: None,
            filter_empty: false,
        },
    }
}

pub fn parse_card(val: &Value) -> Option<Cont3xtCard> {
    let obj = val.as_object()?;
    let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let fields = obj.get("fields")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(parse_card_field).collect())
        .unwrap_or_default();
    Some(Cont3xtCard { title, fields })
}

pub struct FetchClient {
    pub(super) client: Client,
    pub(super) auth_mode: AuthMode,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) arkime_cookie: Option<String>,
    pub(super) cookie_header_name: &'static str,
    pub(super) http_log: HttpLog,
}

impl FetchClient {
    pub async fn fetch_url(&self, url: &str) -> Result<String> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?.to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else {
                    parsed_url.path().to_string()
                };
                let context = digest_auth::AuthContext::new(username, password, &uri);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                let resp = self.client.get(url).header("Authorization", auth_header).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Form => {
                let mut req = self.client.get(url);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header("Cookie", cookie.as_str());
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
        }
    }

    pub async fn fetch_post(&self, url: &str, form: &[(&str, &str)]) -> Result<String> {
        let start = Instant::now();
        let post_data = Some(form.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("&"));
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.post(url).form(form).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.post(url).form(form).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Basic => {
                let mut req = self.client.post(url)
                    .basic_auth(username, Some(password))
                    .form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Digest => {
                let resp = self.client.post(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?.to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                let mut req = self.client.post(url).header("Authorization", auth_header).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Form => {
                let mut req = self.client.post(url).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
        }
    }

    #[allow(dead_code)]
    pub async fn fetch_post_json(&self, url: &str, json_body: &str) -> Result<String> {
        let start = Instant::now();
        let post_data = Some(json_body.to_string());
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.post(url)
                    .header("Content-Type", "application/json")
                    .body(json_body.to_string())
                    .send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        let mut req = match self.auth_mode {
            AuthMode::None => self.client.post(url),
            AuthMode::Basic => self.client.post(url).basic_auth(username, Some(password)),
            AuthMode::Digest => {
                let resp = self.client.post(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?.to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                self.client.post(url).header("Authorization", auth_header)
            }
            AuthMode::Form => self.client.post(url),
        };
        req = req.header("Content-Type", "application/json").body(json_body.to_string());
        if let Some(ref cookie) = self.arkime_cookie {
            if self.auth_mode != AuthMode::Form {
                req = req.header("Cookie", cookie.as_str());
            }
            req = req.header(self.cookie_header_name, cookie.as_str());
        }
        let resp = req.send().await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
        Ok(body)
    }
}

#[derive(Clone)]
pub struct ArkimeClient {
    pub(super) client: Client,
    pub(super) base_url: String,
    pub(super) auth_mode: AuthMode,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(crate) logged_in: bool,
    pub(super) arkime_cookie: Option<String>,
    pub(super) cookie_header_name: &'static str,
    pub(super) http_log: HttpLog,
}

impl ArkimeClient {
    pub fn new(base_url: &str, auth_mode: AuthMode, username: Option<String>, password: Option<String>) -> Self {
        let mut builder = Client::builder()
            .danger_accept_invalid_certs(true);
        if auth_mode == AuthMode::Form {
            builder = builder.cookie_store(true).redirect(reqwest::redirect::Policy::none());
        }
        Self {
            client: builder.build().expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_mode,
            username,
            password,
            logged_in: false,
            arkime_cookie: None,
            cookie_header_name: "x-arkime-cookie",
            http_log: new_http_log(),
        }
    }

    pub fn http_log(&self) -> HttpLog {
        self.http_log.clone()
    }

    pub fn auth_mode(&self) -> AuthMode {
        self.auth_mode
    }

    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn clone_for_fetch(&self) -> FetchClient {
        FetchClient {
            client: self.client.clone(),
            auth_mode: self.auth_mode,
            username: self.username.clone(),
            password: self.password.clone(),
            arkime_cookie: self.arkime_cookie.clone(),
            cookie_header_name: self.cookie_header_name,
            http_log: self.http_log.clone(),
        }
    }

    pub async fn login(&mut self) -> Result<()> {
        if self.auth_mode != AuthMode::Form || self.logged_in {
            return Ok(());
        }
        let username = self.username.as_deref().unwrap_or("");
        let password = self.password.as_deref().unwrap_or("");
        let url = format!("{}/api/login", self.base_url);
        let resp = self.client.post(&url)
            .form(&[("username", username), ("password", password)])
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::FOUND {
            if let Some(loc) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
                if loc.contains("/auth") {
                    anyhow::bail!("Form login failed: invalid username or password");
                }
            }
        } else if !resp.status().is_success() {
            anyhow::bail!("Form login failed: HTTP {}", resp.status());
        }
        let settings_url = format!("{}/api/user/settings", self.base_url);
        let resp = self.client.get(&settings_url).send().await?;
        self.extract_cookie(&resp);
        self.logged_in = true;
        Ok(())
    }

    pub(super) async fn authenticated_get(&self, url: &str) -> Result<String> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
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
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();

                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }

                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Form => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
        }
    }

    /// Like authenticated_get but sends x-arkime-cookie header (for endpoints with checkCookieToken)
    pub(super) async fn authenticated_get_with_cookie(&self, url: &str) -> Result<String> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let mut req = self.client.get(url);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let mut req = self.client.get(url);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Basic => {
                let mut req = self.client.get(url)
                    .basic_auth(username, Some(password));
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?
                    .to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new(username, password, &uri);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                let mut req = self.client.get(url).header("Authorization", auth_header);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
                    anyhow::bail!("HTTP {}: {}", status, text);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Form => {
                let mut req = self.client.get(url);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
        }
    }

    pub(super) async fn authenticated_post(&self, url: &str, form: &[(&str, &str)]) -> Result<String> {
        let start = Instant::now();
        let post_data = Some(form.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("&"));
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.post(url).form(form).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                return Ok(body);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.post(url).form(form).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Basic => {
                let mut req = self.client.post(url)
                    .basic_auth(username, Some(password))
                    .form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Digest => {
                let resp = self.client.post(url).form(form).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                    return Ok(body);
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
                let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();

                let mut req = self.client
                    .post(url)
                    .header("Authorization", auth_header)
                    .form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();

                if !resp.status().is_success() {
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }

                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
            AuthMode::Form => {
                let mut req = self.client.post(url).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie);
                }
                let resp = req.send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", status);
                }
                let body = resp.text().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&body));
                Ok(body)
            }
        }
    }

    pub(super) async fn authenticated_get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let bytes = resp.bytes().await?.to_vec();
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                return Ok(bytes);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let bytes = resp.bytes().await?.to_vec();
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                Ok(bytes)
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }
                let bytes = resp.bytes().await?.to_vec();
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                Ok(bytes)
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let bytes = resp.bytes().await?.to_vec();
                    log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                    return Ok(bytes);
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
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();

                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed", status);
                }

                let bytes = resp.bytes().await?.to_vec();
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                Ok(bytes)
            }
            AuthMode::Form => {
                let resp = self.client.get(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                if !resp.status().is_success() {
                    log_http(&self.http_log, "GET", url, None, status, first_byte, first_byte, None);
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", status);
                }
                let bytes = resp.bytes().await?.to_vec();
                log_http(&self.http_log, "GET", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                Ok(bytes)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn append_view(url: &mut String, view: &Option<String>) {
        if let Some(v) = view {
            if !v.is_empty() {
                url.push_str(&format!("&view={}", urlencoding::encode(v)));
            }
        }
    }

    pub async fn get_user(&self) -> Result<Value> {
        let url = format!("{}/api/user", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let user: Value = serde_json::from_str(&body)?;
        Ok(user)
    }

    pub async fn get_appversion(&self) -> Result<Value> {
        let url = format!("{}/api/appversion", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let val: Value = serde_json::from_str(&body)?;
        Ok(val)
    }

    /// Fetch the ARKIME-COOKIE from /api/user/settings (has setCookie middleware).
    /// Must be called once at startup for layout API support.
    pub async fn fetch_cookie(&mut self) -> Result<()> {
        let url = format!("{}/api/user/settings", self.base_url);
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => return Ok(()),
        };
        let password = self.password.as_deref().unwrap_or("");

        let resp = match self.auth_mode {
            AuthMode::None => self.client.get(&url).send().await?,
            AuthMode::Basic => {
                self.client.get(&url)
                    .basic_auth(username, Some(password))
                    .send().await?
            }
            AuthMode::Digest => {
                let resp = self.client.get(&url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    self.extract_cookie(&resp);
                    return Ok(());
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let parsed_url = reqwest::Url::parse(&url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new(username, password, &uri);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                self.client.get(&url).header("Authorization", auth_header).send().await?
            }
            AuthMode::Form => return Ok(()), // already captured during login
        };
        self.extract_cookie(&resp);
        Ok(())
    }

    fn extract_cookie(&mut self, resp: &reqwest::Response) {
        for cookie_val in resp.headers().get_all("set-cookie") {
            if let Ok(s) = cookie_val.to_str() {
                if let Some(val) = s.strip_prefix("CONT3XT-COOKIE=") {
                    let val = val.split(';').next().unwrap_or(val);
                    self.arkime_cookie = Some(val.to_string());
                    self.cookie_header_name = "x-cont3xt-cookie";
                } else if let Some(val) = s.strip_prefix("ARKIME-COOKIE=") {
                    let val = val.split(';').next().unwrap_or(val);
                    self.arkime_cookie = Some(val.to_string());
                    self.cookie_header_name = "x-arkime-cookie";
                }
            }
        }
    }

    pub(super) async fn authenticated_post_json(&self, url: &str, body: &Value) -> Result<Value> {
        let start = Instant::now();
        let post_data = Some(body.to_string());
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.post(url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                    .send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let result = resp.json().await?;
                log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, None);
                return Ok(result);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        let mut req = match self.auth_mode {
            AuthMode::None => self.client.post(url),
            AuthMode::Basic => self.client.post(url).basic_auth(username, Some(password)),
            AuthMode::Digest => {
                let resp = self.client.post(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let text = resp.text().await?;
                    log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
                    return Ok(serde_json::from_str(&text)?);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                self.client.post(url).header("Authorization", auth_header)
            }
            AuthMode::Form => self.client.post(url),
        };
        if let Some(ref cookie) = self.arkime_cookie {
            req = req.header(self.cookie_header_name, cookie);
        }
        let resp = req
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send().await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        let result = resp.json().await?;
        log_http(&self.http_log, "POST", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, None);
        Ok(result)
    }

    pub(super) async fn authenticated_put_json(&self, url: &str, body: &Value) -> Result<Value> {
        let start = Instant::now();
        let post_data = Some(body.to_string());
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.put(url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                    .send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let result = resp.json().await?;
                log_http(&self.http_log, "PUT", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, None);
                return Ok(result);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        let mut req = match self.auth_mode {
            AuthMode::None => self.client.put(url),
            AuthMode::Basic => self.client.put(url).basic_auth(username, Some(password)),
            AuthMode::Digest => {
                let resp = self.client.put(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let text = resp.text().await?;
                    log_http(&self.http_log, "PUT", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
                    return Ok(serde_json::from_str(&text)?);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new_with_method::<_, _, _, &[u8]>(username, password, &uri, None, digest_auth::HttpMethod::PUT);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                self.client.put(url).header("Authorization", auth_header)
            }
            AuthMode::Form => self.client.put(url),
        };
        if let Some(ref cookie) = self.arkime_cookie {
            req = req.header(self.cookie_header_name, cookie);
        }
        let resp = req
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send().await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            log_http(&self.http_log, "PUT", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        let result = resp.json().await?;
        log_http(&self.http_log, "PUT", url, post_data, status, first_byte, start.elapsed().as_millis() as u64, None);
        Ok(result)
    }

    pub(super) async fn authenticated_delete(&self, url: &str) -> Result<Value> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.delete(url).send().await?;
                let first_byte = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                let result = resp.json().await?;
                log_http(&self.http_log, "DELETE", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
                return Ok(result);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        let mut req = match self.auth_mode {
            AuthMode::None => self.client.delete(url),
            AuthMode::Basic => self.client.delete(url).basic_auth(username, Some(password)),
            AuthMode::Digest => {
                let resp = self.client.delete(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    let first_byte = start.elapsed().as_millis() as u64;
                    let status = resp.status().as_u16();
                    let text = resp.text().await?;
                    log_http(&self.http_log, "DELETE", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
                    return Ok(serde_json::from_str(&text)?);
                }
                let www_auth = resp.headers().get("www-authenticate")
                    .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                let parsed_url = reqwest::Url::parse(url)?;
                let uri = if let Some(q) = parsed_url.query() {
                    format!("{}?{}", parsed_url.path(), q)
                } else { parsed_url.path().to_string() };
                let context = digest_auth::AuthContext::new_with_method::<_, _, _, &[u8]>(username, password, &uri, None, digest_auth::HttpMethod::DELETE);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();
                self.client.delete(url).header("Authorization", auth_header)
            }
            AuthMode::Form => self.client.delete(url),
        };
        if let Some(ref cookie) = self.arkime_cookie {
            req = req.header(self.cookie_header_name, cookie);
        }
        let resp = req.send().await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            log_http(&self.http_log, "DELETE", url, None, status, first_byte, start.elapsed().as_millis() as u64, Some(&text));
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        let result = resp.json().await?;
        log_http(&self.http_log, "DELETE", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
        Ok(result)
    }
}

pub fn parse_packets_html(html: &str) -> PacketsData {
    let mut packets = Vec::new();

    let re_src = regex::Regex::new(r#"class="srccol".*?<span class="small">&nbsp;\(([^)]+)\)"#).unwrap();
    let re_dst = regex::Regex::new(r#"class="dstcol".*?<span class="small">&nbsp;\(([^)]+)\)"#).unwrap();
    let src_label = re_src.captures(html).map(|c| c[1].to_string()).unwrap_or_default();
    let dst_label = re_dst.captures(html).map(|c| c[1].to_string()).unwrap_or_default();
    // Split on packet container divs: sessionsrc or sessiondst
    let re_packet = regex::Regex::new(r#"class="col-md-6[^"]*\s+(sessionsrc|sessiondst)">([\s\S]*?)</pre>"#).unwrap();
    let re_bytes = regex::Regex::new(r#">(\d+)&nbsp;<span class="bytes">"#).unwrap();
    let re_ts = regex::Regex::new(r#"class="session-detail-ts"[^>]*value="(\d+)"#).unwrap();
    let re_flags = regex::Regex::new(r#"</em>([\s\S]*?)<span class="pull-right">"#).unwrap();
    let re_pre = regex::Regex::new(r"<pre>([\s\S]*?)$").unwrap();
    let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();

    for cap in re_packet.captures_iter(html) {
        let is_src = &cap[1] == "sessionsrc";
        let content = &cap[2];

        let nbytes = re_bytes.captures(content)
            .and_then(|c| c[1].parse::<u32>().ok())
            .unwrap_or(0);

        let timestamp = re_ts.captures(content)
            .and_then(|c| c[1].parse::<u64>().ok());

        let flags = re_flags.captures(content)
            .map(|c| {
                let raw = c[1].replace("&nbsp;", " ");
                let clean = re_tag.replace_all(&raw, "");
                clean.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .unwrap_or_default();

        let hex_lines: Vec<String> = if let Some(pre_cap) = re_pre.captures(content) {
            let raw = &pre_cap[1];
            let clean = re_tag.replace_all(raw, "");
            let decoded = clean
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", " ")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&#47;", "/");
            decoded.lines()
                .map(|l| l.to_string())
                .filter(|l| !l.trim().is_empty())
                .collect()
        } else {
            Vec::new()
        };

        packets.push(Packet {
            src: is_src,
            bytes: nbytes,
            timestamp,
            flags,
            lines: hex_lines,
        });
    }
    PacketsData {
        src_label,
        dst_label,
        total: packets.len() as u64,
        packets,
    }
}
