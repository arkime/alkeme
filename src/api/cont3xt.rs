use super::*;

use anyhow::Result;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// A link within a link group
#[derive(Clone)]
pub struct Cont3xtLink {
    pub name: String,
    pub url: String,
    pub itypes: Vec<String>,
    pub info: String,
}

/// A link group from /api/linkGroup
#[derive(Clone)]
pub struct Cont3xtLinkGroup {
    pub name: String,
    pub links: Vec<Cont3xtLink>,
}

impl ArkimeClient {
    pub async fn c3_get_integrations(&self) -> Result<Value> {
        let url = format!("{}/api/integration", self.base_url);
        let body = self.authenticated_get(&url).await?;
        let val: Value = serde_json::from_str(&body)?;
        Ok(val)
    }

    pub fn cont3xt_search_url(&self) -> String {
        format!("{}/api/integration/search", self.base_url)
    }

    /// Fetch cont3xt views (saved integration sets)
    pub async fn c3_get_views(&self) -> Result<Vec<Cont3xtView>> {
        let url = format!("{}/api/views", self.base_url);
        let body = self.authenticated_get_with_cookie(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        let mut views = Vec::new();
        if let Some(data) = parsed.get("views").and_then(|d| d.as_array()) {
            let current_user = self.username.as_deref().unwrap_or("");
            for item in data {
                let id = item.get("_id").or_else(|| item.get("id"))
                    .and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let creator = item.get("creator").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let integrations: Vec<String> = item.get("integrations")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let editable = item.get("_editable").and_then(|v| v.as_bool()).unwrap_or(false)
                    || creator == current_user;
                views.push(Cont3xtView { id, name, integrations, creator, editable });
            }
        }
        Ok(views)
    }

    /// Create a cont3xt view (saved integration set)
    pub async fn c3_create_view(&self, name: &str, integrations: &[String]) -> Result<Value> {
        let url = format!("{}/api/view", self.base_url);
        let body = serde_json::json!({
            "name": name,
            "integrations": integrations,
        });
        self.authenticated_post_json(&url, &body).await
    }

    /// Delete a cont3xt view
    pub async fn c3_delete_view(&self, id: &str) -> Result<Value> {
        let url = format!("{}/api/view/{}", self.base_url, urlencoding::encode(id));
        self.authenticated_delete(&url).await
    }

    /// Fetch cont3xt integration stats
    pub async fn c3_get_stats(&self) -> Result<Value> {
        let url = format!("{}/api/integration/stats", self.base_url);
        let body = self.authenticated_get_with_cookie(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn c3_get_link_groups(&self) -> Result<Vec<Cont3xtLinkGroup>> {
        let url = format!("{}/api/linkGroup", self.base_url);
        let body = self.authenticated_get_with_cookie(&url).await?;
        let parsed: Value = serde_json::from_str(&body)?;
        let mut groups = Vec::new();
        if let Some(arr) = parsed.get("linkGroups").and_then(|v| v.as_array()) {
            for g in arr {
                let name = g.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                let links_arr = g.get("links").and_then(|l| l.as_array());
                let mut links = Vec::new();
                if let Some(larr) = links_arr {
                    for l in larr {
                        let lname = l.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                        let lurl = l.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
                        let itypes: Vec<String> = l.get("itypes")
                            .and_then(|i| i.as_array())
                            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        if lname == "----------" { continue; } // skip separators
                        let info = l.get("infoField").and_then(|n| n.as_str()).unwrap_or("").to_string();
                        links.push(Cont3xtLink { name: lname, url: lurl, itypes, info });
                    }
                }
                if !links.is_empty() {
                    groups.push(Cont3xtLinkGroup { name, links });
                }
            }
        }
        Ok(groups)
    }
}

impl FetchClient {
    /// Like fetch_post_json but streams the response line by line, pushing parsed results
    /// into a shared vec as they arrive. Returns (total, itype) when finished.
    pub async fn fetch_post_json_streaming(
        &self,
        url: &str,
        json_body: &str,
        results: Arc<Mutex<Vec<Cont3xtResult>>>,
        disabled: std::collections::HashSet<String>,
    ) -> Result<(u64, String, Vec<(String, String)>)> {
        let start = std::time::Instant::now();
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
                log_http(&self.http_log, "POST", url, post_data.clone(), status, first_byte, first_byte, None);
                return self.stream_response_lines(resp, results, disabled).await;
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
                    log_http(&self.http_log, "POST", url, post_data.clone(), status, first_byte, first_byte, None);
                    return self.stream_response_lines(resp, results, disabled).await;
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
        log_http(&self.http_log, "POST", url, post_data, status, first_byte, first_byte, None);
        self.stream_response_lines(resp, results, disabled).await
    }

    async fn stream_response_lines(
        &self,
        resp: reqwest::Response,
        results: Arc<Mutex<Vec<Cont3xtResult>>>,
        disabled: std::collections::HashSet<String>,
    ) -> Result<(u64, String, Vec<(String, String)>)> {
        use futures_util::StreamExt;
        let mut total = 0u64;
        let mut itype = String::new();
        let mut init_indicators: Vec<(String, String)> = Vec::new();
        let mut buffer = String::new();
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                let line = line.trim().trim_start_matches('[').trim_end_matches(']').trim_end_matches(',');
                if line.is_empty() { continue; }
                let obj: Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let purpose = obj.get("purpose").and_then(|v| v.as_str()).unwrap_or("");
                match purpose {
                    "init" => {
                        total = obj.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(indicators) = obj.get("indicators").and_then(|v| v.as_array()) {
                            for ind in indicators {
                                let ind_itype = ind.get("itype").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let ind_query = ind.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                if itype.is_empty() {
                                    itype = ind_itype.clone();
                                }
                                init_indicators.push((ind_itype, ind_query));
                            }
                        }
                    }
                    "link" => {
                        // Capture parent-child indicator relationships
                        let child_query = obj.get("indicator")
                            .and_then(|v| v.get("query"))
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let child_itype = obj.get("indicator")
                            .and_then(|v| v.get("itype"))
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let parent_query = obj.get("parentIndicator")
                            .and_then(|v| v.get("query"))
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let parent_itype = obj.get("parentIndicator")
                            .and_then(|v| v.get("itype"))
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if !child_query.is_empty() && !parent_query.is_empty() {
                            if let Ok(mut vec) = results.lock() {
                                // Store link as a special marker result with empty name
                                vec.push(Cont3xtResult {
                                    name: String::new(),
                                    indicator: child_query,
                                    itype: child_itype,
                                    data: serde_json::json!({
                                        "_link_parent_query": parent_query,
                                        "_link_parent_itype": parent_itype,
                                    }),
                                    has_data: false,
                                });
                            }
                        }
                    }
                    "data" => {
                        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if disabled.contains(&name) { continue; }
                        let indicator = obj.get("indicator")
                            .and_then(|v| v.get("query"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("").to_string();
                        let ind_itype = obj.get("indicator")
                            .and_then(|v| v.get("itype"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("").to_string();
                        let data = obj.get("data").cloned().unwrap_or(Value::Null);
                        let has_data = data.as_object()
                            .map(|o| o.keys().any(|k| k != "_cont3xt"))
                            .unwrap_or(false);
                        if has_data {
                            let key = (name.clone(), indicator.clone());
                            if !seen.contains(&key) {
                                seen.insert(key);
                                if let Ok(mut vec) = results.lock() {
                                    vec.push(Cont3xtResult {
                                        name,
                                        indicator,
                                        itype: ind_itype,
                                        data,
                                        has_data,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok((total, itype, init_indicators))
    }
}
