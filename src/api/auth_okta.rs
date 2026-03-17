use anyhow::Result;
use reqwest::Client;
use std::time::Instant;

use super::{ArkimeClient, log_http};

/// Decode JavaScript string escapes commonly found in Okta pages
/// Handles: \xNN (hex byte), \uNNNN (unicode), \n, \r, \t, \", \\, \/
pub(super) fn decode_js_escapes(raw: &str) -> String {
    let mut decoded = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte as char);
                    } else {
                        // Don't silently drop — preserve the original escape
                        decoded.push('\\');
                        decoded.push('x');
                        decoded.push_str(&hex);
                    }
                }
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(code) {
                                decoded.push(c);
                            } else {
                                decoded.push('\\');
                                decoded.push('u');
                                decoded.push_str(&hex);
                            }
                        } else {
                            decoded.push('\\');
                            decoded.push('u');
                            decoded.push_str(&hex);
                        }
                    } else {
                        // Not enough chars for \uNNNN — preserve what we got
                        decoded.push('\\');
                        decoded.push('u');
                        decoded.push_str(&hex);
                    }
                }
                Some('n') => decoded.push('\n'),
                Some('r') => decoded.push('\r'),
                Some('t') => decoded.push('\t'),
                Some('"') => decoded.push('"'),
                Some('\'') => decoded.push('\''),
                Some('\\') => decoded.push('\\'),
                Some('/') => decoded.push('/'),
                Some(other) => { decoded.push('\\'); decoded.push(other); }
                None => { decoded.push('\\'); }
            }
        } else {
            decoded.push(ch);
        }
    }
    decoded
}

impl ArkimeClient {
    /// Okta IDX API flow (Identity Engine) — returns None if cookies are set directly,
    /// or Some(sessionToken) if a classic redirect is needed.
    pub(super) async fn okta_idx_login(&self, okta_base: &str, _from_uri: &str, page_state_token: &str, username: &str, password: &str) -> Result<Option<String>> {
        // The Okta Sign-In Widget on Identity Engine uses the page's stateToken
        // directly with /idp/idx/introspect (no interact call needed).

        // Helper: build IDX request with proper headers matching the Okta Sign-In Widget
        let okta_origin = okta_base.to_string();
        let idx_post = |url: &str, body: serde_json::Value, accept: Option<&str>| {
            self.client.post(url)
                .header("Content-Type", "application/json")
                .header("Accept", accept.unwrap_or("application/ion+json; okta-version=1.0.0"))
                .header("X-Okta-User-Agent-Extended", "okta-auth-js/7.14.0 okta-signin-widget-7.41.0")
                .header("Origin", &okta_origin)
                .header("Referer", &okta_origin)
                .body(body.to_string())
        };

        // Step 1: POST /idp/idx/introspect with the page's stateToken
        let introspect_url = format!("{}/idp/idx/introspect", okta_base);
        let start = Instant::now();
        let resp = self.client.post(&introspect_url)
            .header("Content-Type", "application/ion+json; okta-version=1.0.0")
            .header("Accept", "application/ion+json; okta-version=1.0.0")
            .header("X-Okta-User-Agent-Extended", "okta-auth-js/7.14.0 okta-signin-widget-7.41.0")
            .header("Origin", okta_base)
            .header("Referer", okta_base)
            .body(serde_json::json!({"stateToken": page_state_token}).to_string())
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        log_http(&self.http_log, "POST", &introspect_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
        if status != 200 {
            anyhow::bail!("IDX introspect failed: HTTP {} — {}", status, &body);
        }
        let introspect_resp: serde_json::Value = serde_json::from_str(&body)?;
        let state_handle = introspect_resp["stateHandle"].as_str()
            .ok_or_else(|| anyhow::anyhow!("IDX: no stateHandle in introspect response"))?
            .to_string();

        // Step 2: Check what remediations are available
        let mut current_resp = introspect_resp.clone();
        let mut state_handle = state_handle;

        // Helper: skip device-challenge-poll (Okta Verify FastPass) — CLI can't do FastPass.
        // Poll until the server offers alternative remediations (timeout ~15s).
        macro_rules! skip_device_poll {
            ($current_resp:expr, $state_handle:expr) => {{
                let has_dp = $current_resp["remediation"]["value"].as_array()
                    .map(|rems| rems.iter().any(|r| {
                        let n = r["name"].as_str().unwrap_or("");
                        n == "device-challenge-poll" || n == "challenge-poll"
                    }))
                    .unwrap_or(false);
                if has_dp {
                    // Try Okta Verify loopback challenge if the local agent is running
                    let mut loopback_done = false;
                    // authenticatorChallenge can be at top level or nested under currentAuthenticatorEnrollment
                    let ac_opt = $current_resp.get("authenticatorChallenge").and_then(|v| v.get("value"))
                        .or_else(|| $current_resp.get("currentAuthenticatorEnrollment")
                            .and_then(|v| v.get("value"))
                            .and_then(|v| v.get("contextualData")));
                    if let Some(ac) = ac_opt {
                        let challenge_request = ac.get("challengeRequest").and_then(|v| v.as_str());
                        let ports = ac.get("ports").and_then(|v| v.as_array());
                        // Use httpsDomain (e.g. "https://orgid.authenticatorlocalprod.com"), fall back to domain
                        let https_domain = ac.get("httpsDomain").and_then(|v| v.as_str())
                            .unwrap_or_else(|| ac.get("domain").and_then(|v| v.as_str()).unwrap_or(""));

                        if let (Some(challenge), Some(ports_arr)) = (challenge_request, ports) {
                            let loopback_client = Client::builder()
                                .danger_accept_invalid_certs(true)
                                .timeout(std::time::Duration::from_secs(3))
                                .build()
                                .unwrap();
                            for port_val in ports_arr {
                                // ports can be numbers, strings, or objects
                                let port = port_val.as_u64()
                                    .or_else(|| port_val.as_str().and_then(|s| s.parse::<u64>().ok()))
                                    .or_else(|| port_val.get("port").and_then(|p| p.as_u64()))
                                    .unwrap_or(0);
                                if port == 0 { continue; }
                                let base = format!("{}:{}", https_domain, port);
                                match loopback_client.get(format!("{}/probe", base))
                                    .header("Origin", &okta_origin)
                                    .send().await {
                                    Ok(resp) if resp.status().is_success() => {
                                        match loopback_client.post(format!("{}/challenge", base))
                                            .header("Content-Type", "application/json")
                                            .header("Origin", &okta_origin)
                                            .body(serde_json::json!({"challengeRequest": challenge}).to_string())
                                            .send().await {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    loopback_done = true;
                                                    break;
                                                }
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    if loopback_done {
                        let poll_url = $current_resp["remediation"]["value"].as_array()
                            .and_then(|rems| rems.iter().find(|r| {
                                let n = r["name"].as_str().unwrap_or("");
                                n == "device-challenge-poll" || n == "challenge-poll"
                            }))
                            .and_then(|r| r["href"].as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}/idp/idx/authenticators/poll", okta_base));
                        let poll_start = Instant::now();
                        let mut _attempt = 0u32;
                        while poll_start.elapsed().as_secs() < 15 {
                            _attempt += 1;
                            let start = Instant::now();
                            let resp = idx_post(&poll_url, serde_json::json!({"stateHandle": $state_handle}), None)
                                .send().await?;
                            let status = resp.status().as_u16();
                            let body = resp.text().await?;
                            log_http(&self.http_log, "POST", &poll_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                            if status != 200 { break; }
                            let poll_resp: serde_json::Value = serde_json::from_str(&body)?;
                            let rem_names: Vec<&str> = poll_resp["remediation"]["value"].as_array()
                                .map(|arr| arr.iter().filter_map(|r| r["name"].as_str()).collect())
                                .unwrap_or_default();
                            let only_device_poll = rem_names.len() == 1
                                && (rem_names[0] == "device-challenge-poll" || rem_names[0] == "challenge-poll");
                            if !only_device_poll {
                                $state_handle = poll_resp["stateHandle"].as_str()
                                    .unwrap_or(&$state_handle).to_string();
                                $current_resp = poll_resp;
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        }
                    } else {
                        // No loopback — use authenticator-level cancel to skip device challenge
                        // (different from /idp/idx/cancel which resets the whole flow)
                        let auth_cancel_url = $current_resp.get("authenticatorChallenge")
                            .and_then(|ac| ac.get("value"))
                            .and_then(|v| v.get("cancel"))
                            .and_then(|c| c.get("href"))
                            .and_then(|h| h.as_str())
                            .map(|s| s.to_string());
                        if let Some(cancel_url) = auth_cancel_url {
                            let start = Instant::now();
                            let resp = idx_post(&cancel_url, serde_json::json!({"stateHandle": $state_handle}), None)
                                .send().await?;
                            let status = resp.status().as_u16();
                            let body = resp.text().await?;
                            log_http(&self.http_log, "POST", &cancel_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                            if status == 200 {
                                let cancel_resp: serde_json::Value = serde_json::from_str(&body)?;
                                $state_handle = cancel_resp["stateHandle"].as_str()
                                    .unwrap_or(&$state_handle).to_string();
                                $current_resp = cancel_resp;
                            }
                        } else {
                            let cancel_url = format!("{}/idp/idx/cancel", okta_base);
                            let start = Instant::now();
                            let resp = idx_post(&cancel_url, serde_json::json!({"stateHandle": $state_handle}), None)
                                .send().await?;
                            let status = resp.status().as_u16();
                            let body = resp.text().await?;
                            log_http(&self.http_log, "POST", &cancel_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                            if status == 200 {
                                let cancel_resp: serde_json::Value = serde_json::from_str(&body)?;
                                $state_handle = cancel_resp["stateHandle"].as_str()
                                    .unwrap_or(&$state_handle).to_string();
                                $current_resp = cancel_resp;
                            }
                        }
                    }
                }
            }};
        }

        skip_device_poll!(current_resp, state_handle);

        let has_identify = current_resp["remediation"]["value"].as_array()
            .map(|rems| rems.iter().any(|r| r["name"].as_str() == Some("identify")))
            .unwrap_or(false);
        let has_challenge = current_resp["remediation"]["value"].as_array()
            .map(|rems| rems.iter().any(|r| {
                let name = r["name"].as_str().unwrap_or("");
                name == "challenge-authenticator" || name == "select-authenticator-authenticate"
            }))
            .unwrap_or(false);

        if has_identify && !has_challenge {
            // Need to identify first — build body from remediation's value array
            let identify_rem = current_resp["remediation"]["value"].as_array()
                .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("identify")));
            let identify_url = identify_rem
                .and_then(|r| r["href"].as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}/idp/idx/identify", okta_base));
            let identify_accepts = identify_rem
                .and_then(|r| r["accepts"].as_str())
                .map(|s| s.to_string());

            let mut identify_body = serde_json::json!({});
            if let Some(rem) = identify_rem
                && let Some(fields) = rem["value"].as_array() {
                    for field in fields {
                        let name = field["name"].as_str().unwrap_or("");
                        let mutable = field["mutable"].as_bool().unwrap_or(true);
                        if !mutable
                            && let Some(val) = field.get("value") {
                                identify_body[name] = val.clone();
                            }
                    }
                }
            identify_body["identifier"] = serde_json::Value::String(username.to_string());
            identify_body["credentials"] = serde_json::json!({"passcode": password});

            let start = Instant::now();
            let resp = idx_post(&identify_url, identify_body.clone(), identify_accepts.as_deref())
                .send()
                .await?;
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            log_http(&self.http_log, "POST", &identify_url, Some(format!("identifier={}", username)), status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));

            let identify_resp: serde_json::Value = serde_json::from_str(&body)
                .map_err(|_| anyhow::anyhow!("Okta IDX identify failed: HTTP {} — {}", status, &body))?;

            if status == 401 {
                let err_msg = identify_resp["messages"]["value"][0]["message"].as_str()
                    .unwrap_or("Authentication failed");
                anyhow::bail!("Okta login failed: {}", err_msg);
            }
            if status != 200 {
                let err_msg = identify_resp["messages"]["value"][0]["message"].as_str()
                    .or_else(|| identify_resp["errorSummary"].as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("Okta login failed: HTTP {} — {}", status, err_msg);
            }

            state_handle = identify_resp["stateHandle"].as_str()
                .unwrap_or(&state_handle)
                .to_string();
            current_resp = identify_resp;
        } else if has_challenge {
            // User already known from stateToken — skip identify
        } else {
            anyhow::bail!("Okta IDX: unexpected remediations after introspect — no identify or challenge found");
        }

        // Skip device-challenge-poll if it appears after identify (MFA step)
        skip_device_poll!(current_resp, state_handle);

        // Check if already succeeded (some orgs complete after identify alone)
        let already_done = current_resp["successWithInteractionCode"]["href"].as_str().is_some()
            || current_resp["sessionToken"].as_str().is_some();

        if !already_done {
            // Check if password authenticator is already selected via currentAuthenticator
            let current_auth_key = current_resp["currentAuthenticator"]["value"]["key"].as_str().unwrap_or("");
            let needs_authenticator_select = current_auth_key != "okta_password" && {
                current_resp["remediation"]["value"].as_array()
                    .map(|rems| rems.iter().any(|r| r["name"].as_str() == Some("select-authenticator-authenticate")))
                    .unwrap_or(false)
            };

            if needs_authenticator_select
                && let Some(rems) = current_resp.clone()["remediation"]["value"].as_array().cloned()
                    && let Some(select_auth) = rems.iter().find(|r| r["name"].as_str() == Some("select-authenticator-authenticate")) {
                // Find the password authenticator
                if let Some(auth_options) = select_auth["value"].as_array()
                    .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("authenticator")))
                    .and_then(|auth| auth["options"].as_array()) {
                    let password_auth = auth_options.iter().find(|opt| {
                        opt["label"].as_str().map(|l| l.to_lowercase().contains("password")).unwrap_or(false)
                            || opt["value"]["form"]["value"].as_array()
                                .map(|vals| vals.iter().any(|v| v["value"].as_str() == Some("okta_password")))
                                .unwrap_or(false)
                    });
                    if let Some(pwd_auth) = password_auth {
                        let auth_id = pwd_auth["value"]["form"]["value"].as_array()
                            .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("id")))
                            .and_then(|v| v["value"].as_str());
                        let method_type = pwd_auth["value"]["form"]["value"].as_array()
                            .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("methodType")))
                            .and_then(|v| v["value"].as_str());

                        let default_select_url = format!("{}/idp/idx/challenge", okta_base);
                        let select_url = select_auth["href"].as_str()
                            .unwrap_or(&default_select_url);
                        let mut select_body = serde_json::json!({"stateHandle": state_handle});
                        if let Some(id) = auth_id {
                            select_body["authenticator"] = serde_json::json!({"id": id});
                            if let Some(mt) = method_type {
                                select_body["authenticator"]["methodType"] = serde_json::Value::String(mt.to_string());
                            }
                        }

                        let start = Instant::now();
                        let select_accepts = select_auth["accepts"].as_str();
                        let resp = idx_post(select_url, select_body, select_accepts)
                            .send()
                            .await?;
                        let status = resp.status().as_u16();
                        let body = resp.text().await?;
                        log_http(&self.http_log, "POST", select_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                        if status != 200 {
                            anyhow::bail!("IDX select-authenticator failed: HTTP {} — {}", status, &body);
                        }
                        current_resp = serde_json::from_str(&body)?;
                        state_handle = current_resp["stateHandle"].as_str()
                            .unwrap_or(&state_handle)
                            .to_string();
                    }
                }
            } // close if let select_auth
             // close if needs_authenticator_select

            // Submit password via challenge/answer
            // Build body from the challenge remediation's immutable fields
            let challenge_rem = current_resp["remediation"]["value"].as_array()
                .and_then(|rems| rems.iter().find(|r| {
                    let name = r["name"].as_str().unwrap_or("");
                    name == "challenge-authenticator" || name == "answer"
                }));

            // If no challenge remediation, the identify response might still have "identify"
            // with currentAuthenticator=password, meaning we need to re-POST identify with password
            let (challenge_url, challenge_accepts, mut challenge_body) = if let Some(rem) = challenge_rem {
                let url = rem["href"].as_str().unwrap_or(&format!("{}/idp/idx/challenge/answer", okta_base)).to_string();
                let accepts = rem["accepts"].as_str().map(|s| s.to_string());
                let mut body = serde_json::json!({});
                if let Some(fields) = rem["value"].as_array() {
                    for field in fields {
                        let name = field["name"].as_str().unwrap_or("");
                        if !field["mutable"].as_bool().unwrap_or(true)
                            && let Some(val) = field.get("value") {
                                body[name] = val.clone();
                            }
                    }
                }
                (url, accepts, body)
            } else {
                // Fall back: re-submit to identify endpoint with password using new stateHandle
                let url = format!("{}/idp/idx/identify", okta_base);
                let accepts: Option<String> = None;
                let mut body = serde_json::json!({"stateHandle": state_handle});
                body["identifier"] = serde_json::Value::String(username.to_string());
                (url, accepts, body)
            };
            challenge_body["credentials"] = serde_json::json!({"passcode": password});

            let start = Instant::now();
            let resp = idx_post(&challenge_url, challenge_body, challenge_accepts.as_deref())
                .send()
                .await?;
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            log_http(&self.http_log, "POST", &challenge_url, Some("credentials=***".into()), status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));

            if status == 401 || status == 403 {
                let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                    .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| body.to_string());
                anyhow::bail!("Okta login failed: {}", err_msg);
            }
            if status != 200 {
                let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                    .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| body.to_string());
                anyhow::bail!("Okta login failed: HTTP {} — {}", status, err_msg);
            }

            current_resp = serde_json::from_str(&body)?;
            if let Some(sh) = current_resp["stateHandle"].as_str() {
                state_handle = sh.to_string();
            }
        } // close if !already_done

        // Check if we need MFA
        let mfa_remediations: Vec<String> = current_resp["remediation"]["value"].as_array()
            .map(|arr| arr.iter().filter_map(|r| r["name"].as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        if mfa_remediations.contains(&"select-authenticator-authenticate".to_string())
            || mfa_remediations.contains(&"challenge-authenticator".to_string())
        {
            // Check what authenticator is currently selected (if any)
            let current_auth_key = current_resp["currentAuthenticatorEnrollment"]["value"]["key"]
                .as_str()
                .or_else(|| current_resp["currentAuthenticator"]["value"]["key"].as_str())
                .unwrap_or("");
            let current_auth_type = current_resp["currentAuthenticatorEnrollment"]["value"]["type"]
                .as_str()
                .or_else(|| current_resp["currentAuthenticator"]["value"]["type"].as_str())
                .unwrap_or("");

            // Gather available authenticators from select-authenticator-authenticate options
            let mut available_auths: Vec<(String, String, String)> = Vec::new(); // (label, id, method_type)
            if let Some(select_rem) = current_resp["remediation"]["value"].as_array()
                .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("select-authenticator-authenticate")))
                && let Some(auth_field) = select_rem["value"].as_array()
                    .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("authenticator")))
                    && let Some(options) = auth_field["options"].as_array() {
                        for opt in options {
                            let label = opt["label"].as_str().unwrap_or("?").to_string();
                            let id = opt["value"]["form"]["value"].as_array()
                                .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("id")))
                                .and_then(|v| v["value"].as_str())
                                .unwrap_or("").to_string();
                            let method = opt["value"]["form"]["value"].as_array()
                                .and_then(|vals| vals.iter().find(|v| v["name"].as_str() == Some("methodType")))
                                .and_then(|v| v["value"].as_str())
                                .unwrap_or("").to_string();
                            available_auths.push((label, id, method));
                        }
                    }

            // If challenge-authenticator is already active for a TOTP/OTP-like authenticator, prompt directly
            let challenge_rem = current_resp["remediation"]["value"].as_array()
                .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("challenge-authenticator")).cloned());

            let is_otp_challenge = matches!(current_auth_key,
                "okta_password" | "google_otp" | "okta_verify" | "phone_number" |
                "okta_email" | "duo" | "symantec_vip" | "custom_otp" | "onprem_mfa"
            ) || matches!(current_auth_type, "app" | "email" | "phone" | "security_key");

            // If we don't have a usable challenge, try to select a TOTP/email/phone authenticator
            if !is_otp_challenge || challenge_rem.is_none() {
                // Preference order: TOTP > email > phone
                let preferred_keys = ["google_otp", "okta_verify", "custom_otp", "onprem_mfa", "okta_email", "phone_number"];
                let selected = preferred_keys.iter().find_map(|&pref| {
                    available_auths.iter().find(|(_, _, method)| method == pref || {
                        // Also match by label keywords
                        false
                    }).or_else(|| {
                        available_auths.iter().find(|(label, _, _)| {
                            let lower = label.to_lowercase();
                            match pref {
                                "google_otp" => lower.contains("google"),
                                "okta_verify" => lower.contains("okta verify"),
                                "okta_email" => lower.contains("email"),
                                "phone_number" => lower.contains("phone") || lower.contains("sms"),
                                _ => false,
                            }
                        })
                    })
                });

                if let Some((label, id, method)) = selected {
                    // For Okta Verify, use signed_nonce — this is the only method enrolled
                    // and matches what the browser sends to /idx/challenge.
                    let effective_method = if method.is_empty() && label.to_lowercase().contains("okta verify") {
                        "signed_nonce".to_string()
                    } else {
                        method.clone()
                    };
                    let select_rem = current_resp["remediation"]["value"].as_array()
                        .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("select-authenticator-authenticate")));
                    let select_url = select_rem
                        .and_then(|r| r["href"].as_str())
                        .unwrap_or(&format!("{}/idp/idx/challenge", okta_base))
                        .to_string();
                    let mut select_body = serde_json::json!({"stateHandle": state_handle});
                    select_body["authenticator"] = serde_json::json!({"id": id});
                    if !effective_method.is_empty() {
                        select_body["authenticator"]["methodType"] = serde_json::Value::String(effective_method.clone());
                    }

                    // Browser always sends Accept: application/json for challenge calls
                    // (not ion+json) — this determines whether Okta returns
                    // device-challenge-poll (loopback) vs challenge-poll (push-only)
                    let start = Instant::now();
                    let resp = idx_post(&select_url, select_body, Some("application/json; okta-version=1.0.0"))
                        .send().await?;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", &select_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                    if status != 200 {
                        let err_detail = serde_json::from_str::<serde_json::Value>(&body).ok()
                            .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| body.to_string());
                        anyhow::bail!("IDX MFA select-authenticator failed: HTTP {} — {}", status, err_detail);
                    }
                    current_resp = serde_json::from_str(&body)?;
                    state_handle = current_resp["stateHandle"].as_str()
                        .unwrap_or(&state_handle).to_string();

                    // After selecting push/Okta Verify, we get challenge-poll (push notification)
                    // or device-challenge-poll (loopback to local OV agent)
                    let post_select_rems: Vec<String> = current_resp["remediation"]["value"].as_array()
                        .map(|arr| arr.iter().filter_map(|r| r["name"].as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();

                    if post_select_rems.contains(&"device-challenge-poll".to_string()) {
                        skip_device_poll!(current_resp, state_handle);
                    } else if post_select_rems.contains(&"challenge-poll".to_string()) {
                        // challenge-poll with signed_nonce — check if contextualData has LOOPBACK challenge
                        // The local OV agent must receive the challenge to trigger approval
                        let cd = current_resp.get("currentAuthenticator")
                            .and_then(|v| v.get("value"))
                            .and_then(|v| v.get("contextualData"))
                            .and_then(|v| v.get("challenge"))
                            .and_then(|v| v.get("value"));
                        let challenge_method = cd.and_then(|v| v["challengeMethod"].as_str()).unwrap_or("");
                        if challenge_method == "LOOPBACK" {
                            let challenge_request = cd.and_then(|v| v["challengeRequest"].as_str()).unwrap_or("");
                            let https_domain = cd.and_then(|v| v["httpsDomain"].as_str()).unwrap_or("");
                            let ports = cd.and_then(|v| v["ports"].as_array());
                            if !challenge_request.is_empty()
                                && let Some(ports_arr) = ports {
                                    let loopback_client = Client::builder()
                                        .danger_accept_invalid_certs(true)
                                        .pool_max_idle_per_host(0)
                                        .build()
                                        .unwrap();
                                    // Separate short timeout for probe, long timeout for challenge
                                    let probe_timeout = std::time::Duration::from_secs(3);
                                    let challenge_timeout = std::time::Duration::from_secs(300);
                                    let mut loopback_ok = false;
                                    for port_val in ports_arr {
                                        let port = port_val.as_str().and_then(|s| s.parse::<u64>().ok())
                                            .or_else(|| port_val.as_u64())
                                            .unwrap_or(0);
                                        if port == 0 { continue; }
                                        let base = format!("{}:{}", https_domain, port);
                                        match loopback_client.get(format!("{}/probe", base))
                                            .header("Origin", &okta_origin)
                                            .timeout(probe_timeout)
                                            .send().await {
                                            Ok(resp) if resp.status().is_success() => {
                                                if let Ok(resp) = loopback_client.post(format!("{}/challenge", base))
                                                    .header("Content-Type", "application/json")
                                                    .header("Origin", &okta_origin)
                                                    .timeout(challenge_timeout)
                                                    .body(serde_json::json!({"challengeRequest": challenge_request}).to_string())
                                                    .send().await {
                                                    if resp.status().is_success() {
                                                        loopback_ok = true;
                                                        break;
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    let _ = loopback_ok;
                                }
                        }
                        // Fall through to the challenge-poll handler below
                    } else if post_select_rems.contains(&"authenticator-verification-data".to_string()) {
                        // Need to specify methodType in a second step
                        let avd_rem = current_resp["remediation"]["value"].as_array()
                            .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("authenticator-verification-data")));
                        let avd_url = avd_rem
                            .and_then(|r| r["href"].as_str())
                            .unwrap_or(&format!("{}/idp/idx/challenge", okta_base))
                            .to_string();
                        let avd_body = serde_json::json!({
                            "stateHandle": state_handle,
                            "authenticator": {"id": id, "methodType": "push"}
                        });
                        let start = Instant::now();
                        let resp = idx_post(&avd_url, avd_body, Some("application/json; okta-version=1.0.0"))
                            .send().await?;
                        let status = resp.status().as_u16();
                        let body = resp.text().await?;
                        log_http(&self.http_log, "POST", &avd_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));
                        if status != 200 {
                            let err_detail = serde_json::from_str::<serde_json::Value>(&body).ok()
                                .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                                .unwrap_or_else(|| body.to_string());
                            anyhow::bail!("IDX authenticator-verification-data failed: HTTP {} — {}", status, err_detail);
                        }
                        current_resp = serde_json::from_str(&body)?;
                        state_handle = current_resp["stateHandle"].as_str()
                            .unwrap_or(&state_handle).to_string();
                    } else {
                        // MFA authenticator selected, will prompt for code
                    }
                } else if available_auths.is_empty() && challenge_rem.is_some() {
                    // No select options but challenge is active — try prompting anyway
                } else {
                    let auth_names: Vec<&str> = available_auths.iter().map(|(l, _, _)| l.as_str()).collect();
                    anyhow::bail!("Okta MFA required but no supported authenticator found. Available: {:?}. \
                        Supported: TOTP, email, phone/SMS. WebAuthn/FIDO2 is not supported from CLI.", auth_names);
                }
            }

            // Check for challenge-poll (Okta Verify push notification — poll until user approves)
            let has_challenge_poll = current_resp["remediation"]["value"].as_array()
                .map(|arr| arr.iter().any(|r| r["name"].as_str() == Some("challenge-poll")))
                .unwrap_or(false);

            if has_challenge_poll {
                let poll_rem = current_resp["remediation"]["value"].as_array()
                    .and_then(|rems| rems.iter().find(|r| r["name"].as_str() == Some("challenge-poll")).cloned())
                    .unwrap();
                let poll_url = poll_rem["href"].as_str()
                    .unwrap_or(&format!("{}/idp/idx/challenge/poll", okta_base))
                    .to_string();
                // Check for refresh interval from remediation
                let refresh_ms = poll_rem.get("refresh")
                    .and_then(|r| r.as_u64())
                    .unwrap_or(4000);
                eprintln!("  Okta Verify: check your device to approve the sign-in request...");

                let poll_start = Instant::now();
                let max_poll_secs = 60;
                let mut poll_count = 0u32;
                loop {
                    let wait_secs = if poll_count == 0 { 2 } else { (refresh_ms / 1000).max(2) };
                    tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                    poll_count += 1;
                    let elapsed = poll_start.elapsed().as_secs();
                    if elapsed > max_poll_secs {
                        anyhow::bail!("Okta Verify push timed out after {}s. Try again.", max_poll_secs);
                    }

                    let start = Instant::now();
                    let resp = idx_post(&poll_url, serde_json::json!({"stateHandle": state_handle}), Some("application/json; okta-version=1.0.0"))
                        .send().await?;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", &poll_url, None, status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));

                    if status != 200 {
                        eprintln!("  Okta Verify poll failed: HTTP {}", status);
                        anyhow::bail!("Okta Verify poll failed: HTTP {}", status);
                    }
                    let poll_resp: serde_json::Value = serde_json::from_str(&body)?;
                    let rem_names: Vec<&str> = poll_resp["remediation"]["value"].as_array()
                        .map(|arr| arr.iter().filter_map(|r| r["name"].as_str()).collect())
                        .unwrap_or_default();
                    // Check for success — Okta IDX returns "successWithInteractionCode" (OIDC)
                    // or "success" (some signed_nonce flows)
                    let has_success = poll_resp.get("successWithInteractionCode").is_some()
                        || poll_resp.get("success").is_some();
                    if has_success {
                        current_resp = poll_resp;
                        let _ = current_resp["stateHandle"].as_str()
                            .map(|s| state_handle = s.to_string());
                        break;
                    }
                    if rem_names.contains(&"challenge-poll") || rem_names.is_empty() {
                        // Still waiting — update stateHandle and keep polling
                        if let Some(sh) = poll_resp["stateHandle"].as_str() {
                            state_handle = sh.to_string();
                        }
                        continue;
                    }

                    current_resp = poll_resp;
                    let _ = current_resp["stateHandle"].as_str()
                        .map(|s| state_handle = s.to_string());
                    break;
                }
            } else {
                // Not a push — check for challenge-authenticator (OTP code entry)
                let mfa_challenge = current_resp["remediation"]["value"].as_array()
                    .and_then(|rems| rems.iter().find(|r| {
                        let name = r["name"].as_str().unwrap_or("");
                        name == "challenge-authenticator" || name == "answer"
                    }).cloned());

                if let Some(mfa_rem) = mfa_challenge {
                    let mfa_url = mfa_rem["href"].as_str()
                        .unwrap_or(&format!("{}/idp/idx/challenge/answer", okta_base))
                        .to_string();
                    let mfa_accepts = mfa_rem["accepts"].as_str().map(|s| s.to_string());

                    // Build body with immutable fields
                    let mut mfa_body = serde_json::json!({});
                    if let Some(fields) = mfa_rem["value"].as_array() {
                        for field in fields {
                            let name = field["name"].as_str().unwrap_or("");
                            if !field["mutable"].as_bool().unwrap_or(true)
                                && let Some(val) = field.get("value") {
                                    mfa_body[name] = val.clone();
                                }
                        }
                    }

                    // Determine prompt text from current authenticator
                    let auth_label = current_resp["currentAuthenticatorEnrollment"]["value"]["displayName"]
                        .as_str()
                        .or_else(|| current_resp["currentAuthenticatorEnrollment"]["value"]["profile"]["email"].as_str())
                        .or_else(|| current_resp["currentAuthenticatorEnrollment"]["value"]["profile"]["phoneNumber"].as_str())
                        .or_else(|| current_resp["currentAuthenticator"]["value"]["displayName"].as_str())
                        .unwrap_or("your authenticator");

                    let code = rpassword::prompt_password(format!("Enter verification code from {}: ", auth_label))?;
                    mfa_body["credentials"] = serde_json::json!({"passcode": code});

                    let start = Instant::now();
                    let resp = idx_post(&mfa_url, mfa_body, mfa_accepts.as_deref())
                        .send().await?;
                    let status = resp.status().as_u16();
                    let body = resp.text().await?;
                    log_http(&self.http_log, "POST", &mfa_url, Some("credentials=***".into()), status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));

                    if status == 401 || status == 403 {
                        let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                            .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| body.to_string());
                        anyhow::bail!("Okta MFA failed: {}", err_msg);
                    }
                    if status != 200 {
                        let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                            .and_then(|v| v["messages"]["value"][0]["message"].as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| body.to_string());
                        anyhow::bail!("Okta MFA failed: HTTP {} — {}", status, err_msg);
                    }

                    current_resp = serde_json::from_str(&body)?;
                    let _ = current_resp["stateHandle"].as_str()
                        .map(|s| state_handle = s.to_string());
                } else {
                    // No challenge remediation found after MFA selection
                    let rems: Vec<&str> = current_resp["remediation"]["value"].as_array()
                        .map(|arr| arr.iter().filter_map(|r| r["name"].as_str()).collect())
                        .unwrap_or_default();
                    anyhow::bail!("Okta MFA: no challenge remediation found after authenticator selection. Remediations: {:?}", rems);
                }
            }

            // Check if there's another round of MFA or if we're done
            let post_mfa_rems: Vec<String> = current_resp["remediation"]["value"].as_array()
                .map(|arr| arr.iter().filter_map(|r| r["name"].as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            if !post_mfa_rems.is_empty() && current_resp.get("successWithInteractionCode").is_none()
                && current_resp.get("success").is_none() {
            }
        }

        // Check for success
        // Okta IDX returns "successWithInteractionCode" (OIDC) or "success" (some signed_nonce flows)
        let success_href = current_resp["successWithInteractionCode"]["href"].as_str()
            .or_else(|| current_resp["success"]["href"].as_str());
        if let Some(href) = success_href {
            // Follow the success href directly — it establishes the session
            self.follow_redirects(href, "GET").await?;
            return Ok(None);
        }

        // Some flows return a sessionToken (use classic sessionCookieRedirect)
        if let Some(token) = current_resp["sessionToken"].as_str() {
            return Ok(Some(token.to_string()));
        }

        anyhow::bail!("Okta IDX: unexpected response — no success redirect or sessionToken found");
    }

    /// Classic Okta authn API flow (non-Identity Engine)
    pub(super) async fn okta_classic_authn(&self, okta_base: &str, username: &str, password: &str, state_token: &str) -> Result<String> {
        let authn_url = format!("{}/api/v1/authn", okta_base);

        let authn_body = if state_token.is_empty() {
            serde_json::json!({"username": username, "password": password})
        } else {
            serde_json::json!({"username": username, "password": password, "stateToken": state_token})
        };

        let start = Instant::now();
        let resp = self.client.post(&authn_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(authn_body.to_string())
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        log_http(&self.http_log, "POST", &authn_url, Some(format!("username={}&password=***", username)), status, start.elapsed().as_millis() as u64, start.elapsed().as_millis() as u64, Some(&body));

        if status == 401 {
            let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                .and_then(|v| v["errorSummary"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| body.to_string());
            anyhow::bail!("Okta login failed: {}", err_msg);
        }
        if status != 200 {
            let err_msg = serde_json::from_str::<serde_json::Value>(&body).ok()
                .and_then(|v| v["errorSummary"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| body.to_string());
            anyhow::bail!("Okta login failed: HTTP {} — {}", status, err_msg);
        }

        let authn_resp: serde_json::Value = serde_json::from_str(&body)?;
        match authn_resp["status"].as_str().unwrap_or("") {
            "SUCCESS" => {
                authn_resp["sessionToken"].as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow::anyhow!("Okta: SUCCESS but no sessionToken"))
            }
            "MFA_REQUIRED" => {
                self.okta_handle_mfa(&authn_resp, okta_base).await
            }
            other => {
                anyhow::bail!("Okta: unexpected status '{}'", other);
            }
        }
    }

    /// Handle Okta MFA challenge — supports push notification and TOTP code
    pub(super) async fn okta_handle_mfa(&self, authn_resp: &serde_json::Value, _okta_base: &str) -> Result<String> {
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
}
