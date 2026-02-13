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
    #[serde(default, rename = "bsqErr")]
    pub bsq_err: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
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

pub struct FetchClient {
    client: Client,
    auth_mode: AuthMode,
    username: Option<String>,
    password: Option<String>,
    arkime_cookie: Option<String>,
}

impl FetchClient {
    pub async fn fetch_url(&self, url: &str) -> Result<String> {
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
                Ok(resp.text().await?)
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return Ok(resp.text().await?);
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
                Ok(resp.text().await?)
            }
            AuthMode::Form => {
                let mut req = self.client.get(url);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header("Cookie", cookie.as_str());
                }
                let resp = req.send().await?;
                Ok(resp.text().await?)
            }
        }
    }
}

pub struct ArkimeClient {
    client: Client,
    base_url: String,
    auth_mode: AuthMode,
    username: Option<String>,
    password: Option<String>,
    logged_in: bool,
    arkime_cookie: Option<String>,
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
        }
    }

    pub fn clone_for_fetch(&self) -> FetchClient {
        FetchClient {
            client: self.client.clone(),
            auth_mode: self.auth_mode,
            username: self.username.clone(),
            password: self.password.clone(),
            arkime_cookie: self.arkime_cookie.clone(),
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
        if resp.status() != reqwest::StatusCode::FOUND && !resp.status().is_success() {
            anyhow::bail!("Form login failed: HTTP {}", resp.status());
        }
        // Fetch /api/user/settings to get the ARKIME-COOKIE (needed for POST CSRF protection)
        let settings_url = format!("{}/api/user/settings", self.base_url);
        let resp = self.client.get(&settings_url).send().await?;
        for cookie_val in resp.headers().get_all("set-cookie") {
            if let Ok(s) = cookie_val.to_str()
                && s.starts_with("ARKIME-COOKIE=")
                    && let Some(val) = s.strip_prefix("ARKIME-COOKIE=") {
                        let val = val.split(';').next().unwrap_or(val);
                        self.arkime_cookie = Some(urlencoding::decode(val).unwrap_or_default().into_owned());
                    }
        }
        self.logged_in = true;
        Ok(())
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
            AuthMode::Form => {
                // Cookie-based auth, login() must be called first
                let resp = self.client.get(url).send().await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", resp.status());
                }
                Ok(resp.text().await?)
            }
        }
    }

    async fn authenticated_post(&self, url: &str, form: &[(&str, &str)]) -> Result<String> {
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.post(url).form(form).send().await?;
                return Ok(resp.text().await?);
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.post(url).form(form).send().await?;
                Ok(resp.text().await?)
            }
            AuthMode::Basic => {
                let resp = self.client.post(url)
                    .basic_auth(username, Some(password))
                    .form(form)
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed", resp.status());
                }
                Ok(resp.text().await?)
            }
            AuthMode::Digest => {
                let resp = self.client.post(url).form(form).send().await?;
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
                let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
                let mut prompt = digest_auth::parse(&www_auth)?;
                let auth_header = prompt.respond(&context)?.to_header_string();

                let resp = self.client
                    .post(url)
                    .header("Authorization", auth_header)
                    .form(form)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed", resp.status());
                }

                Ok(resp.text().await?)
            }
            AuthMode::Form => {
                let mut req = self.client.post(url).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header("x-arkime-cookie", cookie);
                }
                let resp = req.send().await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", resp.status());
                }
                Ok(resp.text().await?)
            }
        }
    }

    async fn authenticated_get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                return Ok(resp.bytes().await?.to_vec());
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                Ok(resp.bytes().await?.to_vec())
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed", resp.status());
                }
                Ok(resp.bytes().await?.to_vec())
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return Ok(resp.bytes().await?.to_vec());
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

                Ok(resp.bytes().await?.to_vec())
            }
            AuthMode::Form => {
                let resp = self.client.get(url).send().await?;
                if !resp.status().is_success() {
                    anyhow::bail!("HTTP {}: Authentication failed (session expired?)", resp.status());
                }
                Ok(resp.bytes().await?.to_vec())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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

    pub async fn get_stats(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/stats?sortField={}&desc={}",
            self.base_url, urlencoding::encode(sort_field), dir
        );
        if !filter.is_empty() {
            url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
        }
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn get_esstats(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/esstats?sortField={}&desc={}",
            self.base_url, urlencoding::encode(sort_field), dir
        );
        if !filter.is_empty() {
            url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
        }
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn get_esindices(&self, filter: &str, sort_field: &str, sort_desc: bool) -> Result<Value> {
        let dir = if sort_desc { "desc" } else { "asc" };
        let mut url = format!(
            "{}/api/esindices?sortField={}&desc={}",
            self.base_url, urlencoding::encode(sort_field), dir
        );
        if !filter.is_empty() {
            url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
        }
        let body = self.authenticated_get(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn get_user(&self) -> Result<Value> {
        let url = format!("{}/api/user", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let user: Value = serde_json::from_str(&body)?;
        Ok(user)
    }

    pub async fn get_fields(&self) -> Result<(Vec<ArkimeField>, HashMap<String, String>, HashMap<String, String>, HashMap<String, String>)> {
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

    pub async fn download_session_pcap(&self, node: &str, id: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/api/session/{}/{}.pcap?date=-1",
            self.base_url, urlencoding::encode(node), urlencoding::encode(id)
        );
        self.authenticated_get_bytes(&url).await
    }

    pub async fn download_sessions_pcap(&self, expression: &str, date: &str) -> Result<Vec<u8>> {
        let mut url = format!("{}/api/sessions.pcap?date={}", self.base_url, urlencoding::encode(date));
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }
        self.authenticated_get_bytes(&url).await
    }

    pub async fn download_sessions_pcap_ids(&self, ids: &[String]) -> Result<Vec<u8>> {
        let ids_str = ids.join(",");
        let url = format!("{}/api/sessions.pcap?date=-1&ids={}", self.base_url, urlencoding::encode(&ids_str));
        self.authenticated_get_bytes(&url).await
    }

    pub async fn export_sessions_csv(&self, expression: &str, date: &str, fields: &[String]) -> Result<Vec<u8>> {
        let fields_str = fields.join(",");
        let mut url = format!("{}/api/sessions/csv?date={}&fields={}", self.base_url, urlencoding::encode(date), urlencoding::encode(&fields_str));
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }
        self.authenticated_get_bytes(&url).await
    }

    pub async fn export_sessions_csv_ids(&self, ids: &[String], fields: &[String]) -> Result<Vec<u8>> {
        let fields_str = fields.join(",");
        let ids_str = ids.join(",");
        let url = format!("{}/api/sessions/csv?date=-1&fields={}&ids={}", self.base_url, urlencoding::encode(&fields_str), urlencoding::encode(&ids_str));
        self.authenticated_get_bytes(&url).await
    }

    pub async fn add_session_tags(&self, id: &str, tags: &str) -> Result<String> {
        let url = format!("{}/api/sessions/addtags", self.base_url);
        self.authenticated_post(&url, &[("tags", tags), ("ids", id)]).await
    }

    pub async fn add_sessions_tags(&self, expression: &str, date: &str, tags: &str) -> Result<String> {
        let mut url = format!("{}/api/sessions/addtags?date={}", self.base_url, urlencoding::encode(date));
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }
        self.authenticated_post(&url, &[("tags", tags)]).await
    }

    pub async fn remove_session_tags(&self, id: &str, tags: &str) -> Result<String> {
        let url = format!("{}/api/sessions/removetags", self.base_url);
        self.authenticated_post(&url, &[("tags", tags), ("ids", id)]).await
    }

    pub async fn remove_sessions_tags(&self, expression: &str, date: &str, tags: &str) -> Result<String> {
        let mut url = format!("{}/api/sessions/removetags?date={}", self.base_url, urlencoding::encode(date));
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }
        self.authenticated_post(&url, &[("tags", tags)]).await
    }

    pub async fn get_summary(&self, field: &str, expression: &str, date: &str) -> Result<Vec<SummaryItem>> {
        let mut url = format!("{}/api/sessions/summary?date={}", self.base_url, urlencoding::encode(date));
        if !expression.is_empty() {
            url.push_str(&format!("&expression={}", urlencoding::encode(expression)));
        }
        let body = self.authenticated_post(&url, &[("fields", field)]).await?;
        let arr: Vec<Value> = serde_json::from_str(&body)?;
        // The response is a JSON array: [phase1_stats, {field, data: [...]}, {}]
        // We want the second element's data array
        if arr.len() >= 2
            && let Some(data) = arr[1].get("data") {
                let items: Vec<SummaryItem> = serde_json::from_value(data.clone())?;
                return Ok(items);
            }
        Ok(Vec::new())
    }

    pub fn packets_url(&self, node: &str, id: &str, raw: bool) -> String {
        format!("{}/api/session/{}/{}/packets?base=hex&ts=true&packets=10000&showFrames={}",
            self.base_url, urlencoding::encode(node), urlencoding::encode(id), raw)
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