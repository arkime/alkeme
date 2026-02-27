use super::*;

pub(super) fn draw_stats_toolbar(f: &mut Frame, app: &App, area: Rect) {
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
        .select(StatsTab::ALL.iter().position(|&t| t == app.vr_stats_tab).unwrap_or(0))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, toolbar_chunks[0]);

    // Filter input
    let filter_display = if app.input_mode == InputMode::Expression {
        &app.vr_stats_filter_edit
    } else {
        &app.vr_stats_filter
    };
    let filter_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let inner_width_stats = toolbar_chunks[1].width.saturating_sub(2) as usize;
    let filter_scroll = if app.input_mode == InputMode::Expression && app.expression_cursor > inner_width_stats {
        (app.expression_cursor - inner_width_stats) as u16
    } else {
        0
    };
    let filter_widget = Paragraph::new(Span::styled(filter_display.as_str(), filter_style))
        .scroll((0, filter_scroll))
        .block(Block::default().borders(Borders::ALL).title(" Filter (/) "));
    f.render_widget(filter_widget, toolbar_chunks[1]);

    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            toolbar_chunks[1].x + (app.expression_cursor as u16 - filter_scroll) + 1,
            toolbar_chunks[1].y + 1,
        ));
    }
}

pub(super) fn draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
    draw_stats_list(f, app, area);
    if app.vr_stats_view == StatsView::Detail {
        draw_stats_detail(f, app, area);
    }
}

fn draw_stats_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = app.vr_stats_tab.columns();

    let header_cells = columns.iter().enumerate().map(|(i, (field, label, _))| {
        let text = if i == app.vr_stats_sort_column {
            let arrow = if app.vr_stats_sort_desc { "▼" } else { "▲" };
            format!("{label}{arrow}")
        } else {
            label.to_string()
        };
        let style = if i == app.vr_stats_sort_column {
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

    let rows: Vec<Row> = app.vr_stats_data.iter().map(|item| {
        let cells = columns.iter().map(|(field, _, _)| {
            let val = get_nested_value(item, field);
            let text = format_stats_cell(field, val, item, app.vr_stats_tab);
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
        app.vr_stats_tab.name(),
        app.vr_stats_data.len()
    );

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.vr_stats_table_state);
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
    let detail = match &app.vr_stats_detail {
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
        format!(" {} Detail [filter: {}] ", app.vr_stats_tab.name(), detail.filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        format!(" {} Detail [filter: ] ", app.vr_stats_tab.name())
    } else {
        format!(" {} Detail (/ filter, Esc close) ", app.vr_stats_tab.name())
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
