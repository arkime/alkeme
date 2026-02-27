use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};

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
                let truncated = if b.len() > 4096 { &b[..4096] } else { b };
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
