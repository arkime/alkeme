use crate::app::{App, DetailActionMenu, GraphType, InputMode, SessionView, StatsTab, StatsView, Tab, TimeRange, is_hidden_detail_field};
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

    if app.show_help {
        draw_help(f, f.area());
    }
    if app.action_menu.is_some() {
        draw_action_menu(f, app, f.area());
    }
    if app.input_mode == InputMode::ActionPrompt {
        draw_action_prompt(f, app, f.area());
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
    if app.graph_size.is_visible() {
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

    if app.graph_size.is_visible() {
        draw_graph(f, app, chunks[idx]); idx += 1;
    }

    match app.active_tab {
        Tab::Sessions => draw_sessions(f, app, chunks[idx]),
        Tab::Arkime | Tab::Settings => {
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
    let labels = [
        "IP", "First Packet", "Last Packet", "Src IP", "SrcPort",
        "Dst IP", "DstPort", "Protocols", "Src Pkts", "Dst Pkts",
        "Src Bytes", "Dst Bytes",
    ];
    let header_cells = labels
    .iter()
    .enumerate()
    .map(|(i, h)| {
        let label = if i == app.sort_column {
            let arrow = if app.sort_desc { "▼" } else { "▲" };
            format!("{h}{arrow}")
        } else {
            h.to_string()
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
            let cells = app.session_fields.iter().map(|field| {
                let val = session.get(field).unwrap_or(&serde_json::Value::Null);
                let text = if field == "ipProtocol" {
                    ip_protocol_str(val)
                } else if let Some(field_type) = app.date_fields.get(field.as_str()) {
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

    let widths = [
        Constraint::Length(4),  // ipProtocol
        Constraint::Length(20), // firstPacket
        Constraint::Length(20), // lastPacket
        Constraint::Length(16), // src ip
        Constraint::Length(7),  // src port
        Constraint::Length(16), // dst ip
        Constraint::Length(7),  // dst port
        Constraint::Length(20), // protocols
        Constraint::Length(9),  // src pkts
        Constraint::Length(9),  // dst pkts
        Constraint::Length(10), // src bytes
        Constraint::Length(10), // dst bytes
    ];

    let end = (app.page_start + app.sessions.len() as u64).min(app.sessions_filtered);
    let page_label = if app.sessions_filtered > 0 {
        format!(" Sessions [{}-{} of {}] ◄ ► ", app.page_start + 1, end, app.sessions_filtered)
    } else {
        " Sessions [0] ".into()
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
            let size = val.as_f64().map(|v| format_human_megabytes(v)).unwrap_or_else(|| "-".into());
            let pct = item.get("freeSpaceP")
                .and_then(|v| v.as_f64())
                .map(|v| format!(" ({:.0}%)", v))
                .unwrap_or_default();
            format!("{size}{pct}")
        }
        (StatsTab::Capture, "deltaBytesPerSec") => {
            val.as_f64().map(|v| format_human_bytes(v)).unwrap_or_else(|| "-".into())
        }
        (StatsTab::DBStats, "storeSize") => {
            val.as_f64().map(|v| format_human_bytes(v)).unwrap_or_else(|| "-".into())
        }
        (StatsTab::DBIndices, "store.size") => {
            // Value may be a string like "10.2gb" or a number in bytes
            match val {
                serde_json::Value::Number(n) => {
                    n.as_f64().map(|v| format_human_bytes(v)).unwrap_or_else(|| "-".into())
                }
                serde_json::Value::String(s) => {
                    parse_size_string(s).map(|v| format_human_bytes(v)).unwrap_or_else(|| s.clone())
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

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("  Tab / Shift+Tab  ", Style::default().fg(Color::Yellow)), Span::raw("Switch tabs")]),
        Line::from(vec![Span::styled("  j / k / ↑ / ↓    ", Style::default().fg(Color::Yellow)), Span::raw("Navigate sessions")]),
        Line::from(vec![Span::styled("  ← / →            ", Style::default().fg(Color::Yellow)), Span::raw("Previous/next page")]),
        Line::from(vec![Span::styled("  Shift+← / Shift+→", Style::default().fg(Color::Yellow)), Span::raw("First/last page")]),
        Line::from(vec![Span::styled("  Home             ", Style::default().fg(Color::Yellow)), Span::raw("First page")]),
        Line::from(vec![Span::styled("  PgUp / PgDn      ", Style::default().fg(Color::Yellow)), Span::raw("Scroll detail view")]),
        Line::from(""),
        Line::from(Span::styled("Actions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("  Enter            ", Style::default().fg(Color::Yellow)), Span::raw("Open session detail / add to expression")]),
        Line::from(vec![Span::styled("  Esc              ", Style::default().fg(Color::Yellow)), Span::raw("Close overlay")]),
        Line::from(vec![Span::styled("  r                ", Style::default().fg(Color::Yellow)), Span::raw("Refresh sessions")]),
        Line::from(vec![Span::styled("  /                ", Style::default().fg(Color::Yellow)), Span::raw("Search expression")]),
        Line::from(vec![Span::styled("  t / T            ", Style::default().fg(Color::Yellow)), Span::raw("Cycle time range")]),
        Line::from(vec![Span::styled("  s                ", Style::default().fg(Color::Yellow)), Span::raw("Next sort column")]),
        Line::from(vec![Span::styled("  S                ", Style::default().fg(Color::Yellow)), Span::raw("Toggle sort direction")]),
        Line::from(vec![Span::styled("  g                ", Style::default().fg(Color::Yellow)), Span::raw("Toggle graph")]),
        Line::from(vec![Span::styled("  G                ", Style::default().fg(Color::Yellow)), Span::raw("Cycle graph type")]),
        Line::from(vec![Span::styled("  a                ", Style::default().fg(Color::Yellow)), Span::raw("Session actions (pcap/tags)")]),
        Line::from(vec![Span::styled("  A                ", Style::default().fg(Color::Yellow)), Span::raw("All sessions actions")]),
        Line::from(""),
        Line::from(Span::styled("Stats Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("  1 / 2 / 3       ", Style::default().fg(Color::Yellow)), Span::raw("Switch stats sub-tab")]),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("  h                ", Style::default().fg(Color::Yellow)), Span::raw("Show this help")]),
        Line::from(vec![Span::styled("  q                ", Style::default().fg(Color::Yellow)), Span::raw("Quit")]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let popup_width = 62;
    let popup_height = help_text.len() as u16 + 2; // +2 for borders
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help "),
        );
    f.render_widget(help, popup_area);
}
