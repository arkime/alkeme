mod sessions;
mod stats;
mod arkime;
mod popups;

// Re-export app types for sub-modules via `use super::*`
#[allow(unused_imports)]
use crate::app::{App, AppMode, ActionTarget, C3StatsTab, ColumnEditorMode, Cont3xtFocus, DetailActionMenu, GraphType, InputMode, LayoutPopupMode, LineMode, SessionView, StatsTab, StatsView, SummaryMetric, SummarySort, Tab, TimeRange, ViewPopupMode, is_hidden_detail_field};
use crate::api::{CardField, Cont3xtCard};
use chrono::{DateTime, Local};
use ratatui::{
    prelude::*,
    widgets::*,
};

pub(super) fn format_epoch(val: &serde_json::Value, _field_type: &str) -> String {
    match val {
        serde_json::Value::Number(n) => {
            // Both "seconds" and "date" types are stored as epoch milliseconds
            let ms = n.as_i64().unwrap_or(0);
            let secs = ms / 1000;
            if let Some(dt) = DateTime::from_timestamp(secs, 0) {
                let local: DateTime<Local> = dt.into();
                return local.format("%Y/%m/%d %H:%M:%S").to_string();
            }
            n.to_string()
        }
        serde_json::Value::Null => "-".into(),
        other => other.to_string(),
    }
}

pub(super) fn format_epoch_short(ms: f64) -> String {
    let secs = (ms / 1000.0) as i64;
    if let Some(dt) = DateTime::from_timestamp(secs, 0) {
        let local: DateTime<Local> = dt.into();
        return local.format("%Y/%m/%d %H:%M").to_string();
    }
    "-".into()
}

pub(super) fn format_epoch_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let millis = (ms % 1000) as u32;
    if let Some(dt) = DateTime::from_timestamp(secs, millis * 1_000_000) {
        let local: DateTime<Local> = dt.into();
        return local.format("%H:%M:%S%.3f").to_string();
    }
    "-".into()
}

pub(super) fn format_human_bytes(bytes: f64) -> String {
    const TI: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;
    const GI: f64 = 1024.0 * 1024.0 * 1024.0;
    const MI: f64 = 1024.0 * 1024.0;
    const KI: f64 = 1024.0;

    if bytes >= TI {
        format!("{:.1}Ti", bytes / TI)
    } else if bytes >= GI {
        format!("{:.1}Gi", bytes / GI)
    } else if bytes >= MI {
        format!("{:.1}Mi", bytes / MI)
    } else if bytes >= KI {
        format!("{:.1}Ki", bytes / KI)
    } else {
        format!("{:.0}", bytes)
    }
}

pub(super) fn format_human_megabytes(mb: f64) -> String {
    format_human_bytes(mb * 1024.0 * 1024.0)
}

pub(super) fn format_epoch_secs(val: &serde_json::Value) -> String {
    let secs = match val {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        _ => return "-".into(),
    };
    if let Some(dt) = DateTime::from_timestamp(secs, 0) {
        let local: DateTime<Local> = dt.into();
        return local.format("%Y/%m/%d %H:%M:%S").to_string();
    }
    "-".into()
}

pub(super) fn parse_size_string(s: &str) -> Option<f64> {
    let s = s.trim();
    let (num_str, mult) = if let Some(n) = s.strip_suffix("tb") {
        (n, 1024.0 * 1024.0 * 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix("gb") {
        (n, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix("mb") {
        (n, 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix("kb") {
        (n, 1024.0)
    } else if let Some(n) = s.strip_suffix('b') {
        (n, 1.0)
    } else {
        (s, 1.0)
    };
    num_str.trim().parse::<f64>().ok().map(|v| v * mult)
}

pub(super) fn ip_protocol_str(val: &serde_json::Value) -> String {
    let num = match val {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
        _ => return "-".into(),
    };
    match num {
        1 => "ICMP".into(),
        2 => "IGMP".into(),
        6 => "TCP".into(),
        17 => "UDP".into(),
        41 => "IPv6".into(),
        47 => "GRE".into(),
        50 => "ESP".into(),
        58 => "ICM6".into(),
        89 => "OSPF".into(),
        132 => "SCTP".into(),
        _ => format!("{num}"),
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.app_mode {
        AppMode::Viewer => draw_viewer(f, app),
        AppMode::Cont3xt => draw_cont3xt(f, app),
        _ => draw_placeholder(f, app),
    }

    // Common overlays (shared across modes)
    if app.show_help {
        popups::draw_help(f, app, f.area());
    }
    if app.show_debug {
        popups::draw_debug(f, app, f.area());
    }
    if app.show_loading {
        popups::draw_loading(f, app, f.area());
    }
}

fn draw_viewer(f: &mut Frame, app: &mut App) {
    match app.active_tab {
        Tab::Stats => draw_stats_layout(f, app),
        _ => draw_default_layout(f, app),
    }

    if app.action_menu.is_some() {
        popups::draw_action_menu(f, app, f.area());
    }
    if app.input_mode == InputMode::ActionPrompt {
        popups::draw_action_prompt(f, app, f.area());
    }
    if app.input_mode == InputMode::FieldSelector {
        arkime::draw_field_selector(f, app, f.area());
    }
    if app.vr_detail_action_menu.is_some() && app.active_tab == Tab::Arkime {
        sessions::draw_detail_action_menu(f, app, f.area());
    }
    if app.vr_packets_view.is_some() {
        popups::draw_packets(f, app, f.area());
    }
    if app.vr_show_column_editor {
        popups::draw_column_editor(f, app, f.area());
    }
    if app.vr_show_layout_popup {
        popups::draw_layout_popup(f, app, f.area());
    }
    if app.vr_show_view_popup {
        popups::draw_view_popup(f, app, f.area());
    }
}

fn draw_cont3xt(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(3), // search bar
            Constraint::Min(0),   // content
            Constraint::Length(status_h), // status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Search => {
            draw_cont3xt_search_bar(f, app, chunks[1]);
            draw_cont3xt_results(f, app, chunks[2]);
        }
        Tab::C3Stats => {
            // Merge search bar and content area for stats
            let stats_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, chunks[1].height + chunks[2].height);
            c3_draw_stats(f, app, stats_area);
        }
        Tab::History => {
            let block = Block::default().borders(Borders::ALL).title(" History ");
            f.render_widget(block, chunks[1]);
            let block = Block::default().borders(Borders::ALL).title(" Query History ");
            let placeholder = Paragraph::new("  History coming soon...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            f.render_widget(placeholder, chunks[2]);
        }
        Tab::Settings => {
            let block = Block::default().borders(Borders::ALL).title(" Settings ");
            f.render_widget(block, chunks[1]);
            arkime::draw_under_construction(f, app, chunks[2]);
            arkime::draw_owl(f, app, chunks[2]);
        }
        _ => {}
    }

    draw_status_bar(f, app, chunks[3]);

    if app.c3_show_link_popup {
        draw_link_popup(f, app, f.area());
    }

    if app.c3_show_integration_popup {
        draw_integration_popup(f, app, f.area());
    }
}

fn draw_cont3xt_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let expr_display = if app.input_mode == InputMode::Expression {
        &app.expression_edit
    } else {
        &app.expression
    };
    let expr_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let itype_label = if !app.c3_search_itype.is_empty() {
        format!(" [{}] ", app.c3_search_itype)
    } else {
        String::new()
    };

    let expr_widget = Paragraph::new(Span::styled(expr_display.as_str(), expr_style))
        .block(Block::default().borders(Borders::ALL).title(format!(" Search (/) {itype_label}")));
    f.render_widget(expr_widget, area);

    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            area.x + app.expression_cursor as u16 + 1,
            area.y + 1,
        ));
    }
}

fn draw_cont3xt_results(f: &mut Frame, app: &mut App, area: Rect) {
    if app.c3_results.is_empty() && app.expression.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Results ");
        let placeholder = Paragraph::new("  Enter an indicator to search (IP, domain, hash, email, ...)")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(placeholder, area);
        return;
    }
    if app.c3_results.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Results ");
        let text = if app.show_loading {
            "  Searching...".to_string()
        } else {
            format!("  No results for: {}", app.expression)
        };
        let content = Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(content, area);
        return;
    }

    // Split into left (integration list) and right (detail)
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),  // integration list
            Constraint::Min(0),     // detail pane
        ])
        .split(area);

    // Left pane: integration results list
    let results_focused = app.c3_focus == Cont3xtFocus::Results;
    let results_border_style = if results_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(results_border_style)
        .title(format!(" Integrations ({}) ", app.c3_results.len()));

    let inner = results_block.inner(horiz[0]);
    f.render_widget(results_block, horiz[0]);

    let visible_height = inner.height as usize;
    app.visible_rows = visible_height;

    // Scroll the list to keep selection visible
    let scroll_offset = if app.c3_selected >= visible_height {
        app.c3_selected - visible_height + 1
    } else {
        0
    };

    for (i, result) in app.c3_results.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let y = inner.y + (i - scroll_offset) as u16;
        if y >= inner.y + inner.height { break; }

        let is_selected = i == app.c3_selected;
        let style = if is_selected && results_focused {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let indicator_suffix = if result.indicator != app.expression {
            format!(" ({})", result.indicator)
        } else {
            String::new()
        };
        let label = format!(" {}{}", result.name, indicator_suffix);
        let truncated = if label.len() > inner.width as usize {
            format!("{}…", &label[..inner.width as usize - 1])
        } else {
            format!("{:<width$}", label, width = inner.width as usize)
        };

        let span = Span::styled(truncated, style);
        f.render_widget(Paragraph::new(span), Rect::new(inner.x, y, inner.width, 1));
    }

    // Right pane: detail for selected integration
    let detail_focused = app.c3_focus == Cont3xtFocus::Detail;
    let detail_border_style = if detail_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if let Some(result) = app.c3_results.get(app.c3_selected) {
        // Find the card definition for this integration
        let card = if !app.c3_raw_view {
            app.c3_integrations.iter()
                .find(|i| i.name == result.name)
                .and_then(|i| i.card.as_ref())
        } else {
            None
        };

        let view_label = if app.c3_raw_view { " [RAW] " } else { "" };
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(detail_border_style)
            .title(format!(" {} — {} {view_label}", result.name, result.indicator));

        let detail_inner = detail_block.inner(horiz[1]);
        f.render_widget(detail_block, horiz[1]);

        // Build lines based on card definition or raw JSON
        let mut lines = if let Some(card) = card {
            render_card_lines(card, &result.data, &result.indicator)
        } else {
            flatten_json_to_lines(&result.data, "", 0)
        };
        align_table_columns(&mut lines);
        let total_lines = lines.len();

        // Clamp scroll
        let max_scroll = total_lines.saturating_sub(detail_inner.height as usize);
        let scroll = (app.c3_detail_scroll as usize).min(max_scroll);
        app.c3_detail_scroll = scroll as u16;

        for (i, line) in lines.iter().skip(scroll).take(detail_inner.height as usize).enumerate() {
            let y = detail_inner.y + i as u16;
            if y >= detail_inner.y + detail_inner.height { break; }

            let spans = match line {
                JsonLine::KeyValue(key, value) => {
                    vec![
                        Span::styled(format!(" {}: ", key), Style::default().fg(Color::Yellow)),
                        Span::styled(value.clone(), Style::default().fg(Color::White)),
                    ]
                }
                JsonLine::Header(key, is_array) => {
                    let suffix = if *is_array { " [" } else { " {" };
                    vec![Span::styled(format!(" {}{}", key, suffix), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]
                }
                JsonLine::Close(bracket) => {
                    vec![Span::styled(format!(" {bracket}"), Style::default().fg(Color::DarkGray))]
                }
                JsonLine::ArrayValue(value) => {
                    vec![
                        Span::styled("   • ", Style::default().fg(Color::DarkGray)),
                        Span::styled(value.clone(), Style::default().fg(Color::White)),
                    ]
                }
                JsonLine::TableRow(cells, widths) => {
                    let row_str = format_table_cells(cells, widths, " │ ");
                    let hscroll = app.c3_detail_hscroll as usize;
                    let visible: String = row_str.chars().skip(hscroll).collect();
                    vec![
                        Span::raw("  "),
                        Span::styled(visible, Style::default().fg(Color::White)),
                    ]
                }
                JsonLine::TableHeader(cells, widths) => {
                    let row_str = format_table_cells(cells, widths, " │ ");
                    let hscroll = app.c3_detail_hscroll as usize;
                    let visible: String = row_str.chars().skip(hscroll).collect();
                    vec![
                        Span::raw("  "),
                        Span::styled(visible, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    ]
                }
            };

            f.render_widget(Paragraph::new(Line::from(spans)), Rect::new(detail_inner.x, y, detail_inner.width, 1));
        }

        // Scrollbar indicator + raw toggle hint
        let hint = if detail_focused { " R:toggle raw " } else { "" };
        if total_lines > detail_inner.height as usize {
            let pct = if max_scroll > 0 { scroll * 100 / max_scroll } else { 0 };
            let indicator = format!(" {}/{} ({}%){hint}", scroll + 1, total_lines, pct);
            let x = horiz[1].x + horiz[1].width.saturating_sub(indicator.len() as u16 + 1);
            f.render_widget(
                Paragraph::new(Span::styled(&indicator, Style::default().fg(Color::DarkGray))),
                Rect::new(x, horiz[1].y, indicator.len() as u16, 1),
            );
        } else if !hint.is_empty() {
            let x = horiz[1].x + horiz[1].width.saturating_sub(hint.len() as u16 + 1);
            f.render_widget(
                Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
                Rect::new(x, horiz[1].y, hint.len() as u16, 1),
            );
        }
    } else {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(detail_border_style)
            .title(" Detail ");
        f.render_widget(detail_block, horiz[1]);
    }
}

enum JsonLine {
    KeyValue(String, String),
    Header(String, bool),    // name, is_array
    Close(String),
    ArrayValue(String),
    TableHeader(Vec<String>, Vec<usize>),  // cells, column widths
    TableRow(Vec<String>, Vec<usize>),     // cells, column widths
}

/// Post-process lines to compute aligned column widths for table blocks
fn align_table_columns(lines: &mut Vec<JsonLine>) {
    // Find contiguous blocks of TableHeader + TableRows and compute max col widths
    let mut i = 0;
    while i < lines.len() {
        if matches!(&lines[i], JsonLine::TableHeader(_, _)) {
            let start = i;
            // Collect all column widths in this table block
            let mut col_widths: Vec<usize> = Vec::new();
            let mut j = i;
            while j < lines.len() {
                let cells = match &lines[j] {
                    JsonLine::TableHeader(c, _) => Some(c),
                    JsonLine::TableRow(c, _) => Some(c),
                    _ => None,
                };
                if let Some(cells) = cells {
                    for (col, cell) in cells.iter().enumerate() {
                        let w = cell.chars().count();
                        if col >= col_widths.len() {
                            col_widths.push(w);
                        } else if w > col_widths[col] {
                            col_widths[col] = w;
                        }
                    }
                    j += 1;
                } else {
                    break;
                }
            }
            // Apply widths back to the lines
            for k in start..j {
                match &mut lines[k] {
                    JsonLine::TableHeader(_, widths) |
                    JsonLine::TableRow(_, widths) => {
                        *widths = col_widths.clone();
                    }
                    _ => {}
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn flatten_json_to_lines(value: &serde_json::Value, prefix: &str, depth: usize) -> Vec<JsonLine> {
    let mut lines = Vec::new();
    let indent = "  ".repeat(depth);

    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                if key == "_cont3xt" { continue; }
                let display_key = if prefix.is_empty() {
                    format!("{indent}{key}")
                } else {
                    format!("{indent}{prefix}.{key}")
                };
                match val {
                    serde_json::Value::Object(_) => {
                        lines.push(JsonLine::Header(display_key, false));
                        lines.extend(flatten_json_to_lines(val, "", depth + 1));
                        lines.push(JsonLine::Close(format!("{}}}",  "  ".repeat(depth + 1))));
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.iter().all(|v| !v.is_object() && !v.is_array()) {
                            // Simple array: show inline
                            let vals: Vec<String> = arr.iter().map(|v| format_json_value(v)).collect();
                            lines.push(JsonLine::KeyValue(display_key, vals.join(", ")));
                        } else {
                            lines.push(JsonLine::Header(display_key, true));
                            for item in arr {
                                if item.is_object() {
                                    lines.extend(flatten_json_to_lines(item, "", depth + 1));
                                    lines.push(JsonLine::Close(format!("{}---", "  ".repeat(depth + 1))));
                                } else {
                                    lines.push(JsonLine::ArrayValue(format_json_value(item)));
                                }
                            }
                            lines.push(JsonLine::Close(format!("{}]", "  ".repeat(depth + 1))));
                        }
                    }
                    _ => {
                        lines.push(JsonLine::KeyValue(display_key, format_json_value(val)));
                    }
                }
            }
        }
        _ => {
            lines.push(JsonLine::KeyValue(format!("{indent}{prefix}"), format_json_value(value)));
        }
    }
    lines
}

fn format_json_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

/// Format table cells with aligned column widths
fn format_table_cells(cells: &[String], widths: &[usize], sep: &str) -> String {
    let mut out = String::new();
    for (j, cell) in cells.iter().enumerate() {
        if j > 0 { out.push_str(sep); }
        let w = widths.get(j).copied().unwrap_or(0);
        let char_count = cell.chars().count();
        out.push_str(cell);
        if char_count < w {
            for _ in 0..(w - char_count) {
                out.push(' ');
            }
        }
    }
    out
}

/// Navigate a dotted path like "foo.bar.baz" into a JSON value
fn get_by_path<'a>(data: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    if path.is_empty() {
        return Some(data);
    }
    // Try full key first (handles flattened keys like "source.ip")
    if let Some(v) = data.get(path) {
        return Some(v);
    }
    // Fall back to nested traversal
    let mut current = data;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn defang_string(s: &str) -> String {
    s.replace("http", "hXXp").replace('.', "[.]")
}

fn format_card_value(val: &serde_json::Value, field: &CardField) -> String {
    let raw = match &field.field_type[..] {
        "date" => {
            // Try parsing as ISO date string or epoch ms
            if let Some(s) = val.as_str() {
                s.to_string()
            } else if let Some(n) = val.as_f64() {
                let secs = if n > 1e12 { (n / 1000.0) as i64 } else { n as i64 };
                chrono::DateTime::from_timestamp(secs, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        "ms" => {
            if let Some(n) = val.as_f64() {
                let secs = (n / 1000.0) as i64;
                chrono::DateTime::from_timestamp(secs, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        "seconds" => {
            if let Some(n) = val.as_f64() {
                chrono::DateTime::from_timestamp(n as i64, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        _ => format_json_value(val),
    };
    if field.defang { defang_string(&raw) } else { raw }
}

fn render_card_lines(card: &Cont3xtCard, data: &serde_json::Value, _indicator: &str) -> Vec<JsonLine> {
    let mut lines = Vec::new();

    for field_def in &card.fields {
        let val = get_by_path(data, &field_def.field);

        match &field_def.field_type[..] {
            "table" => {
                lines.push(JsonLine::Header(field_def.label.clone(), true));
                if let Some(arr) = val.and_then(|v| v.as_array()) {
                    // Table header
                    if !field_def.fields.is_empty() {
                        let headers: Vec<String> = field_def.fields.iter()
                            .map(|f| f.label.clone())
                            .collect();
                        lines.push(JsonLine::TableHeader(headers, vec![]));
                    }
                    // Table rows
                    for item in arr {
                        let row_data = if let Some(ref fr) = field_def.field_root {
                            get_by_path(item, fr).unwrap_or(item)
                        } else {
                            item
                        };
                        if field_def.fields.is_empty() {
                            lines.push(JsonLine::ArrayValue(format_json_value(row_data)));
                        } else {
                            let cells: Vec<String> = field_def.fields.iter().map(|sub| {
                                get_by_path(row_data, &sub.field)
                                    .map(|v| format_card_value(v, sub))
                                    .unwrap_or_default()
                            }).collect();
                            lines.push(JsonLine::TableRow(cells, vec![]));
                        }
                    }
                }
                lines.push(JsonLine::Close(format!("  ]")));
            }
            "array" => {
                if let Some(arr) = val.and_then(|v| v.as_array()) {
                    let items: Vec<serde_json::Value> = if let Some(ref fr) = field_def.field_root {
                        arr.iter().filter_map(|item| get_by_path(item, fr).cloned()).collect()
                    } else {
                        arr.clone()
                    };
                    // Filter empty if applicable
                    let items: Vec<&serde_json::Value> = items.iter()
                        .filter(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(true))
                        .collect();
                    if let Some(ref join) = field_def.join {
                        let joined: Vec<String> = items.iter().map(|v| format_json_value(v)).collect();
                        lines.push(JsonLine::KeyValue(field_def.label.clone(), joined.join(join)));
                    } else {
                        lines.push(JsonLine::Header(field_def.label.clone(), true));
                        for item in items {
                            lines.push(JsonLine::ArrayValue(format_json_value(item)));
                        }
                        lines.push(JsonLine::Close(format!("  ]")));
                    }
                } else if let Some(v) = val {
                    lines.push(JsonLine::KeyValue(field_def.label.clone(), format_card_value(v, field_def)));
                }
            }
            "json" => {
                lines.push(JsonLine::Header(field_def.label.clone(), false));
                if let Some(v) = val {
                    let pretty = serde_json::to_string_pretty(v).unwrap_or_else(|_| format_json_value(v));
                    for json_line in pretty.lines() {
                        lines.push(JsonLine::ArrayValue(json_line.to_string()));
                    }
                }
                lines.push(JsonLine::Close(format!("  }}")));
            }
            "dnsRecords" => {
                // DNS records: data is an object with record types as keys
                if let Some(obj) = val.or(Some(data)).and_then(|v| v.as_object()) {
                    for (rtype, rdata) in obj {
                        if rtype == "_cont3xt" { continue; }
                        lines.push(JsonLine::Header(rtype.clone(), false));
                        if let Some(answers) = rdata.get("Answer").and_then(|a| a.as_array()) {
                            for ans in answers {
                                let name = ans.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                let data_str = ans.get("data").and_then(|v| v.as_str()).unwrap_or("");
                                let ttl = ans.get("TTL").and_then(|v| v.as_u64()).unwrap_or(0);
                                lines.push(JsonLine::ArrayValue(format!("{name} → {data_str} (TTL: {ttl})")));
                            }
                        } else {
                            let status = rdata.get("Status").and_then(|v| v.as_u64()).unwrap_or(0);
                            lines.push(JsonLine::ArrayValue(format!("Status: {status}")));
                        }
                    }
                }
            }
            _ => {
                // string, url, externalLink, date, ms, seconds
                if let Some(v) = val {
                    if v.is_null() { continue; }
                    if let Some(s) = v.as_str() {
                        if s.is_empty() { continue; }
                    }
                    lines.push(JsonLine::KeyValue(field_def.label.clone(), format_card_value(v, field_def)));
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(JsonLine::KeyValue("(no card fields matched)".to_string(), String::new()));
    }

    lines
}

fn c3_draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // sub-tab bar
            Constraint::Min(0),   // table
        ])
        .split(area);

    // Sub-tab bar
    let titles: Vec<Line> = C3StatsTab::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.name())))
        .collect();
    let tabs_widget = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .select(C3StatsTab::ALL.iter().position(|&t| t == app.c3_stats_tab).unwrap_or(0));
    f.render_widget(tabs_widget, chunks[0]);

    let columns = app.c3_stats_tab.columns();
    let all_data = app.c3_stats_current_data();

    // Filter
    let mut filtered: Vec<&serde_json::Value> = all_data.iter()
        .filter(|item| {
            app.c3_stats_filter.is_empty()
            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                .to_lowercase().contains(&app.c3_stats_filter.to_lowercase())
        })
        .collect();

    // Sort
    let sort_field = columns.get(app.c3_stats_sort_col).map(|c| c.0).unwrap_or("name");
    filtered.sort_by(|a, b| {
        let cmp = if sort_field == "name" {
            let va = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let vb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            va.to_lowercase().cmp(&vb.to_lowercase())
        } else {
            let va = a.get(sort_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let vb = b.get(sort_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        };
        if app.c3_stats_sort_desc { cmp.reverse() } else { cmp }
    });

    // Build header
    let header_cells: Vec<Cell> = columns.iter().enumerate().map(|(i, &(_, label, _))| {
        let arrow = if i == app.c3_stats_sort_col {
            if app.c3_stats_sort_desc { " ▼" } else { " ▲" }
        } else { "" };
        let text = format!("{label}{arrow}");
        if i == 0 {
            Cell::from(text).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            Cell::from(text).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        }
    }).collect();
    let header = Row::new(header_cells).height(1);

    // Build rows
    let rows: Vec<Row> = filtered.iter().enumerate().map(|(i, item)| {
        let cells: Vec<Cell> = columns.iter().map(|&(field, _, _)| {
            let val = item.get(field);
            let text = match field {
                "name" => val.and_then(|v| v.as_str()).unwrap_or("").to_string(),
                "cacheRecentAvgMS" | "directRecentAvgMS" => {
                    val.and_then(|v| v.as_f64())
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_else(|| "0".to_string())
                }
                _ => {
                    val.and_then(|v| v.as_u64())
                        .map(|v| format_number(v))
                        .unwrap_or_else(|| "0".to_string())
                }
            };
            if field == "name" {
                Cell::from(text)
            } else {
                Cell::from(text).style(Style::default().fg(Color::White))
            }
        }).collect();

        let style = if i == app.c3_stats_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default()
        };
        Row::new(cells).style(style)
    }).collect();

    let widths: Vec<Constraint> = columns.iter().map(|&(_, _, w)| Constraint::Length(w)).collect();

    let filter_info = if app.c3_stats_filtering {
        format!(" /{}█ ", app.c3_stats_filter)
    } else if !app.c3_stats_filter.is_empty() {
        format!(" /{} ", app.c3_stats_filter)
    } else {
        String::new()
    };

    let title = format!(" {} ({}) {}", app.c3_stats_tab.name(), filtered.len(), filter_info);
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default());
    f.render_widget(table, chunks[1]);
}

fn draw_link_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 30u16.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };
    f.render_widget(Clear, popup_area);

    let (indicator, itype) = app.c3_results.get(app.c3_selected)
        .map(|r| (r.indicator.as_str(), r.itype.as_str()))
        .unwrap_or((app.expression.as_str(), app.c3_search_itype.as_str()));
    let title = format!(
        " Links for {} ({}) — {} links ",
        indicator, itype, app.c3_link_flat.len()
    );
    let filter_line = if app.c3_link_popup_filtering {
        format!("Filter: {}█", app.c3_link_popup_filter)
    } else if !app.c3_link_popup_filter.is_empty() {
        format!("Filter: {}", app.c3_link_popup_filter)
    } else {
        String::new()
    };

    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(" / filter  Enter open  q close ").centered())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content_area = if !filter_line.is_empty() {
        let filter_style = if app.c3_link_popup_filtering {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new(filter_line).style(filter_style),
            Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
        );
        Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height.saturating_sub(1) }
    } else {
        inner
    };

    if app.c3_link_flat.is_empty() {
        f.render_widget(
            Paragraph::new("No links available for this indicator type")
                .style(Style::default().fg(Color::DarkGray)),
            content_area,
        );
        return;
    }

    // Scrolling: keep selected in view
    let visible = content_area.height as usize;
    let selected = app.c3_link_popup_selected;
    let scroll_offset = if selected >= visible {
        selected - visible + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut last_group = String::new();
    for (i, (group, name, url)) in app.c3_link_flat.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= visible {
            break;
        }
        // Show group header when group changes
        if *group != last_group {
            if !last_group.is_empty() && lines.len() < visible {
                lines.push(Line::from("")); // spacer between groups
            }
            if lines.len() < visible {
                lines.push(Line::from(Span::styled(
                    format!("── {} ──", group),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
            }
            last_group = group.clone();
        }
        if lines.len() >= visible {
            break;
        }
        let style = if i == selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        // Truncate URL display
        let max_url_len = popup_width as usize - name.len() - 6;
        let url_display = if url.len() > max_url_len {
            format!("{}…", &url[..max_url_len.saturating_sub(1)])
        } else {
            url.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {name}"), style),
            Span::styled(format!("  {url_display}"), if i == selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
        ]));
    }

    f.render_widget(Paragraph::new(lines), content_area);
}

fn draw_integration_popup(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::IntegrationPopupMode;

    let popup_width = 50u16.min(area.width.saturating_sub(4));

    match app.c3_integration_popup_mode {
        IntegrationPopupMode::Views | IntegrationPopupMode::SaveInput | IntegrationPopupMode::ConfirmDelete => {
            // Views list: "Save Current" + saved views
            let list_len = app.c3_views.len() + 1; // +1 for "Save Current"
            let popup_height = (list_len as u16 + 4).min(area.height.saturating_sub(4)).max(6);
            let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
            let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
            let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            f.render_widget(Clear, popup_area);

            let bottom_line = match app.c3_integration_popup_mode {
                IntegrationPopupMode::SaveInput => {
                    let cursor = format!(" Name: {}█ ", app.c3_view_save_name);
                    Line::from(Span::styled(cursor, Style::default().fg(Color::Yellow))).centered()
                }
                IntegrationPopupMode::ConfirmDelete => {
                    let name = app.c3_views.get(app.c3_view_selected.saturating_sub(1))
                        .map(|v| v.name.as_str()).unwrap_or("?");
                    Line::from(Span::styled(
                        format!(" Delete '{name}'? (y/n) "),
                        Style::default().fg(Color::Red)
                    )).centered()
                }
                _ => Line::from(" Enter:load  x:delete  Esc:back ").centered(),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title(" Views ")
                .title_bottom(bottom_line);
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let visible_height = inner.height as usize;
            let scroll_offset = if list_len > visible_height {
                let sel = app.c3_view_selected;
                if sel >= visible_height { sel - visible_height + 1 } else { 0 }
            } else { 0 };

            for i in scroll_offset..(scroll_offset + visible_height).min(list_len) {
                let y = inner.y + (i - scroll_offset) as u16;
                let is_selected = i == app.c3_view_selected;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Magenta)
                } else {
                    Style::default().fg(Color::White)
                };

                if i == 0 {
                    // "Save Current" option
                    let enabled = app.c3_integrations.len() - app.c3_disabled_integrations.len();
                    let label = format!(" 💾 Save Current ({enabled} integrations)");
                    f.render_widget(Paragraph::new(Span::styled(label, style)), Rect::new(inner.x, y, inner.width, 1));
                } else {
                    let view = &app.c3_views[i - 1];
                    let count = view.integrations.len();
                    let shared = if !view.editable { " 🔗" } else { "" };
                    let label = format!(" {} ({count}){shared}", view.name);
                    f.render_widget(Paragraph::new(Span::styled(label, style)), Rect::new(inner.x, y, inner.width, 1));
                }
            }
        }
        IntegrationPopupMode::Integrations => {
            let filtered: Vec<(usize, &crate::api::Cont3xtIntegration)> = app.c3_integrations.iter().enumerate()
                .filter(|(_, int)| {
                    app.c3_integration_popup_filter.is_empty()
                    || int.name.to_lowercase().contains(&app.c3_integration_popup_filter.to_lowercase())
                })
                .collect();

            let disabled_count = app.c3_disabled_integrations.len();
            let total = app.c3_integrations.len();
            let enabled = total - disabled_count;

            let popup_height = (filtered.len() as u16 + 5).min(area.height.saturating_sub(4));
            let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
            let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
            let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            f.render_widget(Clear, popup_area);

            let bottom_line = if app.c3_integration_popup_filtering {
                let cursor = format!(" /{}█ ", app.c3_integration_popup_filter);
                Line::from(Span::styled(cursor, Style::default().fg(Color::Yellow))).centered()
            } else if !app.c3_integration_popup_filter.is_empty() {
                Line::from(format!(" /{} │ Spc:toggle a:all n:none !:inv v:views ", app.c3_integration_popup_filter)).centered()
            } else {
                Line::from(" Spc:toggle a:all n:none !:inv /:filter v:views ").centered()
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" Integrations ({enabled}/{total}) "))
                .title_bottom(bottom_line);
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let visible_height = inner.height as usize;
            let scroll_offset = if filtered.len() > visible_height {
                let sel = app.c3_integration_popup_selected;
                if sel >= visible_height { sel - visible_height + 1 } else { 0 }
            } else { 0 };

            for (i, (_, integ)) in filtered.iter().enumerate().skip(scroll_offset).take(visible_height) {
                let y = inner.y + (i - scroll_offset) as u16;
                let is_selected = i == app.c3_integration_popup_selected;
                let is_disabled = app.c3_disabled_integrations.contains(&integ.name);

                let check = if is_disabled { "✗" } else { "✓" };
                let check_color = if is_disabled { Color::Red } else { Color::Green };

                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                if is_selected {
                    let label = format!(" {check} {}", integ.name);
                    f.render_widget(Paragraph::new(Span::styled(label, style)), Rect::new(inner.x, y, inner.width, 1));
                } else {
                    let line = Line::from(vec![
                        Span::styled(format!(" {check} "), Style::default().fg(check_color)),
                        Span::styled(integ.name.clone(), style),
                    ]);
                    f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                }
            }
        }
    }
}

fn draw_placeholder(f: &mut Frame, app: &mut App, ) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(status_h),
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    arkime::draw_under_construction(f, app, chunks[1]);
    arkime::draw_owl(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);
}

fn status_bar_height(app: &App) -> u16 {
    let lines = app.status_msg.chars().filter(|&c| c == '\n').count() + 1;
    lines as u16
}

fn draw_default_layout(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let mut constraints = vec![
        Constraint::Length(3), // tabs
        Constraint::Length(3), // toolbar: time range + expression
    ];
    if app.vr_graph_size.is_visible() && app.active_tab == Tab::Sessions {
        constraints.push(Constraint::Length(app.vr_graph_size.height())); // graph
    }
    constraints.push(Constraint::Min(0));   // content
    constraints.push(Constraint::Length(status_h)); // status bar

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut idx = 0;
    draw_tabs(f, app, chunks[idx]); idx += 1;
    draw_toolbar(f, app, chunks[idx]); idx += 1;

    if app.vr_graph_size.is_visible() && app.active_tab == Tab::Sessions {
        draw_graph(f, app, chunks[idx]); idx += 1;
    }

    match app.active_tab {
        Tab::Sessions => sessions::draw_sessions(f, app, chunks[idx]),
        Tab::Arkime => arkime::draw_arkime(f, app, chunks[idx]),
        Tab::Settings => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(app.active_tab.name());
            f.render_widget(block, chunks[idx]);
            arkime::draw_under_construction(f, app, chunks[idx]);
            arkime::draw_owl(f, app, chunks[idx]);
        }
        _ => {}
    }
    idx += 1;

    draw_status_bar(f, app, chunks[idx]);
}

fn draw_stats_layout(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(3), // stats sub-tabs + filter
            Constraint::Min(0),   // content
            Constraint::Length(status_h), // status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    stats::draw_stats_toolbar(f, app, chunks[1]);
    stats::draw_stats(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tabs_list = app.app_mode.tabs();
    let titles: Vec<Line> = tabs_list
        .iter()
        .map(|t| Line::from(t.name()))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(format!(" Alkeme ({}) ", app.app_mode.label())))
        .select(tabs_list.iter().position(|&t| t == app.active_tab).unwrap_or(0))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn draw_toolbar(f: &mut Frame, app: &App, area: Rect) {
    let toolbar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // time range
            Constraint::Min(0),    // expression
        ])
        .split(area);

    // Time range: show ◄ prev | SELECTED | next ►
    let selected_idx = TimeRange::ALL.iter().position(|&t| t == app.time_range).unwrap_or(0);
    let mut spans: Vec<Span> = Vec::new();
    if selected_idx > 0 {
        spans.push(Span::styled("◄ ", Style::default().fg(Color::DarkGray)));
    } else {
        spans.push(Span::styled("  ", Style::default()));
    }
    spans.push(Span::styled(
        format!(" {} ", app.time_range.label()),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));
    if selected_idx < TimeRange::ALL.len() - 1 {
        spans.push(Span::styled(" ►", Style::default().fg(Color::DarkGray)));
    }
    let time_widget = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Time t/T "));
    f.render_widget(time_widget, toolbar_chunks[0]);

    // Expression input
    let expr_display = if app.input_mode == InputMode::Expression {
        &app.expression_edit
    } else {
        &app.expression
    };
    let expr_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let expr_widget = Paragraph::new(Span::styled(expr_display.as_str(), expr_style))
        .block(Block::default().borders(Borders::ALL).title(" Expression (/) "));
    f.render_widget(expr_widget, toolbar_chunks[1]);

    // Show cursor in expression field when editing
    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            toolbar_chunks[1].x + app.expression_cursor as u16 + 1,
            toolbar_chunks[1].y + 1,
        ));
    }
}


fn draw_graph(f: &mut Frame, app: &App, area: Rect) {
    let graph = match &app.vr_graph_data {
        Some(g) => g,
        None => {
            let block = Block::default().borders(Borders::ALL)
                .title(format!(" {} (loading...) g/G ", app.vr_graph_type.label()));
            f.render_widget(block, area);
            return;
        }
    };

    let is_split = app.vr_graph_type != GraphType::Sessions;

    let (src_histo, dst_histo, title) = match app.vr_graph_type {
        GraphType::Sessions => (&graph.sessions_histo, &graph.sessions_histo, "Sessions"),
        GraphType::Packets => (&graph.src_packets_histo, &graph.dst_packets_histo, "Packets"),
        GraphType::Bytes => (&graph.src_bytes_histo, &graph.dst_bytes_histo, "Bytes"),
    };

    if src_histo.is_empty() {
        let block = Block::default().borders(Borders::ALL)
            .title(format!(" {title} (no data) g/G "));
        f.render_widget(block, area);
        return;
    }

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;
    if inner_width == 0 || inner_height == 0 {
        return;
    }

    // Map sparse timestamp data into pixel columns using first/last timestamps
    let first_ts = src_histo.first().map(|(t, _)| *t).unwrap_or(0.0);
    let last_ts = src_histo.last().map(|(t, _)| *t).unwrap_or(0.0);
    let ts_range = last_ts - first_ts;

    let src_buckets = spread_to_columns(src_histo, first_ts, ts_range, inner_width);
    let dst_buckets = if is_split {
        spread_to_columns(dst_histo, first_ts, ts_range, inner_width)
    } else {
        vec![0u64; inner_width]
    };

    // For split view, max is max of either; for sessions, just src
    let max_val = if is_split {
        src_buckets.iter().chain(dst_buckets.iter()).copied().max().unwrap_or(1).max(1)
    } else {
        src_buckets.iter().copied().max().unwrap_or(1).max(1)
    };

    // Compute human-readable bar duration
    let bar_dur = if inner_width > 1 && ts_range > 0.0 {
        format_duration_ms(ts_range / inner_width as f64)
    } else {
        "n/a".into()
    };

    let start_label = format_epoch_short(first_ts);
    let stop_label = format_epoch_short(last_ts);

    // Render into a buffer manually using block characters
    let block = Block::default().borders(Borders::ALL)
        .title(if is_split {
            format!(" {title} (max: {max_val}, {bar_dur}/bar) Src=cyan Dst=green g/G ")
        } else {
            format!(" {title} (max: {max_val}, {bar_dur}/bar) g/G ")
        })
        .title_bottom(Line::from(vec![
            Span::raw(format!(" {start_label} ")),
        ]).alignment(Alignment::Left))
        .title_bottom(Line::from(vec![
            Span::raw(format!(" {stop_label} ")),
        ]).alignment(Alignment::Right));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Draw columns using half-block characters for resolution
    let src_color = Color::Cyan;
    let dst_color = Color::Green;
    let buf = f.buffer_mut();

    for col in 0..inner_width.min(inner.width as usize) {
        let x = inner.x + col as u16;

        if is_split {
            let src_h = (src_buckets[col] as f64 / max_val as f64 * inner_height as f64).round() as usize;
            let dst_h = (dst_buckets[col] as f64 / max_val as f64 * inner_height as f64).round() as usize;

            for row in 0..inner_height {
                let y = inner.y + (inner_height - 1 - row) as u16;
                if y >= inner.y + inner.height { continue; }
                if row < src_h && row < dst_h {
                    buf[(x, y)].set_char('▐').set_style(Style::default().fg(dst_color).bg(src_color));
                } else if row < src_h {
                    buf[(x, y)].set_char('█').set_style(Style::default().fg(src_color));
                } else if row < dst_h {
                    buf[(x, y)].set_char('█').set_style(Style::default().fg(dst_color));
                }
            }
        } else {
            let h = (src_buckets[col] as f64 / max_val as f64 * inner_height as f64).round() as usize;
            for row in 0..h.min(inner_height) {
                let y = inner.y + (inner_height - 1 - row) as u16;
                if y >= inner.y + inner.height { continue; }
                buf[(x, y)].set_char('█').set_style(Style::default().fg(src_color));
            }
        }
    }
}
fn spread_to_columns(histo: &[(f64, f64)], first_ts: f64, ts_range: f64, width: usize) -> Vec<u64> {
    let mut buckets = vec![0u64; width];
    if ts_range <= 0.0 || width == 0 {
        if !histo.is_empty() {
            let sum: f64 = histo.iter().map(|(_, v)| v).sum();
            buckets[width / 2] = sum as u64;
        }
        return buckets;
    }
    for &(ts, val) in histo {
        let col = ((ts - first_ts) / ts_range * (width - 1) as f64).round() as usize;
        let col = col.min(width - 1);
        buckets[col] += val as u64;
    }
    buckets
}

fn format_duration_ms(ms: f64) -> String {
    let secs = ms / 1000.0;
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m", secs / 60.0)
    } else if secs < 86400.0 {
        format!("{:.1}h", secs / 3600.0)
    } else if secs < 86400.0 * 7.0 {
        format!("{:.1}d", secs / 86400.0)
    } else if secs < 86400.0 * 30.0 {
        format!("{:.1}w", secs / (86400.0 * 7.0))
    } else {
        format!("{:.1}mo", secs / (86400.0 * 30.0))
    }
}

pub(super) fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = app.status_msg.split('\n')
        .map(|l| Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::raw(l.to_string()),
        ]))
        .collect();
    if let Some(last) = lines.last_mut() {
        last.spans.push(Span::styled(
            "  Tab/Shift+Tab: switch tabs | j/k: navigate | Enter: open | r: refresh | q: quit ",
            Style::default().fg(Color::DarkGray),
        ));
    }
    let status = Paragraph::new(lines)
        .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(status, area);
}
