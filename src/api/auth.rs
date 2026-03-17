use anyhow::Result;
use std::time::Instant;

use super::{ArkimeClient, AuthMode, log_http};
use super::auth_okta::decode_js_escapes;

impl ArkimeClient {
    /// Follow redirects manually, logging each hop. Returns final (url, response).
    pub(super) async fn follow_redirects(&self, initial_url: &str, method: &str) -> Result<(String, reqwest::Response)> {
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
    pub(super) async fn web_login(&mut self) -> Result<()> {
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
                if matches!(tag, "br" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr")
                    && !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
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
    pub(super) async fn okta_login(&mut self) -> Result<()> {
        if self.logged_in {
            return Ok(());
        }

        // Step 1: Navigate to app URL, following redirects to Okta login page
        let (auth_url, resp) = self.follow_redirects(&self.base_url.clone(), "GET").await?;
        let html_body = resp.text().await?;

        // Step 2: Extract modelDataBag JSON for config, stateToken, and labels
        let model_data = regex::Regex::new(r"var modelDataBag = '((?:[^'\\]|\\.)*?)'")
            .unwrap()
            .captures(&html_body)
            .and_then(|c| c.get(1))
            .map(|m| decode_js_escapes(m.as_str()));

        let (state_token, okta_base_url, username_label, password_label, app_name, brand_name) = if let Some(ref json_str) = model_data {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                let token = data["stateToken"].as_str().unwrap_or("").to_string();
                let base = data["baseUrl"].as_str().unwrap_or("").to_string();
                let settings = &data["orgLoginPageSettings"];
                let ulabel = settings["usernameLabel"].as_str().unwrap_or("Username").to_string();
                let plabel = settings["passwordLabel"].as_str().unwrap_or("Password").to_string();
                let app = data["appInstanceName"].as_str().unwrap_or("").to_string();
                let brand = data["brandName"].as_str().unwrap_or("").to_string();
                (token, base, ulabel, plabel, app, brand)
            } else {
                (String::new(), String::new(), "Username".to_string(), "Password".to_string(), String::new(), String::new())
            }
        } else {
            (String::new(), String::new(), "Username".to_string(), "Password".to_string(), String::new(), String::new())
        };

        // Fall back to var stateToken if not in modelDataBag
        let state_token = if !state_token.is_empty() {
            state_token
        } else {
            let raw = regex::Regex::new(r"var stateToken = '([^']+)'")
                .unwrap()
                .captures(&html_body)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| anyhow::anyhow!("Okta login: could not find stateToken in page ({})", auth_url))?;
            decode_js_escapes(&raw)
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

        // Extract fromURI (OAuth2 authorize URL) for the IDX flow
        let from_uri = {
            use scraper::{Html, Selector};
            let doc = Html::parse_document(&html_body);
            let sel = Selector::parse("input#fromURI").unwrap();
            doc.select(&sel).next()
                .and_then(|el| el.value().attr("value"))
                .map(|v| v.to_string())
        };

        // Step 4: Authenticate via Okta IDX API (Identity Engine) or classic authn
        eprintln!("Authenticating as '{}'...", username);

        let session_token = if let Some(ref from_uri_str) = from_uri {
            // Try IDX flow first (for Identity Engine orgs)
            match self.okta_idx_login(&okta_base, from_uri_str, &state_token, &username, &password).await {
                Ok(None) => {
                    // IDX flow completed and cookies are set — no sessionToken needed
                    String::new()
                }
                Ok(Some(token)) => token,
                Err(idx_err) => {
                    let err_str = idx_err.to_string();
                    // Only fall back to classic if IDX had a protocol/setup error,
                    // NOT if the user's credentials were actually rejected
                    let is_auth_failure = err_str.contains("Authentication failed")
                        || err_str.contains("Unable to sign in")
                        || err_str.contains("locked")
                        || err_str.contains("suspended");
                    if is_auth_failure {
                        anyhow::bail!("{}", err_str);
                    }
                    eprintln!("IDX flow failed ({}), trying classic authn...", idx_err);
                    self.okta_classic_authn(&okta_base, &username, &password, &state_token).await?
                }
            }
        } else {
            // No fromURI, use classic authn
            self.okta_classic_authn(&okta_base, &username, &password, &state_token).await?
        };

        // Step 5: Exchange session token for session cookie (if we got one)
        if !session_token.is_empty() {
            let redirect_url = if let Some(ref uri) = from_uri {
                format!("{}/login/sessionCookieRedirect?checkAccountSetupComplete=true&token={}&redirectUrl={}", okta_base, session_token, urlencoding::encode(uri))
            } else {
                format!("{}/login/sessionCookieRedirect?checkAccountSetupComplete=true&token={}&redirectUrl={}", okta_base, session_token, urlencoding::encode(&self.base_url))
            };
            self.follow_redirects(&redirect_url, "GET").await?;
        }

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
                    anyhow::bail!("HTTP {} (see debug log [D] for details)", status);
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
}
