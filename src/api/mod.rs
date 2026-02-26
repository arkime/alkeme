mod viewer;
mod cont3xt;
mod parliament;
mod wise;

pub use cont3xt::*;
pub use parliament::*;
pub use wise::*;

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

/// Decode JavaScript string escapes (\xNN hex codes) commonly found in Okta pages
fn decode_js_escapes(raw: &str) -> String {
    let mut decoded = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte as char);
                    }
                }
                Some(other) => { decoded.push('\\'); decoded.push(other); }
                None => { decoded.push('\\'); }
            }
        } else {
            decoded.push(ch);
        }
    }
    decoded
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
    Web,
    Okta,
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => self.client.post(url),
        };
        req = req.header("Content-Type", "application/json").body(json_body.to_string());
        if let Some(ref cookie) = self.arkime_cookie {
            if self.auth_mode != AuthMode::Form && self.auth_mode != AuthMode::Web && self.auth_mode != AuthMode::Okta {
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
        if auth_mode == AuthMode::Form || auth_mode == AuthMode::Web || auth_mode == AuthMode::Okta {
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
        if self.auth_mode == AuthMode::Web {
            return self.web_login().await;
        }
        if self.auth_mode == AuthMode::Okta {
            return self.okta_login().await;
        }
        if self.auth_mode != AuthMode::Form || self.logged_in {
            return Ok(());
        }
        let username = self.username.as_deref().unwrap_or("");
        let password = self.password.as_deref().unwrap_or("");
        let url = format!("{}/api/login", self.base_url);
        let start = Instant::now();
        let resp = self.client.post(&url)
            .form(&[("username", username), ("password", password)])
            .send()
            .await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        log_http(&self.http_log, "POST", &url, Some(format!("username={}&password=***", username)), status, first_byte, start.elapsed().as_millis() as u64, None);
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
        let start2 = Instant::now();
        let resp = self.client.get(&settings_url).send().await?;
        let first_byte2 = start2.elapsed().as_millis() as u64;
        let status2 = resp.status().as_u16();
        if resp.status() == reqwest::StatusCode::FOUND || !resp.status().is_success() {
            log_http(&self.http_log, "GET", &settings_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
            anyhow::bail!("Form login failed: session not established (HTTP {} on settings fetch)", status2);
        }
        self.extract_cookie(&resp);
        log_http(&self.http_log, "GET", &settings_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
        self.logged_in = true;
        Ok(())
    }

    /// Follow redirects manually, logging each hop. Returns final (url, response).
    async fn follow_redirects(&self, initial_url: &str, method: &str) -> Result<(String, reqwest::Response)> {
        let max_redirects = 10;
        let mut url = initial_url.to_string();
        for _ in 0..max_redirects {
            let start = Instant::now();
            let resp = self.client.get(&url).send().await?;
            let first_byte = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            log_http(&self.http_log, method, &url, None, status, first_byte, start.elapsed().as_millis() as u64, None);

            if resp.status().is_redirection() {
                let location = resp.headers().get("location")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if location.is_empty() {
                    anyhow::bail!("Redirect with no Location header from {}", url);
                }
                url = if location.starts_with("http") {
                    location
                } else {
                    // Resolve relative to current URL
                    let parsed = reqwest::Url::parse(&url)?;
                    parsed.join(&location)?.to_string()
                };
            } else {
                return Ok((url, resp));
            }
        }
        anyhow::bail!("Too many redirects (>{}) following {}", max_redirects, initial_url)
    }

    /// Web auth: fetch the login page, parse the HTML form, fill in credentials, and submit.
    /// Supports multi-hop redirects for enterprise SSO (e.g., app → auth system → app).
    async fn web_login(&mut self) -> Result<()> {
        if self.logged_in {
            return Ok(());
        }
        use scraper::{Html, Selector};

        // Step 1: Navigate to base URL, following redirects to find the login page
        let (auth_url, resp) = self.follow_redirects(&self.base_url.clone(), "GET").await?;
        let html_body = resp.text().await?;

        // Step 2: Parse the HTML to find the first <form> with a password field
        let document = Html::parse_document(&html_body);
        let form_sel = Selector::parse("form").unwrap();
        let input_sel = Selector::parse("input").unwrap();

        let form = document.select(&form_sel)
            .find(|f| {
                // Prefer forms that have a password input
                f.select(&input_sel).any(|i| {
                    i.value().attr("type").unwrap_or("").eq_ignore_ascii_case("password")
                })
            })
            .or_else(|| document.select(&form_sel).next())
            .ok_or_else(|| anyhow::anyhow!("Web login: no <form> found on auth page ({})", auth_url))?;

        // Get form action URL
        let action = form.value().attr("action").unwrap_or("");
        let method = form.value().attr("method").unwrap_or("post").to_lowercase();
        if method != "post" {
            anyhow::bail!("Web login: form method is '{}', expected 'post'", method);
        }

        let submit_url = if action.is_empty() {
            // Empty action means submit to the same URL
            auth_url.clone()
        } else if action.starts_with("http") {
            action.to_string()
        } else {
            // Resolve relative/absolute path against the auth page URL
            let parsed = reqwest::Url::parse(&auth_url)?;
            parsed.join(action)?.to_string()
        };

        // Step 3: Display visible text from the page to give context
        let body_sel = Selector::parse("body").unwrap();
        if let Some(body) = document.select(&body_sel).next() {
            let mut page_text = String::new();
            fn collect_text(el: scraper::ElementRef, out: &mut String) {
                let tag = el.value().name();
                if tag == "script" || tag == "style" || tag == "input" {
                    return;
                }
                if matches!(tag, "br" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr") {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                for child in el.children() {
                    if let Some(text) = child.value().as_text() {
                        let t = text.trim();
                        if !t.is_empty() {
                            if !out.is_empty() && !out.ends_with('\n') {
                                out.push(' ');
                            }
                            out.push_str(t);
                        }
                    } else if let Some(child_el) = scraper::ElementRef::wrap(child) {
                        collect_text(child_el, out);
                    }
                }
            }
            collect_text(body, &mut page_text);
            let page_text = page_text.trim();
            if !page_text.is_empty() {
                eprintln!("\n{}\n", page_text);
            }
        }

        // Step 4: Build label map from <label for="id"> elements
        let label_sel = Selector::parse("label").unwrap();
        let mut label_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for label_el in form.select(&label_sel) {
            if let Some(for_id) = label_el.value().attr("for") {
                let text: String = label_el.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    label_map.insert(for_id.to_string(), text);
                }
            }
        }

        // Step 4: Collect all input fields and fill in credentials
        let mut form_data: Vec<(String, String)> = Vec::new();
        let mut found_user = false;
        let mut found_pass = false;

        for input in form.select(&input_sel) {
            let name = input.value().attr("name").unwrap_or("").to_string();
            let input_type = input.value().attr("type").unwrap_or("text").to_lowercase();
            let value = input.value().attr("value").unwrap_or("").to_string();
            let id = input.value().attr("id").unwrap_or("").to_string();
            let field_label = label_map.get(&id)
                .cloned()
                .or_else(|| input.value().attr("placeholder").map(|s| s.to_string()))
                .unwrap_or_else(|| name.clone());

            if name.is_empty() {
                continue;
            }

            match input_type.as_str() {
                "password" => {
                    let pass_value = if let Some(ref p) = self.password {
                        p.clone()
                    } else {
                        let prompt = format!("{}: ", field_label);
                        rpassword::prompt_password(&prompt)?
                    };
                    form_data.push((name, pass_value.clone()));
                    if self.password.is_none() {
                        self.password = Some(pass_value);
                    }
                    found_pass = true;
                }
                "submit" | "button" | "image" | "checkbox" | "radio" => {
                    // For checkbox/radio, include if checked
                    if (input_type == "checkbox" || input_type == "radio")
                        && input.value().attr("checked").is_some() {
                        form_data.push((name, value));
                    }
                }
                "hidden" => {
                    form_data.push((name, value));
                }
                _ => {
                    // text, email, tel, number, etc.
                    if !found_user {
                        let user_value = if let Some(ref u) = self.username {
                            u.clone()
                        } else {
                            eprint!("{}: ", field_label);
                            let mut input_val = String::new();
                            std::io::stdin().read_line(&mut input_val)?;
                            input_val.trim().to_string()
                        };
                        if self.username.is_none() {
                            self.username = Some(user_value.clone());
                        }
                        form_data.push((name, user_value));
                        found_user = true;
                    } else {
                        // Extra field — prompt the user interactively
                        eprint!("{}: ", field_label);
                        let mut input_value = String::new();
                        std::io::stdin().read_line(&mut input_value)?;
                        let input_value = input_value.trim().to_string();
                        form_data.push((name, input_value));
                    }
                }
            }
        }

        if !found_user || !found_pass {
            anyhow::bail!("Web login: could not find username/password fields in form (found_user={}, found_pass={})", found_user, found_pass);
        }

        // Step 4: Submit the form
        let start = Instant::now();
        let resp = self.client.post(&submit_url)
            .form(&form_data)
            .send()
            .await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let log_user = self.username.as_deref().unwrap_or("***");
        log_http(&self.http_log, "POST", &submit_url, Some(format!("username={}&password=***", log_user)), status, first_byte, start.elapsed().as_millis() as u64, None);

        // Step 5: Follow post-login redirects (auth system → app callback → app)
        if resp.status().is_redirection() {
            let location = resp.headers().get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            if location.is_empty() {
                anyhow::bail!("Web login: POST redirect with no Location header");
            }

            // Check if redirected back to login page (auth failure)
            if location.contains("/auth") || location.contains("/login") {
                let parsed_loc = reqwest::Url::parse(&location)
                    .or_else(|_| reqwest::Url::parse(&submit_url).and_then(|u| u.join(&location)));
                let loc_path = parsed_loc.map(|u| u.path().to_string()).unwrap_or(location.clone());
                let auth_parsed = reqwest::Url::parse(&auth_url).ok();
                let auth_path = auth_parsed.as_ref().map(|u| u.path()).unwrap_or("");
                if loc_path == auth_path {
                    anyhow::bail!("Web login failed: redirected back to login page (invalid credentials?)");
                }
            }

            // Follow the redirect chain back to the app
            let next_url = if location.starts_with("http") {
                location
            } else {
                let parsed = reqwest::Url::parse(&submit_url)?;
                parsed.join(&location)?.to_string()
            };
            self.follow_redirects(&next_url, "GET").await?;
        } else if !resp.status().is_success() {
            anyhow::bail!("Web login failed: HTTP {}", status);
        }

        // Step 6: Verify session is established
        let verify_url = format!("{}/api/appversion", self.base_url);
        let start2 = Instant::now();
        let resp = self.client.get(&verify_url).send().await?;
        let first_byte2 = start2.elapsed().as_millis() as u64;
        let status2 = resp.status().as_u16();
        if resp.status().is_redirection() || !resp.status().is_success() {
            log_http(&self.http_log, "GET", &verify_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
            anyhow::bail!("Web login failed: session not established (HTTP {} on appversion fetch)", status2);
        }
        self.extract_cookie(&resp);
        log_http(&self.http_log, "GET", &verify_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
        self.logged_in = true;
        Ok(())
    }

    /// Okta auth: fetch the Okta login page, extract stateToken/baseUrl from JS,
    /// authenticate via Okta's /api/v1/authn API, then follow session redirect.
    async fn okta_login(&mut self) -> Result<()> {
        if self.logged_in {
            return Ok(());
        }

        // Step 1: Navigate to app URL, following redirects to Okta login page
        let (auth_url, resp) = self.follow_redirects(&self.base_url.clone(), "GET").await?;
        eprintln!("Okta login page: {}", auth_url);
        let html_body = resp.text().await?;

        // Step 2: Extract stateToken and config from page
        let raw_state_token = regex::Regex::new(r"var stateToken = '([^']+)'")
            .unwrap()
            .captures(&html_body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow::anyhow!("Okta login: could not find stateToken in page ({})", auth_url))?;
        let state_token = decode_js_escapes(&raw_state_token);
        eprintln!("stateToken length: {} (raw: {}), starts: {}...", state_token.len(), raw_state_token.len(), &state_token[..state_token.len().min(40)]);

        // Extract modelDataBag JSON for baseUrl and labels
        let model_data = regex::Regex::new(r"var modelDataBag = '([^']+)'")
            .unwrap()
            .captures(&html_body)
            .and_then(|c| c.get(1))
            .map(|m| decode_js_escapes(m.as_str()));

        let (okta_base_url, username_label, password_label, app_name, brand_name) = if let Some(ref json_str) = model_data {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                let base = data["baseUrl"].as_str().unwrap_or("").to_string();
                let settings = &data["orgLoginPageSettings"];
                let ulabel = settings["usernameLabel"].as_str().unwrap_or("Username").to_string();
                let plabel = settings["passwordLabel"].as_str().unwrap_or("Password").to_string();
                let app = data["appInstanceName"].as_str().unwrap_or("").to_string();
                let brand = data["brandName"].as_str().unwrap_or("").to_string();
                (base, ulabel, plabel, app, brand)
            } else {
                (String::new(), "Username".to_string(), "Password".to_string(), String::new(), String::new())
            }
        } else {
            (String::new(), "Username".to_string(), "Password".to_string(), String::new(), String::new())
        };

        // Determine the Okta base URL
        let okta_base = if !okta_base_url.is_empty() {
            okta_base_url
        } else {
            // Fall back to the auth page URL's origin
            let parsed = reqwest::Url::parse(&auth_url)?;
            format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""))
        };

        // Display page context
        if !brand_name.is_empty() || !app_name.is_empty() {
            eprintln!();
            if !brand_name.is_empty() {
                eprintln!("{}", brand_name);
            }
            if !app_name.is_empty() {
                eprintln!("Connecting to {}", app_name);
            }
            eprintln!();
        }

        // Step 3: Prompt for credentials using Okta's labels
        let username = if let Some(ref u) = self.username {
            u.clone()
        } else {
            eprint!("{}: ", username_label);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let val = input.trim().to_string();
            self.username = Some(val.clone());
            val
        };

        let password = if let Some(ref p) = self.password {
            p.clone()
        } else {
            let val = rpassword::prompt_password(format!("{}: ", password_label))?;
            self.password = Some(val.clone());
            val
        };

        // Step 4: POST to Okta authn API
        let authn_url = format!("{}/api/v1/authn", okta_base);
        eprintln!("Authenticating as '{}' via {}", username, authn_url);
        let authn_body = serde_json::json!({
            "username": username,
            "password": password,
            "stateToken": state_token,
        });
        let start = Instant::now();
        let resp = self.client.post(&authn_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(authn_body.to_string())
            .send()
            .await?;
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        let last_byte = start.elapsed().as_millis() as u64;
        log_http(&self.http_log, "POST", &authn_url, Some(format!("username={}&password=***", username)), status, first_byte, last_byte, Some(&body[..body.len().min(200)]));

        if status == 401 {
            let err_msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["errorSummary"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| body[..body.len().min(200)].to_string());
            anyhow::bail!("Okta login failed: {}", err_msg);
        }
        if status != 200 {
            let err_msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["errorSummary"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| body[..body.len().min(200)].to_string());
            anyhow::bail!("Okta login failed: HTTP {} — {}", status, err_msg);
        }

        let authn_resp: serde_json::Value = serde_json::from_str(&body)?;
        let authn_status = authn_resp["status"].as_str().unwrap_or("");

        // Step 5: Handle MFA if required
        let session_token = match authn_status {
            "SUCCESS" => {
                authn_resp["sessionToken"].as_str()
                    .ok_or_else(|| anyhow::anyhow!("Okta login: SUCCESS but no sessionToken"))?
                    .to_string()
            }
            "MFA_REQUIRED" => {
                self.okta_handle_mfa(&authn_resp, &okta_base).await?
            }
            other => {
                anyhow::bail!("Okta login: unexpected status '{}'. May require additional setup.", other);
            }
        };

        // Step 6: Exchange session token for session cookie
        // Follow the fromURI redirect with the session token
        let from_uri = {
            use scraper::{Html, Selector};
            let doc = Html::parse_document(&html_body);
            let sel = Selector::parse("input#fromURI").unwrap();
            doc.select(&sel).next()
                .and_then(|el| el.value().attr("value"))
                .map(|v| v.to_string())
        };

        let redirect_url = if let Some(uri) = from_uri {
            format!("{}/login/sessionCookieRedirect?checkAccountSetupComplete=true&token={}&redirectUrl={}", okta_base, session_token, urlencoding::encode(&uri))
        } else {
            format!("{}/login/sessionCookieRedirect?checkAccountSetupComplete=true&token={}&redirectUrl={}", okta_base, session_token, urlencoding::encode(&self.base_url))
        };

        // Follow the redirect chain back to the app
        self.follow_redirects(&redirect_url, "GET").await?;

        // Step 7: Verify session is established
        let verify_url = format!("{}/api/appversion", self.base_url);
        let start2 = Instant::now();
        let resp = self.client.get(&verify_url).send().await?;
        let first_byte2 = start2.elapsed().as_millis() as u64;
        let status2 = resp.status().as_u16();
        if resp.status().is_redirection() || !resp.status().is_success() {
            log_http(&self.http_log, "GET", &verify_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
            anyhow::bail!("Okta login failed: session not established after redirect (HTTP {})", status2);
        }
        self.extract_cookie(&resp);
        log_http(&self.http_log, "GET", &verify_url, None, status2, first_byte2, start2.elapsed().as_millis() as u64, None);
        self.logged_in = true;
        Ok(())
    }

    /// Handle Okta MFA challenge — supports push notification and TOTP code
    async fn okta_handle_mfa(&self, authn_resp: &serde_json::Value, _okta_base: &str) -> Result<String> {
        let state_token = authn_resp["stateToken"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Okta MFA: no stateToken"))?;

        let factors = authn_resp["_embedded"]["factors"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Okta MFA: no factors found"))?;

        // Display available factors
        eprintln!("\nMulti-factor authentication required.");

        // Prefer Okta Verify push, then TOTP, then others
        let push_factor = factors.iter().find(|f| f["factorType"].as_str() == Some("push"));
        let totp_factor = factors.iter().find(|f| {
            matches!(f["factorType"].as_str(), Some("token:software:totp") | Some("token:hotp"))
        });

        if let Some(factor) = push_factor {
            // Send push notification
            let verify_url = factor["_links"]["verify"]["href"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Okta MFA: no verify URL for push factor"))?;

            eprintln!("Sending push notification to Okta Verify...");
            let body = serde_json::json!({"stateToken": state_token});
            let start = Instant::now();
            let resp = self.client.post(verify_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(body.to_string())
                .send()
                .await?;
            let status = resp.status().as_u16();
            let resp_body = resp.text().await?;
            log_http(&self.http_log, "POST", verify_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, None);

            let mut poll_resp: serde_json::Value = serde_json::from_str(&resp_body)?;

            // Poll for push approval (up to 60 seconds)
            for i in 0..30 {
                let poll_status = poll_resp["status"].as_str().unwrap_or("");
                match poll_status {
                    "SUCCESS" => {
                        return poll_resp["sessionToken"].as_str()
                            .map(|s| s.to_string())
                            .ok_or_else(|| anyhow::anyhow!("Okta MFA: SUCCESS but no sessionToken"));
                    }
                    "MFA_CHALLENGE" => {
                        let result = poll_resp["factorResult"].as_str().unwrap_or("");
                        match result {
                            "WAITING" => {
                                if i == 0 { eprintln!("Waiting for approval..."); }
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                // Poll again
                                let poll_url = poll_resp["_links"]["next"]["href"].as_str()
                                    .unwrap_or(verify_url);
                                let body = serde_json::json!({"stateToken": state_token});
                                let start = Instant::now();
                                let resp = self.client.post(poll_url)
                                    .header("Content-Type", "application/json")
                                    .header("Accept", "application/json")
                                    .body(body.to_string())
                                    .send()
                                    .await?;
                                let poll_status_code = resp.status().as_u16();
                                let resp_body = resp.text().await?;
                                log_http(&self.http_log, "POST", poll_url, None, poll_status_code, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, None);
                                poll_resp = serde_json::from_str(&resp_body)?;
                            }
                            "REJECTED" => {
                                anyhow::bail!("Okta MFA: push notification was rejected");
                            }
                            "TIMEOUT" => {
                                anyhow::bail!("Okta MFA: push notification timed out");
                            }
                            other => {
                                anyhow::bail!("Okta MFA: unexpected factorResult '{}'", other);
                            }
                        }
                    }
                    other => {
                        anyhow::bail!("Okta MFA: unexpected status '{}' during push", other);
                    }
                }
            }
            anyhow::bail!("Okta MFA: push notification timed out after 60 seconds");

        } else if let Some(factor) = totp_factor {
            // Prompt for TOTP code
            let verify_url = factor["_links"]["verify"]["href"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Okta MFA: no verify URL for TOTP factor"))?;
            let provider = factor["provider"].as_str().unwrap_or("authenticator");

            let code = rpassword::prompt_password(format!("Enter code from {}: ", provider))?;

            let body = serde_json::json!({
                "stateToken": state_token,
                "passCode": code,
            });
            let start = Instant::now();
            let resp = self.client.post(verify_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(body.to_string())
                .send()
                .await?;
            let status = resp.status().as_u16();
            let resp_body = resp.text().await?;
            log_http(&self.http_log, "POST", verify_url, Some("passCode=***".into()), status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, None);

            let mfa_resp: serde_json::Value = serde_json::from_str(&resp_body)?;
            if mfa_resp["status"].as_str() == Some("SUCCESS") {
                return mfa_resp["sessionToken"].as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow::anyhow!("Okta MFA: SUCCESS but no sessionToken"));
            }
            anyhow::bail!("Okta MFA: verification failed ({})", mfa_resp["status"].as_str().unwrap_or("unknown"));
        } else {
            // List available factor types for the user
            let factor_types: Vec<&str> = factors.iter()
                .filter_map(|f| f["factorType"].as_str())
                .collect();
            anyhow::bail!("Okta MFA: no supported factor type. Available: {:?}", factor_types);
        }
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => return Ok(()), // already captured during login
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => self.client.post(url),
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => self.client.put(url),
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
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => self.client.delete(url),
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
