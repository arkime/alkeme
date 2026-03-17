use super::*;

pub(super) fn draw_stats_toolbar(f: &mut Frame, app: &App, area: Rect) {
    let toolbar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(112), // sub-tabs (7 tabs)
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
        .select(StatsTab::ALL.iter().position(|&t| t == app.viewer.stats_tab).unwrap_or(0))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, toolbar_chunks[0]);

    // Filter input
    let filter_display = if app.input_mode == InputMode::Expression {
        &app.viewer.stats_filter_edit
    } else {
        &app.viewer.stats_filter
    };
    let is_editing = app.input_mode == InputMode::Expression;
    render_text_input(f, filter_display, app.expression_cursor, is_editing, " Filter (/) ", toolbar_chunks[1]);
}

pub(super) fn draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
    if app.viewer.stats_tab == StatsTab::CaptureGraphs {
        draw_capture_graphs(f, app, area);
        return;
    }
    if app.viewer.stats_tab == StatsTab::DBShards {
        draw_stats_shards(f, app, area);
        return;
    }
    draw_stats_list(f, app, area);
    if app.viewer.stats_view == StatsView::Detail {
        draw_stats_detail(f, app, area);
    }
}

fn draw_stats_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = app.vr_stats_active_columns().clone();

    let header_cells = columns.iter().enumerate().map(|(i, col)| {
        let is_sorted = i == app.viewer.stats_sort_column;
        let text = sort_header_label(&col.label, is_sorted, app.viewer.stats_sort_desc);
        let style = sort_header_style(is_sorted);
        let line = if col.is_numeric() {
            Line::from(text).alignment(Alignment::Right)
        } else {
            Line::from(text)
        };
        Cell::from(line).style(style)
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app.viewer.stats_data.iter().map(|item| {
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
        app.viewer.stats_tab.name(),
        app.viewer.stats_data.len()
    );

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.viewer.stats_table_state);

    // Render mode bar on bottom border for DB Recovery
    if app.viewer.stats_tab == StatsTab::DBRecovery {
        let modes = [("Active", false), ("All", true)];
        let mode_spans: Vec<Span> = modes.iter().flat_map(|(label, is_all)| {
            let style = if *is_all == app.viewer.recovery_show_all {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            vec![Span::styled(format!(" {} ", label), style), Span::raw("│")]
        }).collect();
        let mode_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2).min(30),
            height: 1,
        };
        f.render_widget(Line::from(mode_spans), mode_area);
    }
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
        StatsFormat::Nanos => {
            val.as_f64().map(|nanos| {
                let secs = nanos / 1_000_000_000.0;
                if secs >= 3600.0 {
                    format!("{:.0}h {:.0}m", (secs / 3600.0).floor(), ((secs % 3600.0) / 60.0).floor())
                } else if secs >= 60.0 {
                    format!("{:.0}m {:.0}s", (secs / 60.0).floor(), (secs % 60.0).floor())
                } else if secs >= 1.0 {
                    format!("{:.1}s", secs)
                } else {
                    format!("{:.0}ms", nanos / 1_000_000.0)
                }
            }).unwrap_or_else(|| "-".into())
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
    let detail = match &app.viewer.stats_detail {
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
    if app.viewer.stats_tab == crate::app::StatsTab::DBStats {
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
    let all_cols = crate::app::stats_tab_all_columns(app.viewer.stats_tab);
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
        format!(" {} Detail [filter: {}] ", app.viewer.stats_tab.name(), detail.filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        format!(" {} Detail [filter: ] ", app.viewer.stats_tab.name())
    } else {
        format!(" {} Detail (/ filter, Esc close) ", app.viewer.stats_tab.name())
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

fn draw_stats_shards(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::ShardsShow;

    let nodes = &app.viewer.shards_nodes;
    let indices = &app.viewer.shards_indices;

    if !app.viewer.shards_loaded || indices.is_empty() {
        let title = format!(
            " DB Shards [{}] {} ",
            app.viewer.shards_show.label(),
            if !app.viewer.shards_loaded { "" } else { "No shards" }
        );
        let block = Block::default().borders(Borders::ALL).title(title);
        f.render_widget(block, area);

        // Still render mode bar on bottom border
        let mode_line: Vec<Span> = ShardsShow::ALL.iter().map(|m| {
            if *m == app.viewer.shards_show {
                Span::styled(format!(" {} ", m.label()), Style::default().fg(Color::Black).bg(Color::Cyan))
            } else {
                Span::styled(format!(" {} ", m.label()), Style::default().fg(Color::DarkGray))
            }
        }).collect();
        let mode_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2).min(60),
            height: 1,
        };
        f.render_widget(Line::from(mode_line), mode_area);
        return;
    }

    // Compute index column width: longest index name + 1, min 8
    let index_col_width: u16 = indices.iter()
        .map(|n| n.len() as u16)
        .max()
        .unwrap_or(5)
        .saturating_add(1)
        .max(8);

    // Compute node column widths based on name lengths + 1, min 6
    let node_widths: Vec<u16> = nodes.iter()
        .map(|n| (n.len() as u16).saturating_add(1).max(6))
        .collect();

    // Figure out how many nodes fit in the visible area
    let available_width = area.width.saturating_sub(index_col_width + 3); // borders + index col
    let mut max_visible_nodes = 0;
    let mut used_width: u16 = 0;
    for w in node_widths.iter().skip(app.viewer.shards_hscroll) {
        if used_width + w > available_width { break; }
        used_width += w;
        max_visible_nodes += 1;
    }
    if max_visible_nodes == 0 && !nodes.is_empty() { max_visible_nodes = 1; }
    let hscroll = app.viewer.shards_hscroll.min(nodes.len().saturating_sub(max_visible_nodes));
    app.viewer.shards_hscroll = hscroll;

    let visible_nodes: Vec<&String> = nodes.iter().skip(hscroll).take(max_visible_nodes).collect();
    let visible_node_widths: Vec<u16> = node_widths.iter().skip(hscroll).take(max_visible_nodes).copied().collect();
    let visible_rows = area.height.saturating_sub(4) as usize; // borders + header + mode bar
    let scroll_top = if app.viewer.shards_selected_row >= visible_rows {
        app.viewer.shards_selected_row - visible_rows + 1
    } else {
        0
    };

    // Mode bar at top
    let mode_line = ShardsShow::ALL.iter().map(|m| {
        if *m == app.viewer.shards_show {
            Span::styled(format!(" {} ", m.label()), Style::default().fg(Color::Black).bg(Color::Cyan))
        } else {
            Span::styled(format!(" {} ", m.label()), Style::default().fg(Color::DarkGray))
        }
    }).collect::<Vec<_>>();

    let title = format!(
        " DB Shards [{} indices, {} nodes] (m: mode, ←→: scroll) ",
        indices.len(), nodes.len()
    );

    // Build header row
    let mut header_cells = vec![
        Cell::from(Line::from("Index").alignment(Alignment::Left))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ];
    for node_name in &visible_nodes {
        let style = if node_name.as_str() == "Unassigned" {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        };
        header_cells.push(Cell::from(node_name.as_str()).style(style));
    }
    let header = Row::new(header_cells).height(1);

    // Access indices array from shards_data
    let indices_arr = app.viewer.shards_data.get("indices")
        .and_then(|i| i.as_array());

    // Build data rows
    let rows: Vec<Row> = indices.iter().enumerate()
        .skip(scroll_top)
        .take(visible_rows)
        .map(|(row_idx, index_name)| {
            let mut cells = vec![
                Cell::from(index_name.as_str())
                    .style(Style::default().fg(Color::White)),
            ];

            // Find this index in the data
            let index_data = indices_arr.and_then(|arr| {
                arr.iter().find(|idx| idx.get("name").and_then(|n| n.as_str()) == Some(index_name))
            });

            for node_name in &visible_nodes {
                let cell_text = if let Some(idx) = index_data {
                    if let Some(node_shards) = idx.get("nodes")
                        .and_then(|n| n.get(node_name.as_str()))
                        .and_then(|s| s.as_array())
                    {
                        // Build shard badges
                        let parts: Vec<Span> = node_shards.iter().map(|shard| {
                            let num = shard.get("shard").map(|s| {
                                if let Some(n) = s.as_u64() { n.to_string() }
                                else { s.as_str().unwrap_or("?").to_string() }
                            }).unwrap_or_else(|| "?".to_string());
                            let prirep = shard.get("prirep").and_then(|s| s.as_str()).unwrap_or("");
                            let state = shard.get("state").and_then(|s| s.as_str()).unwrap_or("");

                            let style = if state != "STARTED" {
                                Style::default().fg(Color::White).bg(Color::Red)
                            } else if prirep == "p" {
                                Style::default().fg(Color::White).bg(Color::Blue)
                            } else {
                                Style::default().fg(Color::White).bg(Color::DarkGray)
                            };
                            Span::styled(num, style)
                        }).collect();

                        // Join with spaces
                        let mut spans = Vec::new();
                        for (i, part) in parts.into_iter().enumerate() {
                            if i > 0 { spans.push(Span::raw(" ")); }
                            spans.push(part);
                        }
                        Cell::from(Line::from(spans))
                    } else {
                        Cell::from("")
                    }
                } else {
                    Cell::from("")
                };
                cells.push(cell_text);
            }

            let row = Row::new(cells);
            if row_idx == app.viewer.shards_selected_row {
                row.style(Style::default().bg(Color::DarkGray))
            } else {
                row
            }
        })
        .collect();

    // Build column widths
    let mut widths = vec![Constraint::Length(index_col_width)];
    for w in &visible_node_widths {
        widths.push(Constraint::Length(*w));
    }

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(table, area);

    // Render mode bar on top of the table border (last line of area)
    let mode_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(1),
        width: area.width.saturating_sub(2).min(60),
        height: 1,
    };
    let mode_bar = Line::from(mode_line);
    f.render_widget(mode_bar, mode_area);

    // Render shards detail overlay if open
    if app.viewer.shards_detail.is_some() {
        draw_shards_detail(f, app, area);
    }
    // Render sub-detail (single shard + explain) on top
    if app.viewer.shards_sub_detail.is_some() {
        draw_shards_sub_detail(f, app, area);
    }
}

fn draw_shards_detail(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::Clear;

    let detail = match &app.viewer.shards_detail {
        Some(d) => d,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.85) as u16;
    let popup_height = (area.height as f32 * 0.85) as u16;
    let popup_area = crate::ui::center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let index_name = detail.data.get("index").and_then(|v| v.as_str()).unwrap_or("?");
    let shards = detail.data.get("shards").and_then(|v| v.as_array());
    let filter_lower = detail.filter.to_lowercase();

    // Column definitions: label, field key, width, right-align
    let columns: &[(&str, &str, u16, bool)] = &[
        ("Node", "node", 28, false),
        ("Shard", "shard", 6, true),
        ("P/R", "prirep", 4, false),
        ("State", "state", 14, false),
        ("Docs", "docs", 12, true),
        ("Store", "store", 10, true),
        ("IP", "ip", 16, false),
        ("Segment", "segmentCount", 8, true),
    ];

    // Build rows from shard data
    struct ShardRow {
        spans: Vec<(String, Style)>,
    }
    let mut rows: Vec<ShardRow> = Vec::new();
    if let Some(shard_arr) = shards {
        for shard in shard_arr {
            let row_text: String = columns.iter().map(|(_, key, _, _)| {
                match shard.get(*key) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(v) => v.to_string(),
                    None => String::new(),
                }
            }).collect::<Vec<_>>().join(" ");

            if !filter_lower.is_empty() && !row_text.to_lowercase().contains(&filter_lower) {
                continue;
            }

            let prirep = shard.get("prirep").and_then(|v| v.as_str()).unwrap_or("");
            let state = shard.get("state").and_then(|v| v.as_str()).unwrap_or("");

            let fg = if state != "STARTED" {
                Color::Red
            } else if prirep == "p" {
                Color::Cyan
            } else {
                Color::White
            };

            let mut cells = Vec::new();
            for (_, key, width, right) in columns.iter() {
                let val = match shard.get(*key) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(v) => v.to_string(),
                    None => String::new(),
                };
                let w = *width as usize;
                let formatted = if *right { format!("{:>w$}", val) } else { format!("{:<w$}", val) };
                cells.push((formatted, Style::default().fg(fg)));
            }
            rows.push(ShardRow { spans: cells });
        }
    }

    let inner = popup_area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let content_height = inner.height.saturating_sub(1) as usize; // -1 for header
    let total = rows.len();
    let selected = (detail.scroll as usize).min(total.saturating_sub(1));

    // Compute scroll window
    let scroll_top = if selected >= content_height {
        selected - content_height + 1
    } else {
        0
    };

    // Build header
    let mut header_spans = Vec::new();
    for (i, (label, _, width, right)) in columns.iter().enumerate() {
        if i > 0 { header_spans.push(Span::raw(" ")); }
        let w = *width as usize;
        let formatted = if *right { format!("{:>w$}", label) } else { format!("{:<w$}", label) };
        header_spans.push(Span::styled(formatted, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    }

    // Build visible lines
    let mut visible_lines: Vec<Line> = vec![Line::from(header_spans)];
    for (i, row) in rows.iter().enumerate().skip(scroll_top).take(content_height) {
        let is_selected = i == selected;
        let mut spans = Vec::new();
        for (j, (text, style)) in row.spans.iter().enumerate() {
            if j > 0 { spans.push(Span::raw(" ")); }
            let s = if is_selected {
                Style::default().fg(style.fg.unwrap_or(Color::White)).bg(Color::DarkGray)
            } else {
                *style
            };
            spans.push(Span::styled(text.clone(), s));
        }
        visible_lines.push(Line::from(spans));
    }

    let shard_count = shards.map(|a| a.len()).unwrap_or(0);
    let title = if !detail.filter.is_empty() {
        format!(" {} [{} shards] filter: {} ", index_name, shard_count, detail.filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        format!(" {} [{} shards] filter: ", index_name, shard_count)
    } else {
        format!(" {} [{} shards] (/ filter, Esc close) ", index_name, shard_count)
    };
    let position_info = if total > 0 {
        format!(" {}/{} ", selected + 1, total)
    } else {
        " 0/0 ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(position_info).alignment(Alignment::Right))
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(visible_lines).block(block);
    f.render_widget(paragraph, popup_area);

    // Show cursor in filter mode
    if app.input_mode == crate::app::InputMode::DetailFilter {
        let filter_x = popup_area.x + 1 + index_name.len() as u16 + format!(" [{} shards] filter: ", shard_count).len() as u16 + detail.filter_cursor as u16;
        let filter_y = popup_area.y;
        if filter_x < popup_area.x + popup_area.width - 1 {
            f.set_cursor_position((filter_x, filter_y));
        }
    }
}

fn draw_shards_sub_detail(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::Clear;

    let detail = match &app.viewer.shards_sub_detail {
        Some(d) => d,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.9) as u16;
    let popup_height = (area.height as f32 * 0.9) as u16;
    let popup_area = crate::ui::center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let obj = detail.data.as_object();
    let node = detail.data.get("node").and_then(|v| v.as_str()).unwrap_or("?");
    let shard_num = detail.data.get("shard").map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => "?".to_string(),
    }).unwrap_or_else(|| "?".to_string());
    let prirep = detail.data.get("prirep").and_then(|v| v.as_str()).unwrap_or("?");
    let pr_label = if prirep == "p" { "Primary" } else { "Replica" };

    let mut lines: Vec<Line> = Vec::new();

    // Shard fields (everything except _explain)
    lines.push(Line::from(vec![
        Span::styled("── Shard Info ──", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));
    if let Some(obj) = obj {
        let mut keys: Vec<&String> = obj.keys().filter(|k| *k != "_explain").collect();
        keys.sort();
        for key in keys {
            let val = &obj[key];
            let val_str = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => "null".to_string(),
                _ => val.to_string(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{:>20}: ", key), Style::default().fg(Color::Yellow)),
                Span::raw(val_str),
            ]));
        }
    }

    // Explain section
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("── Allocation Explain ──", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));
    if let Some(explain) = detail.data.get("_explain") {
        if let Some(err) = explain.get("error").and_then(|v| v.as_str()) {
            lines.push(Line::from(Span::styled(format!("  Error: {err}"), Style::default().fg(Color::Red))));
        } else {
            let pretty = serde_json::to_string_pretty(explain).unwrap_or_else(|_| explain.to_string());
            for line in pretty.lines() {
                lines.push(Line::from(Span::raw(format!("  {line}"))));
            }
        }
    }

    let inner = popup_area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let content_height = inner.height as usize;
    let total = lines.len();
    let scroll = (detail.scroll as usize).min(total.saturating_sub(content_height));

    let title = format!(" Shard {} {} on {} (Esc close) ", shard_num, pr_label, node);
    let position_info = format!(" {}/{} ", scroll + 1, total);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(position_info).alignment(Alignment::Right))
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, popup_area);
}

fn draw_capture_graphs(f: &mut Frame, app: &mut App, area: Rect) {
    if app.viewer.cg_nodes.is_empty() {
        let msg = if app.viewer.cg_loaded {
            "No nodes found"
        } else {
            "Press 'r' to fetch data or wait for auto-load"
        };
        let block = Block::default().borders(Borders::ALL).title(" Capture Graphs ");
        let paragraph = Paragraph::new(msg)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    let metric = CAPTURE_GRAPH_METRICS[app.viewer.cg_metric_index];
    let node_count = app.viewer.cg_nodes.len();

    // 1 row per node (horizon chart style)
    let inner_height = area.height.saturating_sub(2); // top+bottom border
    let visible_count = inner_height.max(1) as usize;
    let scroll = app.viewer.cg_scroll.min(node_count.saturating_sub(visible_count));
    app.viewer.cg_scroll = scroll;

    // Find the label width (longest node name)
    let label_width = app.viewer.cg_nodes.iter()
        .map(|n| n.node_name.len())
        .max()
        .unwrap_or(8)
        .max(8) as u16 + 1;

    // Build mode bar spans for bottom border
    let mode_spans = vec![
        Span::styled(
            format!(" m:{} ", metric.label),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("i:{}", app.viewer.cg_interval.label()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("H:{}", app.viewer.cg_hide.label()),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {}-{} of {} ", scroll + 1, (scroll + visible_count).min(node_count), node_count)),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Capture Graphs: {} ", metric.label))
        .title_bottom(Line::from(mode_spans).alignment(Alignment::Left))
        .border_style(Style::default().fg(Color::White));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Graph width: each braille char = 2 data points
    let graph_char_width = inner.width.saturating_sub(label_width) as usize;
    if graph_char_width < 2 {
        return;
    }
    let graph_data_width = graph_char_width * 2;

    // Find global max across ALL nodes for consistent scale
    let global_max = app.viewer.cg_nodes.iter()
        .flat_map(|n| n.values.iter())
        .cloned()
        .fold(0.0f64, f64::max);

    // Horizon band colors (cubism-style, increasing intensity)
    let band_colors: [Color; 4] = [
        Color::Rgb(20, 60, 100),   // darkest blue
        Color::Rgb(40, 130, 190),  // medium blue
        Color::Rgb(70, 190, 235),  // light blue
        Color::Rgb(110, 230, 255), // brightest cyan
    ];

    let visible_nodes = &app.viewer.cg_nodes[scroll..(scroll + visible_count).min(node_count)];

    for (i, node) in visible_nodes.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        // Alternating subtle background for row separation
        let row_bg = if i % 2 == 1 { Some(Color::Rgb(25, 25, 35)) } else { None };

        // Node name label
        let label_text = if node.node_name.len() > label_width as usize {
            &node.node_name[..label_width as usize]
        } else {
            &node.node_name
        };
        let mut label_style = Style::default().fg(Color::Yellow);
        if let Some(bg) = row_bg { label_style = label_style.bg(bg); }
        let label = Paragraph::new(label_text.to_string()).style(label_style);
        f.render_widget(label, Rect::new(inner.x, y, label_width, 1));

        // Downsample values to graph_data_width points
        let values = &node.values;
        let num_points = values.len();
        let mut columns = vec![0.0f64; graph_data_width];
        if num_points > 0 {
            for (ci, col) in columns.iter_mut().enumerate() {
                let start_idx = ci * num_points / graph_data_width;
                let end_idx = ((ci + 1) * num_points / graph_data_width).min(num_points);
                if end_idx > start_idx {
                    let sum: f64 = values[start_idx..end_idx].iter().sum();
                    *col = sum / (end_idx - start_idx) as f64;
                }
            }
        }

        // Build braille characters with horizon-chart band coloring
        let mut spans = Vec::with_capacity(graph_char_width);
        for ci in 0..graph_char_width {
            let left_val = columns.get(ci * 2).copied().unwrap_or(0.0);
            let right_val = columns.get(ci * 2 + 1).copied().unwrap_or(0.0);

            let (left_band, left_h) = horizon_band_height(left_val, global_max);
            let (right_band, right_h) = horizon_band_height(right_val, global_max);

            let ch = braille_from_heights(left_h, right_h);
            let color = band_colors[left_band.max(right_band) as usize];

            let mut style = Style::default().fg(color);
            if let Some(bg) = row_bg { style = style.bg(bg); }
            spans.push(Span::styled(String::from(ch), style));
        }

        let graph_area = Rect::new(inner.x + label_width, y, graph_char_width as u16, 1);
        f.render_widget(Paragraph::new(Line::from(spans)), graph_area);
    }

    // Draw metric popup if open
    if app.viewer.cg_show_metric_popup {
        draw_metric_popup(f, app, area);
    }
}

/// Horizon chart: divide value range into 4 bands, return (band_index, height_within_band).
/// Height is 0-4 (braille dots from bottom).
fn horizon_band_height(value: f64, max_val: f64) -> (u8, u8) {
    if max_val <= 0.0 || value <= 0.0 {
        return (0, 0);
    }
    let norm = (value / max_val).min(1.0);
    let band = (norm * 4.0).floor().min(3.0) as u8;
    let within = norm * 4.0 - band as f64;
    // ceil + max(1) ensures any positive value shows at least 1 dot
    let height = (within * 4.0).ceil().max(1.0).min(4.0) as u8;
    (band, height)
}

/// Build a braille character from left/right column fill heights (0-4 dots from bottom).
fn braille_from_heights(left: u8, right: u8) -> char {
    // Left column bottom-to-top: dot7(0x40), dot3(0x04), dot2(0x02), dot1(0x01)
    const LEFT: [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];
    // Right column bottom-to-top: dot8(0x80), dot6(0x20), dot5(0x10), dot4(0x08)
    const RIGHT: [u8; 5] = [0x00, 0x80, 0xA0, 0xB0, 0xB8];
    let bits = LEFT[left.min(4) as usize] | RIGHT[right.min(4) as usize];
    char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
}

fn draw_metric_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 40u16;
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Metric ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Filter input at top
    let filter_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));

    let filter_text = if app.viewer.cg_metric_popup_filter.is_empty() {
        Span::styled("Type to filter...", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.viewer.cg_metric_popup_filter, Style::default().fg(Color::White))
    };
    f.render_widget(Paragraph::new(filter_text), filter_area);

    // Filtered metrics list
    let filtered: Vec<(usize, &CaptureGraphMetric)> = CAPTURE_GRAPH_METRICS.iter().enumerate()
        .filter(|(_, m)| {
            if app.viewer.cg_metric_popup_filter.is_empty() {
                true
            } else {
                let f = app.viewer.cg_metric_popup_filter.to_lowercase();
                m.label.to_lowercase().contains(&f) || m.field.to_lowercase().contains(&f)
            }
        })
        .collect();

    let items: Vec<ListItem> = filtered.iter().enumerate()
        .map(|(i, (orig_idx, m))| {
            let marker = if *orig_idx == app.viewer.cg_metric_index { "● " } else { "  " };
            let style = if i == app.viewer.cg_metric_popup_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{}{}", marker, m.label)).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, list_area);
}
