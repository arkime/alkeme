use crate::app::{App, GraphType, InputMode, SessionView, Tab, TimeRange};
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
    let mut constraints = vec![
        Constraint::Length(3), // tabs
        Constraint::Length(3), // toolbar: time range + expression
    ];
    if app.graph_size.is_visible() {
        constraints.push(Constraint::Length(app.graph_size.height())); // graph
    }
    constraints.push(Constraint::Min(0));   // content
    constraints.push(Constraint::Length(1)); // status bar

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
        _ => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(app.active_tab.name());
            let text = Paragraph::new("Coming soon...").block(block);
            f.render_widget(text, chunks[idx]);
        }
    }
    idx += 1;

    draw_status_bar(f, app, chunks[idx]);

    if app.show_help {
        draw_help(f, f.area());
    }
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
            toolbar_chunks[1].x + app.expression_edit.len() as u16 + 1,
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
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect) {
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

fn draw_session_detail(f: &mut Frame, app: &App, area: Rect) {
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

    if let Some(obj) = detail.data.as_object() {
        let mut keys: Vec<&String> = obj.keys().collect();
        keys.sort();
        for key in keys {
            let val = &obj[key];
            let val_str = if let Some(field_type) = app.date_fields.get(key.as_str()) {
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
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{key:>30}: "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(val_str),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Session Detail (Esc to close) "),
        )
        .scroll((detail.scroll, 0));

    f.render_widget(paragraph, popup_area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::raw(&app.status_msg),
        Span::styled(
            "  Tab/Shift+Tab: switch tabs | j/k: navigate | Enter: open | r: refresh | q: quit ",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(status, area);
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
        Line::from(vec![Span::styled("  Enter            ", Style::default().fg(Color::Yellow)), Span::raw("Open session detail")]),
        Line::from(vec![Span::styled("  Esc              ", Style::default().fg(Color::Yellow)), Span::raw("Close overlay")]),
        Line::from(vec![Span::styled("  r                ", Style::default().fg(Color::Yellow)), Span::raw("Refresh sessions")]),
        Line::from(vec![Span::styled("  /                ", Style::default().fg(Color::Yellow)), Span::raw("Search expression")]),
        Line::from(vec![Span::styled("  t / T            ", Style::default().fg(Color::Yellow)), Span::raw("Cycle time range")]),
        Line::from(vec![Span::styled("  s                ", Style::default().fg(Color::Yellow)), Span::raw("Next sort column")]),
        Line::from(vec![Span::styled("  S                ", Style::default().fg(Color::Yellow)), Span::raw("Toggle sort direction")]),
        Line::from(vec![Span::styled("  g                ", Style::default().fg(Color::Yellow)), Span::raw("Toggle graph")]),
        Line::from(vec![Span::styled("  G                ", Style::default().fg(Color::Yellow)), Span::raw("Cycle graph type")]),
        Line::from(vec![Span::styled("  h                ", Style::default().fg(Color::Yellow)), Span::raw("Show this help")]),
        Line::from(vec![Span::styled("  q                ", Style::default().fg(Color::Yellow)), Span::raw("Quit")]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let popup_width = 44;
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
