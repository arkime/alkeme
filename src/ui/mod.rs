mod sessions;
mod stats;
mod arkime;
mod popups;
mod help;
mod cont3xt;
mod cont3xt_settings;
mod parliament;
mod wise;
mod files;

// Re-export app types for sub-modules via `use super::*`
#[allow(unused_imports)]
use crate::app::{App, AppMode, ActionTarget, C3StatsTab, C3_HISTORY_COLUMNS, ColumnEditorMode, Cont3xtFocus, DetailActionMenu, GraphType, InputMode, LayoutPopupMode, LineMode, PlIssueSort, SessionView, StatsColumnDef, StatsColumnEditorItem, StatsFormat, StatsTab, StatsView, SummaryMetric, SummarySort, Tab, TimeRange, ViewPopupMode, WsStatsTab, is_hidden_detail_field};

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

pub(super) fn highlight_filter_spans<'a>(text: &str, filter_lower: &str, base_style: Style) -> Vec<Span<'a>> {
    if filter_lower.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let text_lower = text.to_lowercase();
    let highlight_style = base_style.fg(Color::LightRed);
    let mut spans = Vec::new();
    let mut last_end = 0;
    for (start, _) in text_lower.match_indices(filter_lower) {
        if start > last_end {
            spans.push(Span::styled(text[last_end..start].to_string(), base_style));
        }
        spans.push(Span::styled(text[start..start + filter_lower.len()].to_string(), highlight_style));
        last_end = start + filter_lower.len();
    }
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }
    spans
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
        AppMode::Cont3xt => cont3xt::draw_cont3xt(f, app),
        AppMode::Parliament => parliament::draw_parliament(f, app),
        AppMode::Wise => wise::draw_wise(f, app),
    }

    // Common overlays (shared across modes)
    if app.confirm_dialog.is_some() {
        popups::draw_confirm_dialog(f, app, f.area());
    }
    if app.show_help {
        help::draw_help(f, app, f.area());
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
        Tab::Files => draw_files_layout(f, app),
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
    if app.viewer.detail_action_menu.is_some() && app.active_tab == Tab::Arkime {
        sessions::draw_detail_action_menu(f, app, f.area());
    }
    if app.viewer.packets_view.is_some() {
        popups::draw_packets(f, app, f.area());
    }
    if app.viewer.show_column_editor {
        popups::draw_column_editor(f, app, f.area());
    }
    if app.viewer.show_layout_popup {
        popups::draw_layout_popup(f, app, f.area());
    }
    if app.viewer.stats_show_column_editor {
        popups::draw_stats_column_editor(f, app, f.area());
    }
    if app.viewer.stats_show_layout_popup {
        popups::draw_stats_layout_popup(f, app, f.area());
    }
    if app.viewer.files_show_column_editor {
        popups::draw_files_column_editor(f, app, f.area());
    }
    if app.viewer.files_show_layout_popup {
        popups::draw_files_layout_popup(f, app, f.area());
    }
    if app.viewer.show_view_popup {
        popups::draw_view_popup(f, app, f.area());
    }
}


pub(super) fn status_bar_height(app: &App) -> u16 {
    let lines = app.status_msg.chars().filter(|&c| c == '\n').count() + 1;
    lines as u16
}

fn draw_default_layout(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let mut constraints = vec![
        Constraint::Length(3), // tabs
        Constraint::Length(3), // toolbar: time range + expression
    ];
    if app.viewer.graph_size.is_visible() && app.active_tab == Tab::Sessions {
        constraints.push(Constraint::Length(app.viewer.graph_size.height())); // graph
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

    if app.viewer.graph_size.is_visible() && app.active_tab == Tab::Sessions {
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

fn draw_files_layout(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(3), // filter + pagination bar
            Constraint::Min(0),   // content
            Constraint::Length(status_h), // status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    files::draw_files_toolbar(f, app, chunks[1]);
    files::draw_files(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

pub(super) fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
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
    let selected_idx = app.time_ranges.iter().position(|t| t == &app.time_range).unwrap_or(0);
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
    if selected_idx < app.time_ranges.len() - 1 {
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
    let is_editing = app.input_mode == InputMode::Expression;
    render_text_input(f, expr_display, app.expression_cursor, is_editing, " Expression (/) ", toolbar_chunks[1]);
}


fn draw_graph(f: &mut Frame, app: &App, area: Rect) {
    let graph = match &app.viewer.graph_data {
        Some(g) => g,
        None => {
            let block = Block::default().borders(Borders::ALL)
                .title(format!(" {} (loading...) g/G ", app.viewer.graph_type.label()));
            f.render_widget(block, area);
            return;
        }
    };

    let is_split = app.viewer.graph_type != GraphType::Sessions;

    let (src_histo, dst_histo, title) = match app.viewer.graph_type {
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

/// Center a popup of given width/height within an area
pub(super) fn center_popup(width: u16, height: u16, area: Rect) -> Rect {
    Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}

/// Style a table header cell with sort indicator
pub(super) fn sort_header_style(is_sorted: bool) -> Style {
    if is_sorted {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    }
}

/// Format a column label with sort arrow if this column is sorted
pub(super) fn sort_header_label(label: &str, is_sorted: bool, is_desc: bool) -> String {
    if is_sorted {
        let arrow = if is_desc { "▼" } else { "▲" };
        format!("{label}{arrow}")
    } else {
        label.to_string()
    }
}

/// Render a text input field with scroll and cursor positioning
pub(super) fn render_text_input(
    f: &mut Frame,
    text: &str,
    cursor: usize,
    is_editing: bool,
    title: &str,
    area: Rect,
) {
    let style = if is_editing {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let inner_width = area.width.saturating_sub(2) as usize;
    let scroll = if is_editing && cursor > inner_width {
        (cursor - inner_width) as u16
    } else {
        0
    };
    let widget = Paragraph::new(Span::styled(text, style))
        .scroll((0, scroll))
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(widget, area);

    if is_editing {
        f.set_cursor_position((
            area.x + (cursor as u16 - scroll) + 1,
            area.y + 1,
        ));
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

/// Parse a hex color string like "#ff0000" or "ff0000" into a ratatui Color::Rgb.
pub(super) fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 { return None; }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

pub(super) fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
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
