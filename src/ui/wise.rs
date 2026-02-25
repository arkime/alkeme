use ratatui::{
    prelude::*,
    widgets::*,
};

use super::*;

pub fn draw_wise(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    // Tab bar
    let tabs: Vec<Line> = app.app_mode.tabs().iter().map(|t| {
        Line::from(t.name())
    }).collect();
    let tab_idx = app.app_mode.tabs().iter().position(|&t| t == app.active_tab).unwrap_or(0);
    let tabs_widget = Tabs::new(tabs)
        .select(tab_idx)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .divider(" │ ");
    f.render_widget(tabs_widget, chunks[0]);

    match app.active_tab {
        Tab::WsStats => draw_ws_stats(f, app, chunks[1]),
        Tab::WsQuery => draw_ws_query(f, app, chunks[1]),
        _ => {
            let block = Block::default().borders(Borders::ALL).title("Settings");
            f.render_widget(block, chunks[1]);
        }
    }

    // Status bar
    let status = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_msg, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled("Tab/Shift+Tab: switch tabs | h: help | q: quit", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(status, chunks[2]);
}

fn draw_ws_stats(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // sub-tab bar
            Constraint::Length(1), // filter
            Constraint::Min(0),   // table
        ])
        .split(area);

    // Sub-tab bar: [1] Sources  [2] Types
    let sub_tabs = Line::from(vec![
        Span::styled("[1] ", Style::default().fg(Color::DarkGray)),
        Span::styled("Sources", if app.ws_stats_tab == WsStatsTab::Sources {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::White) }),
        Span::raw("  "),
        Span::styled("[2] ", Style::default().fg(Color::DarkGray)),
        Span::styled("Types", if app.ws_stats_tab == WsStatsTab::Types {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::White) }),
    ]);
    f.render_widget(Paragraph::new(sub_tabs), chunks[0]);

    // Filter bar
    let filter_text = if app.input_mode == InputMode::Expression {
        &app.ws_stats_filter_edit
    } else {
        &app.ws_stats_filter
    };
    let filter_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let filter = Paragraph::new(Line::from(vec![
        Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
        Span::styled(filter_text, filter_style),
    ]));
    f.render_widget(filter, chunks[1]);

    match app.ws_stats_tab {
        WsStatsTab::Sources => draw_sources_table(f, app, chunks[2]),
        WsStatsTab::Types => draw_types_table(f, app, chunks[2]),
    }
}

fn draw_sources_table(f: &mut Frame, app: &mut App, area: Rect) {
    let sources = app.ws_filtered_sources();
    if sources.is_empty() {
        let msg = Paragraph::new("No source stats available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Source"),
        Cell::from("Requests").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Hit").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Miss").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Refresh").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Direct Hit").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Dropped").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Avg MS").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Items").style(Style::default().fg(Color::DarkGray)),
    ]).style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = sources.iter().enumerate().map(|(i, s)| {
        let style = if i == app.ws_stats_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Row::new(vec![
            Cell::from(s.source.as_str()),
            Cell::from(format_num(s.request)),
            Cell::from(format_num(s.cache_hit)),
            Cell::from(format_num(s.cache_miss)),
            Cell::from(format_num(s.cache_refresh)),
            Cell::from(format_num(s.direct_hit)),
            Cell::from(format_num(s.request_dropped)),
            Cell::from(format!("{:.4}", s.recent_avg_ms)),
            Cell::from(format_num(s.items)),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(13),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(table, area);
}

fn draw_types_table(f: &mut Frame, app: &mut App, area: Rect) {
    let types = app.ws_filtered_types();
    if types.is_empty() {
        let msg = Paragraph::new("No type stats available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Type"),
        Cell::from("Requests").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Found").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Hit").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Src Hit").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Src Miss").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Cache Src Refresh").style(Style::default().fg(Color::DarkGray)),
    ]).style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = types.iter().enumerate().map(|(i, t)| {
        let style = if i == app.ws_stats_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Row::new(vec![
            Cell::from(t.type_name.as_str()),
            Cell::from(format_num(t.request)),
            Cell::from(format_num(t.found)),
            Cell::from(format_num(t.cache_hit)),
            Cell::from(format_num(t.cache_src_hit)),
            Cell::from(format_num(t.cache_src_miss)),
            Cell::from(format_num(t.cache_src_refresh)),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(17),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(table, area);
}

fn draw_ws_query(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // query inputs bar
            Constraint::Length(1), // value input
            Constraint::Min(0),   // results
        ])
        .split(area);

    // Query inputs bar: Source: [any]  Type: [ip]
    let query_bar = Line::from(vec![
        Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("[{}]", app.ws_query_source), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("Type: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("[{}]", app.ws_query_type), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("(s: cycle source, t: cycle type, /: edit value, Enter: query)", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(query_bar), chunks[0]);

    // Value input
    let value_text = if app.input_mode == InputMode::Expression {
        &app.ws_query_value_edit
    } else {
        &app.ws_query_value
    };
    let value_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let value_line = Line::from(vec![
        Span::styled("Value: ", Style::default().fg(Color::DarkGray)),
        Span::styled(value_text, value_style),
    ]);
    f.render_widget(Paragraph::new(value_line), chunks[1]);

    // Results table
    if app.ws_query_results.is_empty() {
        let msg = if app.ws_query_value.is_empty() {
            "Enter a value and press Enter to query"
        } else {
            "No results"
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(p, chunks[2]);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Field"),
        Cell::from("Value"),
    ]).style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.ws_query_results.iter().enumerate().map(|(i, r)| {
        let style = if i == app.ws_query_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let val = match &r.value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Row::new(vec![
            Cell::from(r.field.as_str()),
            Cell::from(val),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Length(30),
        Constraint::Min(20),
    ];

    let title = format!(" Results ({}) ", app.ws_query_results.len());
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::TOP).title(title));
    f.render_widget(table, chunks[2]);
}

fn format_num(n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
