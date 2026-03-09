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
    let is_editing = app.input_mode == InputMode::Expression;
    render_text_input(f, filter_display, app.expression_cursor, is_editing, " Filter (/) ", toolbar_chunks[1]);
}

pub(super) fn draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
    draw_stats_list(f, app, area);
    if app.vr_stats_view == StatsView::Detail {
        draw_stats_detail(f, app, area);
    }
}

fn draw_stats_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = app.vr_stats_active_columns().clone();

    let header_cells = columns.iter().enumerate().map(|(i, col)| {
        let is_sorted = i == app.vr_stats_sort_column;
        let text = sort_header_label(&col.label, is_sorted, app.vr_stats_sort_desc);
        let style = sort_header_style(is_sorted);
        let line = if col.is_numeric() {
            Line::from(text).alignment(Alignment::Right)
        } else {
            Line::from(text)
        };
        Cell::from(line).style(style)
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app.vr_stats_data.iter().map(|item| {
        let cells = columns.iter().map(|col| {
            let val = get_nested_value(item, &col.field);
            let text = format_stats_cell_dynamic(col, val, item);
            if col.is_numeric() {
                Cell::from(Line::from(text).alignment(Alignment::Right))
            } else {
                Cell::from(text)
            }
        });
        Row::new(cells)
    }).collect();

    let widths: Vec<Constraint> = columns.iter()
        .map(|col| Constraint::Length(col.width))
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

pub(super) fn get_nested_value<'a>(item: &'a serde_json::Value, field: &str) -> &'a serde_json::Value {
    if let Some(v) = item.get(field) {
        return v;
    }
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

pub(super) fn format_stats_cell_dynamic(col: &StatsColumnDef, val: &serde_json::Value, item: &serde_json::Value) -> String {
    match col.format {
        StatsFormat::EpochSecs => format_epoch_secs(val),
        StatsFormat::Bytes => {
            let base = val.as_f64().map(format_human_bytes).unwrap_or_else(|| "-".into());
            // Show percentage if a companion "P" field exists (e.g. memoryP for memory)
            let pct_field = format!("{}P", col.field);
            let pct = item.get(&pct_field)
                .and_then(|v| v.as_f64())
                .map(|v| format!(" ({:.0}%)", v))
                .unwrap_or_default();
            format!("{base}{pct}")
        }
        StatsFormat::BytesPerSec => {
            val.as_f64().map(format_human_bytes).unwrap_or_else(|| "-".into())
        }
        StatsFormat::MegaBytes => {
            let size = val.as_f64().map(format_human_megabytes).unwrap_or_else(|| "-".into());
            // Show percentage if a companion "P" field exists (e.g. freeSpaceP for freeSpaceM)
            let pct_field = col.field.trim_end_matches('M').to_string() + "P";
            let pct = item.get(&pct_field)
                .and_then(|v| v.as_f64())
                .map(|v| format!(" ({:.0}%)", v))
                .unwrap_or_default();
            format!("{size}{pct}")
        }
        StatsFormat::Percent => {
            val.as_f64().map(|v| format!("{:.1}%", v / 100.0)).unwrap_or_else(|| "-".into())
        }
        StatsFormat::SizeString => {
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
        StatsFormat::Number => format_stats_value(val),
        StatsFormat::String => format_stats_value(val),
        StatsFormat::EpochMs => {
            val.as_f64().map(|ms| {
                let secs = (ms / 1000.0) as i64;
                format_epoch_secs(&serde_json::Value::Number(serde_json::Number::from(secs)))
            }).unwrap_or_else(|| "-".into())
        }
        StatsFormat::Boolean => {
            match val.as_i64().or_else(|| val.as_f64().map(|f| f as i64)) {
                Some(0) => "False".into(),
                Some(_) => "True".into(),
                None => match val.as_bool() {
                    Some(b) => if b { "True" } else { "False" }.into(),
                    None => "-".into(),
                },
            }
        }
        StatsFormat::PercentSuffix => {
            val.as_f64().map(|v| format!("{:.1}%", v)).unwrap_or_else(|| "-".into())
        }
    }
}

pub(super) fn format_stats_value(val: &serde_json::Value) -> String {
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
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    let filter_lower = detail.filter.to_lowercase();

    // Show exclusion status banner for DB Nodes
    if app.vr_stats_tab == crate::app::StatsTab::DBStats {
        let node_excluded = detail.data.get("nodeExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
        let ip_excluded = detail.data.get("ipExcluded").and_then(|v| v.as_bool()).unwrap_or(false);
        let node_style = if node_excluded {
            Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let ip_style = if ip_excluded {
            Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        lines.push(Line::from(vec![
            Span::styled("  Node: ", Style::default().fg(Color::Yellow)),
            Span::styled(if node_excluded { "EXCLUDED" } else { "included" }, node_style),
            Span::raw("  (e) toggle    "),
            Span::styled("IP: ", Style::default().fg(Color::Yellow)),
            Span::styled(if ip_excluded { "EXCLUDED" } else { "included" }, ip_style),
            Span::raw("  (x) toggle"),
        ]));
        lines.push(Line::from(""));
    }

    // Build field → friendly label map from all columns for this tab
    let all_cols = crate::app::stats_tab_all_columns(app.vr_stats_tab);
    let label_map: std::collections::HashMap<&str, &str> = all_cols.iter()
        .map(|c| (c.field.as_str(), c.label.as_str()))
        .collect();

    if let Some(obj) = detail.data.as_object() {
        let mut keys: Vec<&String> = obj.keys()
            .filter(|k| {
                if filter_lower.is_empty() {
                    return true;
                }
                let k_str = k.as_str();
                let friendly = label_map.get(k_str).copied().unwrap_or(k_str);
                k_str.to_lowercase().contains(&filter_lower)
                    || friendly.to_lowercase().contains(&filter_lower)
            })
            .collect();
        keys.sort();
        for key in keys {
            let val = &obj[key];
            let val_str = format_stats_value(val);
            let key_str = key.as_str();
            let display_name = label_map.get(key_str).copied().unwrap_or(key_str);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{display_name:>30}: "),
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
