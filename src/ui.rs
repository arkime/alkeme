use crate::app::{App, ColumnEditorMode, DetailActionMenu, GraphType, InputMode, LayoutPopupMode, SessionView, StatsTab, StatsView, SummaryMetric, SummarySort, Tab, TimeRange, is_hidden_detail_field};
use chrono::{DateTime, Local};
use ratatui::{
    prelude::*,
    widgets::*,
};

fn format_epoch(val: &serde_json::Value, _field_type: &str) -> String {
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

fn format_epoch_short(ms: f64) -> String {
    let secs = (ms / 1000.0) as i64;
    if let Some(dt) = DateTime::from_timestamp(secs, 0) {
        let local: DateTime<Local> = dt.into();
        return local.format("%Y/%m/%d %H:%M").to_string();
    }
    "-".into()
}

fn format_epoch_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let millis = (ms % 1000) as u32;
    if let Some(dt) = DateTime::from_timestamp(secs, millis * 1_000_000) {
        let local: DateTime<Local> = dt.into();
        return local.format("%H:%M:%S%.3f").to_string();
    }
    "-".into()
}

fn format_human_bytes(bytes: f64) -> String {
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

fn format_human_megabytes(mb: f64) -> String {
    format_human_bytes(mb * 1024.0 * 1024.0)
}

fn format_epoch_secs(val: &serde_json::Value) -> String {
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

fn parse_size_string(s: &str) -> Option<f64> {
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

fn ip_protocol_str(val: &serde_json::Value) -> String {
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
    match app.active_tab {
        Tab::Stats => draw_stats_layout(f, app),
        _ => draw_default_layout(f, app),
    }

    if app.action_menu.is_some() {
        draw_action_menu(f, app, f.area());
    }
    if app.input_mode == InputMode::ActionPrompt {
        draw_action_prompt(f, app, f.area());
    }
    if app.input_mode == InputMode::FieldSelector {
        draw_field_selector(f, app, f.area());
    }
    if app.detail_action_menu.is_some() && app.active_tab == Tab::Arkime {
        draw_detail_action_menu(f, app, f.area());
    }
    if app.packets_view.is_some() {
        draw_packets(f, app, f.area());
    }
    if app.show_column_editor {
        draw_column_editor(f, app, f.area());
    }
    if app.show_layout_popup {
        draw_layout_popup(f, app, f.area());
    }
    if app.show_view_popup {
        draw_view_popup(f, app, f.area());
    }
    if app.show_help {
        draw_help(f, app, f.area());
    }
    if app.show_debug {
        draw_debug(f, app, f.area());
    }
    if app.show_loading {
        draw_loading(f, app, f.area());
    }
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
    if app.graph_size.is_visible() && app.active_tab == Tab::Sessions {
        constraints.push(Constraint::Length(app.graph_size.height())); // graph
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

    if app.graph_size.is_visible() && app.active_tab == Tab::Sessions {
        draw_graph(f, app, chunks[idx]); idx += 1;
    }

    match app.active_tab {
        Tab::Sessions => draw_sessions(f, app, chunks[idx]),
        Tab::Arkime => draw_arkime(f, app, chunks[idx]),
        Tab::Settings => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(app.active_tab.name());
            f.render_widget(block, chunks[idx]);
            draw_under_construction(f, app, chunks[idx]);
            draw_owl(f, app, chunks[idx]);
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
    draw_stats_toolbar(f, app, chunks[1]);
    draw_stats(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(t.name()))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Alkeme "))
        .select(Tab::ALL.iter().position(|&t| t == app.active_tab).unwrap_or(0))
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
    let graph = match &app.graph_data {
        Some(g) => g,
        None => {
            let block = Block::default().borders(Borders::ALL)
                .title(format!(" {} (loading...) g/G ", app.graph_type.label()));
            f.render_widget(block, area);
            return;
        }
    };

    let is_split = app.graph_type != GraphType::Sessions;

    let (src_histo, dst_histo, title) = match app.graph_type {
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

fn draw_sessions(f: &mut Frame, app: &mut App, area: Rect) {
    draw_session_list(f, app, area);
    if app.session_view == SessionView::Detail {
        draw_session_detail(f, app, area);
        if app.detail_action_menu.is_some() {
            draw_detail_action_menu(f, app, area);
        }
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect) {
    // header row + borders = 3 lines overhead
    app.visible_rows = area.height.saturating_sub(3) as usize;
    let header_cells = app.columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let label = if i == app.sort_column {
                let arrow = if app.sort_desc { "▼" } else { "▲" };
                format!("{}{arrow}", col.label)
            } else {
                col.label.clone()
            };
            let style = if i == app.sort_column {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            };
            Cell::from(label).style(style)
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .map(|session| {
            let cells = app.columns.iter().enumerate().map(|(col_idx, col)| {
                let val = session.get(&col.field).unwrap_or(&serde_json::Value::Null);
                let text = if col.field == "ipProtocol" && col_idx == 0 {
                    ip_protocol_str(val)
                } else if let Some(field_type) = app.date_fields.get(col.field.as_str()) {
                    format_epoch(val, field_type)
                } else {
                    match val {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Array(arr) => arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(","),
                        serde_json::Value::Null => "-".into(),
                        other => other.to_string(),
                    }
                };
                Cell::from(text)
            });
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = app.columns.iter()
        .map(|col| Constraint::Length(col.width))
        .collect();

    let end = (app.page_start + app.sessions.len() as u64).min(app.sessions_filtered);
    let view_label = if let Some(ref v) = app.active_view_name {
        format!(" [view: {}]", v)
    } else {
        String::new()
    };
    let page_label = if app.sessions_filtered > 0 {
        format!(" Sessions{} [{}-{} of {}] ◄ ► ", view_label, app.page_start + 1, end, app.sessions_filtered)
    } else {
        format!(" Sessions{} [0] ", view_label)
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(page_label),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_session_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let detail = match &app.session_detail {
        Some(d) => d,
        None => return,
    };

    // Centered overlay: 80% width, 80% height
    let popup_width = (area.width as f32 * 0.8) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the popup area
    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    let filter_lower = detail.filter.to_lowercase();

    if let Some(obj) = detail.data.as_object() {
        let mut keys: Vec<&String> = obj.keys()
            .filter(|k| !is_hidden_detail_field(k))
            .filter(|k| {
                if filter_lower.is_empty() {
                    return true;
                }
                let friendly = app.field_friendly_map.get(k.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(k.as_str());
                k.to_lowercase().contains(&filter_lower)
                    || friendly.to_lowercase().contains(&filter_lower)
            })
            .collect();
        keys.sort();
        for (i, db_field) in keys.iter().enumerate() {
            let val = &obj[*db_field];
            let val_str = if let Some(field_type) = app.date_fields.get(db_field.as_str()) {
                format_epoch(val, field_type)
            } else {
                match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(arr) => {
                        arr.iter()
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                    serde_json::Value::Null => "-".into(),
                    other => other.to_string(),
                }
            };
            let display_name = app.field_friendly_map.get(db_field.as_str())
                .map(|s| s.as_str())
                .unwrap_or(db_field.as_str());
            let is_selected = i == detail.selected;
            let key_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            let val_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{display_name:>30}: "),
                    key_style,
                ),
                Span::styled(val_str, val_style),
            ]));
        }
    }

    // Auto-scroll to keep selected row visible
    let visible_rows = popup_height.saturating_sub(2) as usize;
    let selected = detail.selected;
    let mut scroll = detail.scroll;
    let detail_filter = detail.filter.clone();
    if visible_rows > 0 {
        if selected < scroll as usize {
            scroll = selected as u16;
        } else if selected >= scroll as usize + visible_rows {
            scroll = (selected - visible_rows + 1) as u16;
        }
    }
    // Write back the computed scroll
    if let Some(ref mut d) = app.session_detail {
        d.scroll = scroll;
    }

    let title = if !detail_filter.is_empty() {
        format!(" Session Detail [filter: {}] ", detail_filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        " Session Detail [filter: ] ".to_string()
    } else {
        " Session Detail (↑↓ navigate, Enter add to expression, / filter, a action menu, Esc close) ".to_string()
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title.as_str()),
        )
        .scroll((scroll, 0));

    f.render_widget(paragraph, popup_area);
}

fn draw_detail_action_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu = match &app.detail_action_menu {
        Some(m) => m,
        None => return,
    };

    if let Some(ref values) = menu.values {
        // Value selection sub-menu
        let popup_width = 40u16;
        let popup_height = (values.len() as u16) + 3;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            format!(" {} ", menu.display),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));

        for (i, val) in values.iter().enumerate() {
            let is_selected = i == menu.value_selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "▸ " } else { "  " };
            lines.push(Line::from(Span::styled(
                format!("{prefix}{val}"),
                style,
            )));
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Select Value "),
            );
        f.render_widget(paragraph, popup_area);
        return;
    }

    let popup_width = 40u16;
    let popup_height = (DetailActionMenu::OPTIONS.len() as u16) + 4; // borders + title + field line
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {} = {} ", menu.display, menu.value),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));

    for (i, option) in DetailActionMenu::OPTIONS.iter().enumerate() {
        let is_selected = i == menu.selected;
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{option}"),
            style,
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Add to Expression "),
        );
    f.render_widget(paragraph, popup_area);
}

fn format_number(n: u64) -> String {
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

fn draw_arkime(f: &mut Frame, app: &mut App, area: Rect) {
    let arkime_title = if let Some(ref v) = app.active_view_name {
        format!(" Arkime Summary [view: {}] ", v)
    } else {
        " Arkime Summary ".to_string()
    };
    if app.summary_field.is_empty() {
        // Show prompt to select a field
        let block = Block::default()
            .borders(Borders::ALL)
            .title(arkime_title);
        let text = Paragraph::new(Line::from(vec![
            Span::raw("Press "),
            Span::styled("f", Style::default().fg(Color::Yellow)),
            Span::raw(" to select a field"),
        ]))
        .alignment(Alignment::Center)
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // bar chart
            Constraint::Min(0),    // table
        ])
        .split(area);

    draw_summary_bar_chart(f, app, chunks[0]);
    draw_summary_table(f, app, chunks[1]);
}

fn draw_summary_bar_chart(f: &mut Frame, app: &App, area: Rect) {
    let metric = app.summary_metric;
    let data: Vec<(&str, u64)> = app.summary_data.iter()
        .map(|item| {
            let label = item.item.as_str().unwrap_or("");
            let val = match metric {
                SummaryMetric::Sessions => item.sessions,
                SummaryMetric::Packets => item.packets,
                SummaryMetric::Bytes => item.bytes,
            };
            (label, val)
        })
        .collect();

    if data.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} - {} (no data) [G]raph type ", app.summary_field, metric.label()));
        f.render_widget(block, area);
        return;
    }

    let bar_width = if data.is_empty() { 1 } else {
        let w = (area.width.saturating_sub(2)) / data.len() as u16;
        w.clamp(1, 12)
    };

    let bars: Vec<Bar> = data.iter()
        .map(|(label, val)| {
            let truncated: String = if label.len() > bar_width as usize {
                label.chars().take(bar_width as usize).collect()
            } else {
                label.to_string()
            };
            Bar::default()
                .value(*val)
                .label(Line::from(truncated))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let chart = BarChart::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} - {} [G]raph type ", app.summary_field, metric.label())))
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    f.render_widget(chart, area);
}

fn draw_summary_table(f: &mut Frame, app: &mut App, area: Rect) {
    let arrow = if app.summary_sort_desc { "▼" } else { "▲" };
    let sort_indicator = |sort: SummarySort, label: &str| -> String {
        if app.summary_sort == sort { format!("{label} {arrow}") } else { label.to_string() }
    };

    let header = Row::new(vec![
        Cell::from(sort_indicator(SummarySort::Value, "Value")).style(Style::default().fg(Color::Yellow)),
        Cell::from(sort_indicator(SummarySort::Sessions, "Sessions")).style(Style::default().fg(Color::Yellow)),
        Cell::from(sort_indicator(SummarySort::Packets, "Packets")).style(Style::default().fg(Color::Yellow)),
        Cell::from(sort_indicator(SummarySort::Bytes, "Bytes")).style(Style::default().fg(Color::Yellow)),
    ])
    .height(1)
    .bottom_margin(0);

    let rows: Vec<Row> = app.summary_data.iter().map(|item| {
        let label = match &item.item {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Row::new(vec![
            Cell::from(label),
            Cell::from(format_number(item.sessions)).style(Style::default().fg(Color::White)),
            Cell::from(format_number(item.packets)).style(Style::default().fg(Color::White)),
            Cell::from(format_human_bytes(item.bytes as f64)).style(Style::default().fg(Color::White)),
        ])
    }).collect();

    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(" {} [f]ield [s]ort [S]ort dir ", app.summary_field)))
    .row_highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.summary_table_state);
}

fn draw_field_selector(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_area = Rect::new(
        area.x + (area.width.saturating_sub(popup_width)) / 2,
        area.y + (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // filter input
            Constraint::Min(0),   // field list
        ])
        .split(popup_area);

    // Filter input
    let filter_style = Style::default().fg(Color::Yellow);
    let filter_display = if app.field_filter.is_empty() {
        "Type to filter fields...".to_string()
    } else {
        app.field_filter.clone()
    };
    let filter_input = Paragraph::new(Span::styled(&filter_display,
        if app.field_filter.is_empty() { Style::default().fg(Color::DarkGray) } else { filter_style }))
        .block(Block::default().borders(Borders::ALL).title(" Select Field "));
    f.render_widget(filter_input, chunks[0]);

    // Field list
    let filtered = app.filtered_fields();
    let items: Vec<ListItem> = filtered.iter().enumerate().map(|(i, field)| {
        let style = if i == app.field_filter_selected {
            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
        } else {
            Style::default()
        };
        let line = if field.friendly_name.is_empty() {
            field.exp.clone()
        } else {
            format!("{} ({})", field.exp, field.friendly_name)
        };
        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(list, chunks[1]);
}

fn draw_under_construction(f: &mut Frame, app: &App, area: Rect) {
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    if inner.width < 40 || inner.height < 12 {
        return;
    }

    // Blinking color based on tick
    let tick = (app.anim_start.elapsed().as_millis() / 300) as usize;
    let colors = [Color::Yellow, Color::Red, Color::Magenta, Color::Cyan, Color::Green];

    // Animated construction barricade
    let shift = tick % 6;
    let barricade: String = (0..inner.width as usize)
        .map(|i| if (i + shift) % 6 < 3 { '▓' } else { '░' })
        .collect();

    let cy = inner.y + 1;
    let buf = f.buffer_mut();

    // Top barricade
    for (i, ch) in barricade.chars().enumerate() {
        let x = inner.x + i as u16;
        if x < inner.x + inner.width {
            buf[(x, cy)].set_char(ch).set_style(Style::default().fg(Color::Yellow).bg(Color::Black));
        }
    }

    let banner = [
        " ██╗   ██╗███╗   ██╗██████╗ ███████╗██████╗  ",
        " ██║   ██║████╗  ██║██╔══██╗██╔════╝██╔══██╗ ",
        " ██║   ██║██╔██╗ ██║██║  ██║█████╗  ██████╔╝ ",
        " ██║   ██║██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗ ",
        " ╚██████╔╝██║ ╚████║██████╔╝███████╗██║  ██║ ",
        "  ╚═════╝ ╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝ ",
    ];

    let construction = "★ ☆ CONSTRUCTION ☆ ★";
    let visitor_line = "You are visitor #000,001";
    let best_viewed = "Best viewed in alkeme TUI";

    // Draw banner
    let banner_y = cy + 2;
    for (row, line) in banner.iter().enumerate() {
        let y = banner_y + row as u16;
        if y >= inner.y + inner.height { break; }
        let bx = inner.x + (inner.width.saturating_sub(line.chars().count() as u16)) / 2;
        let color = colors[(row + tick) % colors.len()];
        for (col, ch) in line.chars().enumerate() {
            let x = bx + col as u16;
            if x < inner.x + inner.width && ch != ' ' {
                buf[(x, y)].set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }

    // "CONSTRUCTION" line
    let con_y = banner_y + banner.len() as u16 + 1;
    if con_y < inner.y + inner.height {
        let con_x = inner.x + (inner.width.saturating_sub(construction.len() as u16)) / 2;
        let blink_color = colors[tick % colors.len()];
        for (col, ch) in construction.chars().enumerate() {
            let x = con_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, con_y)].set_char(ch).set_style(
                    Style::default().fg(blink_color).add_modifier(Modifier::BOLD)
                );
            }
        }
    }

    // Visitor counter
    let vis_y = con_y + 2;
    if vis_y < inner.y + inner.height {
        let vis_x = inner.x + (inner.width.saturating_sub(visitor_line.len() as u16)) / 2;
        for (col, ch) in visitor_line.chars().enumerate() {
            let x = vis_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, vis_y)].set_char(ch).set_style(Style::default().fg(Color::Green));
            }
        }
    }

    // Best viewed line
    let bv_y = vis_y + 1;
    if bv_y < inner.y + inner.height {
        let bv_x = inner.x + (inner.width.saturating_sub(best_viewed.len() as u16)) / 2;
        for (col, ch) in best_viewed.chars().enumerate() {
            let x = bv_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, bv_y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }

    // Bottom barricade
    let bot_y = bv_y + 2;
    if bot_y < inner.y + inner.height {
        for (i, ch) in barricade.chars().enumerate() {
            let x = inner.x + i as u16;
            if x < inner.x + inner.width {
                buf[(x, bot_y)].set_char(ch).set_style(Style::default().fg(Color::Yellow).bg(Color::Black));
            }
        }
    }
}

fn draw_owl(f: &mut Frame, app: &mut App, area: Rect) {
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    if inner.width < 12 || inner.height < 6 {
        return;
    }

    // Owl walking frames (facing right and left)
    let owl_right: [&[&str]; 2] = [
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  /)  )  ",
            " / \" \"   ",
            " _|  |_  ",
        ],
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  /)  )  ",
            "   \" \"   ",
            "  _| |_  ",
        ],
    ];
    let owl_left: [&[&str]; 2] = [
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  (  (\\  ",
            "   \" \" \\ ",
            "  _|  |_ ",
        ],
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  (  (\\  ",
            "   \" \"   ",
            "  _| |_  ",
        ],
    ];

    let owl_w = 10u16;
    let owl_h = 5u16;

    // Update position every 150ms
    if app.owl_tick.elapsed() >= std::time::Duration::from_millis(75) {
        app.owl_tick = std::time::Instant::now();
        app.owl_frame = (app.owl_frame + 1) % 2;

        app.owl_x += app.owl_dx;
        app.owl_y += app.owl_dy;

        let max_x = (inner.width.saturating_sub(owl_w)) as f32;
        let max_y = (inner.height.saturating_sub(owl_h)) as f32;

        if app.owl_x <= 0.0 {
            app.owl_x = 0.0;
            app.owl_dx = app.owl_dx.abs();
        } else if app.owl_x >= max_x {
            app.owl_x = max_x;
            app.owl_dx = -app.owl_dx.abs();
        }

        if app.owl_y <= 0.0 {
            app.owl_y = 0.0;
            app.owl_dy = app.owl_dy.abs();
        } else if app.owl_y >= max_y {
            app.owl_y = max_y;
            app.owl_dy = -app.owl_dy.abs();
        }
    }

    let frames = if app.owl_dx > 0.0 { &owl_right } else { &owl_left };
    let owl = frames[app.owl_frame % 2];

    let ox = inner.x + app.owl_x as u16;
    let oy = inner.y + app.owl_y as u16;
    let buf = f.buffer_mut();

    for (row, line) in owl.iter().enumerate() {
        let y = oy + row as u16;
        if y >= inner.y + inner.height { break; }
        for (col, ch) in line.chars().enumerate() {
            let x = ox + col as u16;
            if x >= inner.x + inner.width { break; }
            if ch != ' ' {
                buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::Yellow));
            }
        }
    }
}

fn draw_stats_toolbar(f: &mut Frame, app: &App, area: Rect) {
    let toolbar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(52), // sub-tabs
            Constraint::Min(0),    // filter
        ])
        .split(area);

    // Sub-tab selector
    let titles: Vec<Line> = StatsTab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| Line::from(format!("{} {}", i + 1, t.name())))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Stats "))
        .select(StatsTab::ALL.iter().position(|&t| t == app.stats_tab).unwrap_or(0))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, toolbar_chunks[0]);

    // Filter input
    let filter_display = if app.input_mode == InputMode::Expression {
        &app.stats_filter_edit
    } else {
        &app.stats_filter
    };
    let filter_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let filter_widget = Paragraph::new(Span::styled(filter_display.as_str(), filter_style))
        .block(Block::default().borders(Borders::ALL).title(" Filter (/) "));
    f.render_widget(filter_widget, toolbar_chunks[1]);

    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            toolbar_chunks[1].x + app.expression_cursor as u16 + 1,
            toolbar_chunks[1].y + 1,
        ));
    }
}

fn draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
    draw_stats_list(f, app, area);
    if app.stats_view == StatsView::Detail {
        draw_stats_detail(f, app, area);
    }
}

fn draw_stats_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = app.stats_tab.columns();

    let header_cells = columns.iter().enumerate().map(|(i, (field, label, _))| {
        let text = if i == app.stats_sort_column {
            let arrow = if app.stats_sort_desc { "▼" } else { "▲" };
            format!("{label}{arrow}")
        } else {
            label.to_string()
        };
        let style = if i == app.stats_sort_column {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        };
        let line = if is_numeric_field(field) {
            Line::from(text).alignment(Alignment::Right)
        } else {
            Line::from(text)
        };
        Cell::from(line).style(style)
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app.stats_data.iter().map(|item| {
        let cells = columns.iter().map(|(field, _, _)| {
            let val = get_nested_value(item, field);
            let text = format_stats_cell(field, val, item, app.stats_tab);
            if is_numeric_field(field) {
                Cell::from(Line::from(text).alignment(Alignment::Right))
            } else {
                Cell::from(text)
            }
        });
        Row::new(cells)
    }).collect();

    let widths: Vec<Constraint> = columns.iter()
        .map(|(_, _, w)| Constraint::Length(*w))
        .collect();

    let title = format!(
        " {} [{} items] ",
        app.stats_tab.name(),
        app.stats_data.len()
    );

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.stats_table_state);
}

fn is_numeric_field(field: &str) -> bool {
    matches!(field,
        "monitoring" | "freeSpaceM" | "deltaPackets" | "deltaBytesPerSec" |
        "deltaSessions" | "deltaDropped" | "storeSize" | "docs" |
        "searches" | "searchesTime" | "docs.count" | "store.size" | "pri"
    )
}

fn get_nested_value<'a>(item: &'a serde_json::Value, field: &str) -> &'a serde_json::Value {
    // Try flat key first (handles keys like "store.size" that contain dots)
    if let Some(v) = item.get(field) {
        return v;
    }
    // Only try dot-separated path if flat key didn't match
    if field.contains('.') {
        let mut current = item;
        for part in field.split('.') {
            match current.get(part) {
                Some(v) => current = v,
                None => return &serde_json::Value::Null,
            }
        }
        return current;
    }
    &serde_json::Value::Null
}

fn format_stats_cell(field: &str, val: &serde_json::Value, item: &serde_json::Value, tab: StatsTab) -> String {
    match (tab, field) {
        (StatsTab::Capture, "currentTime") => format_epoch_secs(val),
        (StatsTab::Capture, "freeSpaceM") => {
            let size = val.as_f64().map(format_human_megabytes).unwrap_or_else(|| "-".into());
            let pct = item.get("freeSpaceP")
                .and_then(|v| v.as_f64())
                .map(|v| format!(" ({:.0}%)", v))
                .unwrap_or_default();
            format!("{size}{pct}")
        }
        (StatsTab::Capture, "deltaBytesPerSec") => {
            val.as_f64().map(format_human_bytes).unwrap_or_else(|| "-".into())
        }
        (StatsTab::DBStats, "storeSize") => {
            val.as_f64().map(format_human_bytes).unwrap_or_else(|| "-".into())
        }
        (StatsTab::DBIndices, "store.size") => {
            // Value may be a string like "10.2gb" or a number in bytes
            match val {
                serde_json::Value::Number(n) => {
                    n.as_f64().map(format_human_bytes).unwrap_or_else(|| "-".into())
                }
                serde_json::Value::String(s) => {
                    parse_size_string(s).map(format_human_bytes).unwrap_or_else(|| s.clone())
                }
                _ => "-".into(),
            }
        }
        _ => format_stats_value(val),
    }
}

fn format_stats_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == f.floor() && f.abs() < 1e15 {
                    format!("{}", f as i64)
                } else {
                    format!("{:.1}", f)
                }
            } else {
                n.to_string()
            }
        }
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "-".into(),
        serde_json::Value::Array(arr) => arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn draw_stats_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail = match &app.stats_detail {
        Some(d) => d,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.8) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    let filter_lower = detail.filter.to_lowercase();

    if let Some(obj) = detail.data.as_object() {
        let mut keys: Vec<&String> = obj.keys()
            .filter(|k| {
                if filter_lower.is_empty() {
                    return true;
                }
                k.to_lowercase().contains(&filter_lower)
            })
            .collect();
        keys.sort();
        for key in keys {
            let val = &obj[key];
            let val_str = format_stats_value(val);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{key:>30}: "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(val_str),
            ]));
        }
    }

    let title = if !detail.filter.is_empty() {
        format!(" {} Detail [filter: {}] ", app.stats_tab.name(), detail.filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        format!(" {} Detail [filter: ] ", app.stats_tab.name())
    } else {
        format!(" {} Detail (/ filter, Esc close) ", app.stats_tab.name())
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title.as_str()),
        )
        .scroll((detail.scroll, 0));

    f.render_widget(paragraph, popup_area);
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

fn draw_action_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu = match &app.action_menu {
        Some(m) => m,
        None => return,
    };

    if menu.scope.is_some() {
        // Scope selection sub-menu
        let kind_label = menu.pending_kind.map(|k| k.label()).unwrap_or("");
        let title = format!(" {} ", kind_label);
        let scope_options = ["Visible", "Matching"];

        let popup_width = 30u16;
        let popup_height = scope_options.len() as u16 + 2;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let lines: Vec<Line> = scope_options.iter().enumerate().map(|(i, label)| {
            let is_selected = i == menu.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "▸ " } else { "  " };
            Line::from(Span::styled(format!("{prefix}{label}"), style))
        }).collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(title.as_str()),
            );
        f.render_widget(paragraph, popup_area);
        return;
    }

    let options = menu.options(app.remove_enabled());
    let title = match menu.target {
        crate::app::ActionTarget::Single => " Session Action ",
        crate::app::ActionTarget::All => " All Sessions Action ",
    };

    let popup_width = 30u16;
    let popup_height = options.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines: Vec<Line> = options.iter().enumerate().map(|(i, kind)| {
        let is_selected = i == menu.selected;
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▸ " } else { "  " };
        Line::from(Span::styled(format!("{prefix}{}", kind.label()), style))
    }).collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_action_prompt(f: &mut Frame, app: &App, area: Rect) {
    let prompt = match &app.action_prompt {
        Some(p) => p,
        None => return,
    };

    let label = prompt.kind.prompt_label();
    let popup_width = 50u16;
    let popup_height = 3u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let line = Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Yellow)),
        Span::styled(&prompt.input, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Gray)),
    ]);

    let title = format!(" {} ", prompt.kind.label());
    let paragraph = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_debug(f: &mut Frame, app: &App, area: Rect) {
    let entries = app.http_log.lock().unwrap();
    let total = entries.len();

    let mut lines: Vec<Line> = Vec::new();

    // Header line
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<23}", "Timestamp"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:<6}", "Method"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>4}", "Code"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>6}", "First"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>6}", "Last"), Style::default().fg(Color::Yellow)),
        Span::styled("  URL", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    // Newest first
    let visible_height = area.height.saturating_sub(6) as usize; // borders + header + hints
    let scroll = app.debug_scroll.min(total.saturating_sub(1));
    let start = total.saturating_sub(scroll + visible_height);
    let end = total.saturating_sub(scroll);

    for entry in entries[start..end].iter().rev() {
        let ts = entry.timestamp.format("%Y/%m/%d %H:%M:%S%.3f").to_string();
        let status_color = if entry.status >= 400 { Color::Red }
            else if entry.status >= 300 { Color::Yellow }
            else { Color::Green };

        lines.push(Line::from(vec![
            Span::raw(format!(" {:<23}", ts)),
            Span::styled(format!(" {:<6}", entry.method), Style::default().fg(Color::Cyan)),
            Span::styled(format!(" {:>4}", entry.status), Style::default().fg(status_color)),
            Span::raw(format!(" {:>5}ms", entry.first_byte_ms)),
            Span::raw(format!(" {:>5}ms", entry.last_byte_ms)),
            Span::raw(format!("  {}", entry.url)),
        ]));

        if let Some(ref data) = entry.post_data {
            let truncated = if data.len() > 120 { &data[..120] } else { data.as_str() };
            lines.push(Line::from(vec![
                Span::raw("                          "),
                Span::styled(format!("↳ {}", truncated), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    drop(entries);

    let popup_width = area.width.saturating_sub(4).min(140);
    let popup_height = area.height.saturating_sub(4);
    let popup_area = Rect::new(
        area.x + (area.width.saturating_sub(popup_width)) / 2,
        area.y + (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, popup_area);
    let title = format!(" HTTP Debug Log ({} requests) ", total);
    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(" Esc:close  ↑↓:scroll  Home:top ").centered())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let key = |k: &str| Span::styled(format!("  {k:19}"), Style::default().fg(Color::Yellow));
    let blank = || Line::from("");

    macro_rules! hdr {
        ($s:expr) => { Line::from(Span::styled($s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))) };
    }

    let (title, help_text) = if app.packets_view.is_some() {
        ("Packets", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Scroll one page")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Scroll one page")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Options"),
            blank(),
            Line::from(vec![key("r"), Span::raw("Toggle raw packets")]),
            Line::from(vec![key("l"), Span::raw("Cycle line numbers: hex/dec/off")]),
            blank(),
            Line::from(vec![key("Esc / p / q"), Span::raw("Close packets view")]),
            blank(),
            hdr!("Colors"),
            blank(),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Cyan)), Span::raw("Source packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Green)), Span::raw("Destination packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::DarkGray)), Span::raw("Hex offset")]),
        ])
    } else if app.session_view == SessionView::Detail && app.active_tab == Tab::Sessions {
        ("Session Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Add field to expression")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("p"), Span::raw("View packets")]),
            Line::from(vec![key("a"), Span::raw("Session actions")]),
            Line::from(vec![key("A"), Span::raw("All sessions actions")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
        ])
    } else if app.active_tab == Tab::Stats && app.stats_view == StatsView::Detail {
        ("Stats Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
        ])
    } else if app.active_tab == Tab::Stats {
        ("Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("1 / 2 / 3"), Span::raw("Switch sub-tab")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open detail")]),
            Line::from(vec![key("/"), Span::raw("Filter")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("Esc"), Span::raw("Close overlay")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.active_tab == Tab::Arkime {
        ("Arkime Summary", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Add to expression")]),
            Line::from(vec![key("/"), Span::raw("Edit expression")]),
            Line::from(vec![key("f"), Span::raw("Select field")]),
            Line::from(vec![key("G"), Span::raw("Cycle graph metric")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("t / T"), Span::raw("Cycle time range")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("v"), Span::raw("Views")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.show_column_editor {
        ("Column Editor", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Space / Enter"), Span::raw("Toggle field on/off")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("m"), Span::raw("Reorder mode (↑/↓ to move)")]),
            Line::from(vec![key("a"), Span::raw("Apply changes")]),
            Line::from(vec![key("d"), Span::raw("Reset to defaults")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
        ])
    } else if app.show_layout_popup {
        ("Layouts", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate layouts")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select / save / load layout")]),
            Line::from(vec![key("/"), Span::raw("Filter layouts")]),
            Line::from(vec![key("x / Delete"), Span::raw("Delete selected layout")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
        ])
    } else {
        ("Sessions", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate sessions")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
            Line::from(vec![key("Shift+← / Shift+→"), Span::raw("First / last page")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open session detail")]),
            Line::from(vec![key("p"), Span::raw("View packets")]),
            Line::from(vec![key("/"), Span::raw("Edit expression")]),
            Line::from(vec![key("t / T"), Span::raw("Cycle time range")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("g"), Span::raw("Toggle graph")]),
            Line::from(vec![key("G"), Span::raw("Cycle graph type")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("a"), Span::raw("Session actions")]),
            Line::from(vec![key("A"), Span::raw("All sessions actions")]),
            Line::from(vec![key("c"), Span::raw("Columns & layouts")]),
            Line::from(vec![key("v"), Span::raw("Views")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    };

    let mut lines = help_text;
    lines.push(blank());
    lines.push(Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))));

    let popup_width = 52;
    let popup_height = lines.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {title} Help ")),
        );
    f.render_widget(help, popup_area);
}

fn draw_packets(f: &mut Frame, app: &mut App, area: Rect) {
    let pkt_data = match &app.packets_view {
        Some(p) => p,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.9) as u16;
    let popup_height = (area.height as f32 * 0.9) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let half_width = popup_area.width.saturating_sub(2) / 2;

    // Build rows: each row is (left_spans, right_spans)
    let mut rows: Vec<(Vec<Span>, Vec<Span>)> = Vec::new();

    // Column headers
    rows.push((
        vec![Span::styled(
            format!(" {}", pkt_data.src_label),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )],
        vec![Span::styled(
            format!(" {}", pkt_data.dst_label),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )],
    ));

    for pkt in &pkt_data.packets {
        let dir_color = if pkt.src { Color::Cyan } else { Color::Green };
        let mut header_parts = Vec::new();
        if let Some(ts) = pkt.timestamp {
            header_parts.push(Span::styled(
                format!("{} ", format_epoch_ms(ts)),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let mut info = format!("{} bytes", pkt.bytes);
        if !pkt.flags.is_empty() {
            info = format!("{} {}", pkt.flags, info);
        }
        header_parts.push(Span::styled(
            format!("── {} ──", info),
            Style::default().fg(dir_color).add_modifier(Modifier::BOLD),
        ));
        let mut pkt_rows = vec![header_parts];
        for (i, hex_line) in pkt.lines.iter().enumerate() {
            let offset = i * 16;
            let mut spans = Vec::new();
            match app.packets_line {
                crate::app::LineMode::Hex => {
                    spans.push(Span::styled(
                        format!("{:04x}: ", offset),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                crate::app::LineMode::Decimal => {
                    spans.push(Span::styled(
                        format!("{:5}: ", offset),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                crate::app::LineMode::Off => {}
            }
            spans.push(Span::styled(
                hex_line.to_string(),
                Style::default().fg(dir_color),
            ));
            pkt_rows.push(spans);
        }
        for row in pkt_rows {
            if pkt.src {
                rows.push((row, Vec::new()));
            } else {
                rows.push((Vec::new(), row));
            }
        }
    }

    let visible = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = rows.len().saturating_sub(visible) as u16;
    app.packets_scroll = app.packets_scroll.min(max_scroll);
    let start = app.packets_scroll as usize;

    let pct = if rows.is_empty() {
        100
    } else {
        ((start + visible).min(rows.len()) * 100) / rows.len()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Packets ({}) {}% [r]aw:{} [l]ine:{} ",
            pkt_data.total, pct,
            if app.packets_raw { "on" } else { "off" },
            app.packets_line.label(),
        ));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    for (i, (left, right)) in rows.iter().skip(start).enumerate() {
        if i >= visible {
            break;
        }
        let y = inner.y + i as u16;
        if !left.is_empty() {
            let left_area = Rect::new(inner.x, y, half_width, 1);
            let line = Line::from(left.clone());
            f.render_widget(Paragraph::new(line), left_area);
        }
        if !right.is_empty() {
            let right_area = Rect::new(inner.x + half_width, y, inner.width - half_width, 1);
            let line = Line::from(right.clone());
            f.render_widget(Paragraph::new(line), right_area);
        }
    }
}

fn draw_column_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mode_label = if app.column_editor_mode == ColumnEditorMode::Reorder { " [REORDER] " } else { "" };
    let bottom = if app.column_editor_filter.is_empty() {
        " space:toggle /:filter m:move a:apply d:default Esc:close "
    } else {
        " space:toggle ↑↓:navigate Esc:clear filter "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Columns{mode_label}"))
        .title_bottom(Line::from(bottom).fg(Color::DarkGray));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Filter bar at top
    let filter_height = 1u16;
    let filter_area = Rect::new(inner.x, inner.y, inner.width, filter_height);
    let list_area = Rect::new(inner.x, inner.y + filter_height, inner.width, inner.height.saturating_sub(filter_height));

    let filter_active = !app.column_editor_filter.is_empty();
    let filter_text = app.column_editor_filter.trim_matches('\0');
    let filter_display = if !filter_active {
        Line::from(vec![
            Span::styled("  / to filter", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Yellow)),
            Span::raw(filter_text),
            Span::styled("█", Style::default().fg(Color::White)),
        ])
    };
    f.render_widget(Paragraph::new(vec![filter_display]), filter_area);

    // Build filtered view
    let filter_lower = filter_text.to_lowercase();
    let filtered: Vec<usize> = if filter_lower.is_empty() {
        (0..app.column_editor_available.len()).collect()
    } else {
        app.column_editor_available.iter().enumerate()
            .filter(|(_, item)| {
                item.exp.to_lowercase().contains(&filter_lower)
                    || item.friendly_name.to_lowercase().contains(&filter_lower)
            })
            .map(|(i, _)| i)
            .collect()
    };

    let visible_rows = list_area.height as usize;
    let total = filtered.len();

    // Find position of selected in filtered list
    let sel_pos = filtered.iter().position(|&i| i == app.column_editor_selected).unwrap_or(0);

    let scroll_offset = if sel_pos >= visible_rows {
        sel_pos - visible_rows + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for &idx in filtered.iter().skip(scroll_offset).take(visible_rows) {
        let item = &app.column_editor_available[idx];
        let is_selected = idx == app.column_editor_selected;
        let checkbox = if item.enabled { "[x] " } else { "[ ] " };
        let marker = if is_selected && app.column_editor_mode == ColumnEditorMode::Reorder {
            "≡ "
        } else if is_selected {
            "► "
        } else {
            "  "
        };
        let display = if item.friendly_name.is_empty() || item.friendly_name == item.exp {
            item.exp.clone()
        } else {
            format!("{} ({})", item.exp, item.friendly_name)
        };
        let text = format!("{marker}{checkbox}{display}");
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if item.enabled {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(text).style(style));
    }

    // Show scroll indicator
    if total > visible_rows && !lines.is_empty() {
        let pct = if total > 1 { (sel_pos * 100) / (total - 1).max(1) } else { 0 };
        let indicator = format!(" ↕ {}/{} ({}%) ", sel_pos + 1, total, pct);
        let last = lines.len() - 1;
        lines[last] = Line::from(indicator).style(Style::default().fg(Color::DarkGray));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, list_area);
}

fn draw_layout_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_width = 44u16.min(area.width.saturating_sub(4));
    let popup_height = (app.saved_layouts.len() as u16 + 9).min(area.height.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    match app.layout_popup_mode {
        LayoutPopupMode::ConfirmDelete => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Confirm Delete ");
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);
            let lines = vec![
                Line::from(""),
                Line::from(format!("  Delete layout '{}'?", app.layout_delete_name))
                    .style(Style::default().fg(Color::Yellow)),
                Line::from(""),
                Line::from("  y: yes  any other key: cancel")
                    .style(Style::default().fg(Color::DarkGray)),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        LayoutPopupMode::List => {
            let filter_active = !app.layout_filter.is_empty();
            let bottom = if filter_active {
                " Enter:select ↑↓:navigate Esc:clear "
            } else {
                " Enter:select /:filter x:delete Esc:close "
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Layouts ")
                .title_bottom(Line::from(bottom).fg(Color::DarkGray));

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let mut lines: Vec<Line> = Vec::new();

            if !filter_active {
                // "Edit Columns" option
                let style = if app.layout_popup_selected == 0 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Magenta)
                };
                lines.push(Line::from("  ⚙ Edit Columns").style(style));

                // "Save Current" option
                let style = if app.layout_popup_selected == 1 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                lines.push(Line::from("  [+] Save Current Layout").style(style));

                // "Default" option
                let style = if app.layout_popup_selected == 2 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from("  ↺ Default Columns").style(style));

                // Separator
                lines.push(Line::from("  ────────────────────────────────").style(Style::default().fg(Color::DarkGray)));
            } else {
                // Filter bar
                let filter_text = app.layout_filter.trim_matches('\0');
                lines.push(Line::from(vec![
                    Span::styled("  /", Style::default().fg(Color::Yellow)),
                    Span::raw(filter_text),
                    Span::styled("█", Style::default().fg(Color::White)),
                ]));
            }

            // Saved layouts (filtered if filter active)
            let filter_text = app.layout_filter.trim_matches('\0').to_lowercase();
            let mut any_shown = false;
            for (i, layout) in app.saved_layouts.iter().enumerate() {
                if !filter_text.is_empty() && !layout.name.to_lowercase().contains(&filter_text) {
                    continue;
                }
                any_shown = true;
                let is_selected = app.layout_popup_selected == i + 3;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };
                let col_count = layout.columns.len();
                lines.push(Line::from(format!("  {} ({} cols)", layout.name, col_count)).style(style));
            }

            if !any_shown {
                lines.push(Line::from("  (no saved layouts)").style(Style::default().fg(Color::DarkGray)));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
        LayoutPopupMode::SaveInput => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Save Layout ");

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from("  Layout name:").style(Style::default().fg(Color::Yellow)));

            // Input field with cursor
            let name = &app.layout_save_name;
            let cursor = app.layout_save_cursor;
            let mut spans = vec![Span::raw("  ")];
            if cursor < name.len() {
                spans.push(Span::raw(&name[..cursor]));
                spans.push(Span::styled(&name[cursor..cursor+1], Style::default().bg(Color::White).fg(Color::Black)));
                spans.push(Span::raw(&name[cursor+1..]));
            } else {
                spans.push(Span::raw(name.as_str()));
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            }
            lines.push(Line::from(spans));

            lines.push(Line::from(""));
            lines.push(Line::from("  Enter: save  Esc: cancel").style(Style::default().fg(Color::DarkGray)));

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
    }
}

fn draw_view_popup(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::ViewPopupMode;

    let filtered = app.view_filtered_indices();
    let popup_height = (filtered.len() as u16 + 8).min(area.height - 2).max(8);
    let popup_width = 60u16.min(area.width - 4);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let title = match app.view_popup_mode {
        ViewPopupMode::SaveInput => " Save View ",
        ViewPopupMode::ConfirmDelete => " Confirm Delete ",
        _ => " Views ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title);
    f.render_widget(block, popup_area);

    let inner = Rect::new(popup_area.x + 1, popup_area.y + 1, popup_area.width - 2, popup_area.height - 2);

    match app.view_popup_mode {
        ViewPopupMode::SaveInput => {
            let checkbox = if app.view_save_columns { "[x]" } else { "[ ]" };
            let lines = vec![
                Line::from("Enter view name:"),
                Line::from(""),
                Line::from(Span::styled(&app.view_save_name, Style::default().fg(Color::White).add_modifier(Modifier::UNDERLINED))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(checkbox, Style::default().fg(Color::Cyan)),
                    Span::styled(" Save current columns (Tab to toggle)", Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Expression: ", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(app.expression.clone(), Style::default().fg(Color::Gray))),
            ];
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
            let cursor_x = inner.x + app.view_save_cursor as u16;
            let cursor_y = inner.y + 2;
            if cursor_x < inner.right() {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
        ViewPopupMode::ConfirmDelete => {
            let lines = vec![
                Line::from(vec![
                    Span::raw("Delete view "),
                    Span::styled(&app.view_delete_name, Style::default().fg(Color::Yellow)),
                    Span::raw("?"),
                ]),
                Line::from(""),
                Line::from(Span::styled("y/N", Style::default().fg(Color::Red))),
            ];
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
        ViewPopupMode::List => {
            let mut lines: Vec<Line> = Vec::new();
            let active_marker = |id: &str| -> &str {
                if app.active_view.as_deref() == Some(id) { " ●" } else { "" }
            };

            // Option 0: Save current expression as view
            let save_style = if app.view_popup_selected == 0 {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Green)
            };
            lines.push(Line::from(Span::styled("[+] Save Current Expression as View", save_style)));

            // Option 1: Clear view
            let clear_style = if app.view_popup_selected == 1 {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Red)
            };
            let clear_label = if app.active_view.is_some() { "✖ Clear Active View" } else { "✖ No View Active" };
            lines.push(Line::from(Span::styled(clear_label, clear_style)));

            // Separator
            lines.push(Line::from(Span::styled("─".repeat(inner.width as usize), Style::default().fg(Color::DarkGray))));

            // Filter indicator
            if app.view_filter_active {
                lines.push(Line::from(vec![
                    Span::styled("/", Style::default().fg(Color::DarkGray)),
                    Span::styled(&app.view_filter, Style::default().fg(Color::Yellow)),
                ]));
            }

            // Views
            for (fi, &idx) in filtered.iter().enumerate() {
                let view = &app.saved_views[idx];
                let selected = app.view_popup_selected == fi + 2;
                let base_style = if selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                let mut spans = Vec::new();
                if view.shared {
                    spans.push(Span::styled("🔗 ", Style::default().fg(Color::Cyan)));
                } else {
                    spans.push(Span::raw("   "));
                }
                spans.push(Span::styled(&view.name, base_style));
                let marker = active_marker(&view.id);
                if !marker.is_empty() {
                    spans.push(Span::styled(marker, Style::default().fg(Color::Green)));
                }
                // Show expression in gray
                let remaining = inner.width as usize - view.name.len() - 4 - marker.len();
                if remaining > 5 {
                    let expr_display = if view.expression.len() > remaining {
                        format!(" {}..", &view.expression[..remaining - 3])
                    } else {
                        format!(" {}", view.expression)
                    };
                    spans.push(Span::styled(expr_display, Style::default().fg(Color::DarkGray)));
                }
                lines.push(Line::from(spans));
            }

            if filtered.is_empty() && !app.saved_views.is_empty() {
                lines.push(Line::from(Span::styled("  (no matching views)", Style::default().fg(Color::DarkGray))));
            } else if app.saved_views.is_empty() {
                lines.push(Line::from(Span::styled("  (no saved views)", Style::default().fg(Color::DarkGray))));
            }

            // Footer
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Enter=select  x=delete  /=filter  Esc=close",
                Style::default().fg(Color::DarkGray),
            )));

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
    }
}

fn draw_loading(f: &mut Frame, app: &mut App, area: Rect) {
    let owl_right = [
        " ,___,  ",
        " (O,O)  ",
        " /)  )  ",
        "  \" \"   ",
        " _| |_  ",
    ];
    let owl_left = [
        "  ,___,  ",
        "  (O,O)  ",
        "  (  (\\  ",
        "   \" \"   ",
        "  _| |_  ",
    ];
    let popup_width = 30u16;
    let popup_height = (owl_right.len() + 5) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let owl_w = 10u16;
    let max_x = inner.width.saturating_sub(owl_w);

    // Animate owl position
    if app.loading_owl_tick.elapsed() >= std::time::Duration::from_millis(100) {
        app.loading_owl_tick = std::time::Instant::now();
        let new_x = app.loading_owl_x as i16 + app.loading_owl_dx;
        if new_x <= 0 {
            app.loading_owl_x = 0;
            app.loading_owl_dx = 1;
        } else if new_x >= max_x as i16 {
            app.loading_owl_x = max_x;
            app.loading_owl_dx = -1;
        } else {
            app.loading_owl_x = new_x as u16;
        }
    }

    let owl = if app.loading_owl_dx > 0 { &owl_right } else { &owl_left };

    // Draw owl at animated position
    for (i, row) in owl.iter().enumerate() {
        let y = inner.y + 1 + i as u16;
        if y < inner.y + inner.height {
            let x = inner.x + app.loading_owl_x;
            let owl_area = Rect::new(x, y, owl_w.min(inner.width - app.loading_owl_x), 1);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(*row, Style::default().fg(Color::Yellow)))),
                owl_area,
            );
        }
    }

    // Draw "Loading ..." text centered
    let loading_y = inner.y + owl_right.len() as u16 + 2;
    if loading_y < inner.y + inner.height {
        let text = "Loading ...";
        let text_x = inner.x + (inner.width.saturating_sub(text.len() as u16)) / 2;
        let text_area = Rect::new(text_x, loading_y, text.len() as u16, 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)))),
            text_area,
        );
    }
}
