mod viewer;
mod cont3xt;
mod parliament;
mod wise;
mod types;
mod auth;

pub use cont3xt::*;
pub use parliament::*;
pub use wise::*;
pub use types::*;


use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use aes_gcm::aead::generic_array::GenericArray;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use rand::RngCore;

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
    fn check_status(status: u16, body: &str, http_log: &HttpLog, method: &str, url: &str, post_data: Option<String>, first_byte: u64, last_byte: u64) -> Result<()> {
        if status >= 400 {
            log_http(http_log, method, url, post_data, status, first_byte, last_byte, Some(body));
            anyhow::bail!("HTTP {} (see debug log [D] for details)", status);
        }
        Ok(())
    }

    async fn finish_response(&self, start: Instant, method: &str, url: &str, post_data: Option<String>, resp: reqwest::Response) -> Result<String> {
        let first_byte = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        let last_byte = start.elapsed().as_millis() as u64;
        Self::check_status(status, &body, &self.http_log, method, url, post_data.clone(), first_byte, last_byte)?;
        log_http(&self.http_log, method, url, post_data, status, first_byte, last_byte, Some(&body));
        Ok(body)
    }

    fn digest_auth_header(resp: &reqwest::Response, url: &str, username: &str, password: &str, is_post: bool) -> Result<String> {
        let www_auth = resp.headers().get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?.to_string();
        let parsed_url = reqwest::Url::parse(url)?;
        let uri = if let Some(q) = parsed_url.query() {
            format!("{}?{}", parsed_url.path(), q)
        } else { parsed_url.path().to_string() };
        let mut prompt = digest_auth::parse(&www_auth)?;
        if is_post {
            let context = digest_auth::AuthContext::new_post::<_, _, _, &[u8]>(username, password, &uri, None);
            Ok(prompt.respond(&context)?.to_header_string())
        } else {
            let context = digest_auth::AuthContext::new(username, password, &uri);
            Ok(prompt.respond(&context)?.to_header_string())
        }
    }

    pub async fn fetch_url(&self, url: &str) -> Result<String> {
        let start = Instant::now();
        let username = match self.username.as_deref() {
            Some(u) => u,
            None => {
                let resp = self.client.get(url).send().await?;
                return self.finish_response(start, "GET", url, None, resp).await;
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.get(url).send().await?;
                self.finish_response(start, "GET", url, None, resp).await
            }
            AuthMode::Basic => {
                let resp = self.client.get(url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await?;
                self.finish_response(start, "GET", url, None, resp).await
            }
            AuthMode::Digest => {
                let resp = self.client.get(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return self.finish_response(start, "GET", url, None, resp).await;
                }
                let auth_header = Self::digest_auth_header(&resp, url, username, password, false)?;
                let resp = self.client.get(url).header("Authorization", auth_header).send().await?;
                self.finish_response(start, "GET", url, None, resp).await
            }
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
                let resp = self.client.get(url).send().await?;
                self.finish_response(start, "GET", url, None, resp).await
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
                return self.finish_response(start, "POST", url, post_data, resp).await;
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        match self.auth_mode {
            AuthMode::None => {
                let resp = self.client.post(url).form(form).send().await?;
                self.finish_response(start, "POST", url, post_data, resp).await
            }
            AuthMode::Basic => {
                let mut req = self.client.post(url)
                    .basic_auth(username, Some(password))
                    .form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                self.finish_response(start, "POST", url, post_data, resp).await
            }
            AuthMode::Digest => {
                let resp = self.client.post(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return self.finish_response(start, "POST", url, post_data, resp).await;
                }
                let auth_header = Self::digest_auth_header(&resp, url, username, password, true)?;
                let mut req = self.client.post(url).header("Authorization", auth_header).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                self.finish_response(start, "POST", url, post_data, resp).await
            }
            AuthMode::Form | AuthMode::Web | AuthMode::Okta => {
                let mut req = self.client.post(url).form(form);
                if let Some(ref cookie) = self.arkime_cookie {
                    req = req.header(self.cookie_header_name, cookie.as_str());
                }
                let resp = req.send().await?;
                self.finish_response(start, "POST", url, post_data, resp).await
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
                return self.finish_response(start, "POST", url, post_data, resp).await;
            }
        };
        let password = self.password.as_deref().unwrap_or("");

        let mut req = match self.auth_mode {
            AuthMode::None => self.client.post(url),
            AuthMode::Basic => self.client.post(url).basic_auth(username, Some(password)),
            AuthMode::Digest => {
                let resp = self.client.post(url).send().await?;
                if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
                    return self.finish_response(start, "POST", url, post_data, resp).await;
                }
                let auth_header = Self::digest_auth_header(&resp, url, username, password, true)?;
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
        self.finish_response(start, "POST", url, post_data, resp).await
    }
}

fn digest_auth_header(resp: &reqwest::Response, url: &str, username: &str, password: &str, method: &str) -> Result<String> {
    let www_auth = resp.headers().get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("No WWW-Authenticate header"))?.to_string();
    let parsed_url = reqwest::Url::parse(url)?;
    let uri = if let Some(q) = parsed_url.query() {
        format!("{}?{}", parsed_url.path(), q)
    } else { parsed_url.path().to_string() };
    let http_method = match method {
        "POST" => digest_auth::HttpMethod::POST,
        "PUT" => digest_auth::HttpMethod::PUT,
        "DELETE" => digest_auth::HttpMethod::DELETE,
        _ => digest_auth::HttpMethod::GET,
    };
    let context = digest_auth::AuthContext::new_with_method::<_, _, _, &[u8]>(username, password, &uri, None, http_method);
    let mut prompt = digest_auth::parse(&www_auth)?;
    Ok(prompt.respond(&context)?.to_header_string())
}

const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

fn encrypt_data(plaintext: &[u8], password: &str) -> Vec<u8> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = GenericArray::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).expect("encryption failed");
    // Format: salt || nonce || ciphertext
    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

fn decrypt_data(data: &[u8], password: &str) -> Option<Vec<u8>> {
    if data.len() < SALT_LEN + NONCE_LEN + 16 { // 16 = GCM tag
        return None;
    }
    let salt = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];
    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = GenericArray::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).ok()
}

/// Load a cookie store from a file, optionally decrypting with a password.
/// Returns Ok((store, username)) on success, Err if the file exists but decryption fails.
/// Returns Ok((empty store, None)) if the file doesn't exist yet.
pub fn load_cookie_store(path: &str, password: Option<&str>) -> Result<(reqwest_cookie_store::CookieStore, Option<String>)> {
    let data = match std::fs::read(path) {
        Ok(d) if d.is_empty() => return Ok((reqwest_cookie_store::CookieStore::default(), None)),
        Ok(d) => d,
        Err(_) => return Ok((reqwest_cookie_store::CookieStore::default(), None)),
    };
    let json = if let Some(pass) = password {
        match decrypt_data(&data, pass) {
            Some(d) => d,
            None => anyhow::bail!("Wrong cookie jar password"),
        }
    } else {
        data
    };
    // Try new envelope format: {"username":"...","cookies":[...]}
    if let Ok(wrapper) = serde_json::from_slice::<serde_json::Value>(&json) {
        if wrapper.get("cookies").is_some() {
            let username = wrapper.get("username").and_then(|v| v.as_str()).map(String::from);
            let cookies_bytes = serde_json::to_vec(wrapper.get("cookies").unwrap()).unwrap_or_default();
            let reader = std::io::BufReader::new(cookies_bytes.as_slice());
            return Ok((cookie_store::serde::json::load_all(reader).unwrap_or_default(), username));
        }
    }
    // Fall back to legacy format (raw cookie array)
    let reader = std::io::BufReader::new(json.as_slice());
    Ok((cookie_store::serde::json::load_all(reader).unwrap_or_default(), None))
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
    pub(crate) cookie_store: Option<Arc<reqwest_cookie_store::CookieStoreMutex>>,
}

impl ArkimeClient {
    pub fn new(base_url: &str, auth_mode: AuthMode, username: Option<String>, password: Option<String>,
               cookie_store: Option<Arc<reqwest_cookie_store::CookieStoreMutex>>) -> Self {
        let mut builder = Client::builder()
            .danger_accept_invalid_certs(true);
        if auth_mode == AuthMode::Form || auth_mode == AuthMode::Web || auth_mode == AuthMode::Okta {
            if let Some(ref store) = cookie_store {
                builder = builder.cookie_provider(Arc::clone(store)).redirect(reqwest::redirect::Policy::none());
            } else {
                builder = builder.cookie_store(true).redirect(reqwest::redirect::Policy::none());
            }
        }
        if auth_mode == AuthMode::Okta {
            builder = builder.user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36");
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
            cookie_store,
        }
    }

    pub fn http_log(&self) -> HttpLog {
        self.http_log.clone()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn set_credentials(&mut self, username: Option<String>, password: Option<String>) {
        self.username = username;
        self.password = password;
    }

    /// Save cookies to a file if a cookie store is configured
    pub fn save_cookies(&self, path: &str, jar_password: Option<&str>) {
        if let Some(ref store) = self.cookie_store {
            use std::os::unix::fs::OpenOptionsExt;
            let store = store.lock().unwrap();
            let mut cookie_buf = Vec::new();
            if cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut cookie_buf).is_err() {
                return;
            }
            let cookies_value: serde_json::Value = serde_json::from_slice(&cookie_buf).unwrap_or_default();
            let wrapper = serde_json::json!({
                "username": self.username,
                "cookies": cookies_value,
            });
            let json_buf = serde_json::to_vec(&wrapper).unwrap_or_default();
            let data = if let Some(pass) = jar_password {
                encrypt_data(&json_buf, pass)
            } else {
                json_buf
            };
            let file = std::fs::OpenOptions::new()
                .write(true).create(true).truncate(true).mode(0o600)
                .open(path);
            if let Ok(mut file) = file {
                use std::io::Write;
                file.write_all(&data).ok();
            }
        }
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

    /// Create a new client pointing to a different URL but sharing the same
    /// reqwest::Client (and its cookie jar). Skips login — reuses existing session.
    pub fn clone_with_url(&self, base_url: &str) -> Self {
        Self {
            client: self.client.clone(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_mode: self.auth_mode,
            username: self.username.clone(),
            password: self.password.clone(),
            logged_in: self.logged_in,
            arkime_cookie: None,
            cookie_header_name: self.cookie_header_name,
            http_log: new_http_log(),
            cookie_store: self.cookie_store.clone(),
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

    /// Check if existing session cookies are valid (200 on /api/user/settings).
    /// Returns Ok(true) if valid, Ok(false) if not. Does NOT fall back to login.
    pub async fn check_session(&mut self) -> Result<bool> {
        let url = format!("{}/api/user/settings", self.base_url);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            self.extract_cookie(&resp);
            self.logged_in = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Try to use existing session cookies for a new URL. If the session
    /// is valid (200 on /api/user/settings), extract the CSRF cookie.
    /// If not (redirect/4xx), fall back to full login().
    pub async fn ensure_session(&mut self) -> Result<()> {
        if self.auth_mode == AuthMode::None || self.auth_mode == AuthMode::Basic || self.auth_mode == AuthMode::Digest {
            return Ok(()); // these don't use session cookies
        }
        if let Ok(true) = self.check_session().await {
            return Ok(());
        }
        // Session not valid for this host — do full login
        self.logged_in = false;
        self.login().await
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

    pub async fn get_eshealth(&self) -> Result<Value> {
        let url = format!("{}/api/eshealth", self.base_url);
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
                let auth_header = digest_auth_header(&resp, &url, username, password, "GET")?;
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
                let auth_header = digest_auth_header(&resp, url, username, password, "POST")?;
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
            anyhow::bail!("HTTP {} (see debug log [D] for details)", status);
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
                let auth_header = digest_auth_header(&resp, url, username, password, "PUT")?;
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
            anyhow::bail!("HTTP {} (see debug log [D] for details)", status);
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
                let auth_header = digest_auth_header(&resp, url, username, password, "DELETE")?;
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
            anyhow::bail!("HTTP {} (see debug log [D] for details)", status);
        }
        let result = resp.json().await?;
        log_http(&self.http_log, "DELETE", url, None, status, first_byte, start.elapsed().as_millis() as u64, None);
        Ok(result)
    }
}
