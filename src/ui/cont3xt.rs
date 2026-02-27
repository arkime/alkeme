use super::*;
use crate::api::{CardField, Cont3xtCard};

pub(super) fn draw_cont3xt(f: &mut Frame, app: &mut App) {
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

    let integrations_label = if let Some(ref name) = app.c3_active_view_name {
        format!("[view: {name}] ")
    } else if app.c3_disabled_integrations.is_empty() {
        "[all] ".to_string()
    } else {
        "[custom] ".to_string()
    };

    let inner_width = area.width.saturating_sub(2) as usize;
    let expr_scroll = if app.expression_cursor > inner_width {
        (app.expression_cursor - inner_width) as u16
    } else {
        0
    };
    let expr_widget = Paragraph::new(Span::styled(expr_display.as_str(), expr_style))
        .scroll((0, expr_scroll))
        .block(Block::default().borders(Borders::ALL).title(format!(" Search (/) {integrations_label}")));
    f.render_widget(expr_widget, area);

    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            area.x + (app.expression_cursor as u16 - expr_scroll) + 1,
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
        let text = if app.c3_searching {
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

    // Left pane: results tree grouped by indicator
    let results_focused = app.c3_focus == Cont3xtFocus::Results;
    let results_border_style = if results_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(results_border_style)
        .title(format!(" Results ({}) ", app.c3_results.len()));

    let inner = results_block.inner(horiz[0]);
    f.render_widget(results_block, horiz[0]);

    let visible_height = inner.height as usize;
    app.visible_rows = visible_height;

    // Build tree using parent-child indicator relationships
    // Group results by (itype, indicator)
    let mut indicator_results: std::collections::HashMap<(String, String), Vec<usize>> = std::collections::HashMap::new();
    let mut indicator_order: Vec<(String, String)> = Vec::new();

    // Start with init-ordered indicators as the canonical order
    for (itype, query) in &app.c3_init_indicators {
        let key = (itype.clone(), query.clone());
        if !indicator_order.contains(&key) {
            indicator_order.push(key);
        }
    }

    for (idx, result) in app.c3_results.iter().enumerate() {
        let key = (result.itype.clone(), result.indicator.clone());
        if !indicator_order.contains(&key) {
            indicator_order.push(key.clone());
        }
        indicator_results.entry(key).or_default().push(idx);
    }

    // Also ensure parent indicators exist in indicator_order even without results
    // This handles chains like URL -> DOMAIN -> IP where URL may have no direct results
    for ((_child_ind, _child_itype), parents) in &app.c3_indicator_parents {
        for (parent_query, parent_itype) in parents {
            let parent_key = (parent_itype.clone(), parent_query.clone());
            if !indicator_order.contains(&parent_key) {
                indicator_order.push(parent_key);
            }
        }
    }

    // Build a tree: find root indicators (those with no parent or whose parent is not in our set)
    let mut children_of: std::collections::HashMap<(String, String), Vec<(String, String)>> = std::collections::HashMap::new();
    let mut has_parent: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for key in &indicator_order {
        // c3_indicator_parents is keyed as (indicator, itype), convert to match our (itype, indicator)
        let lookup_key = (key.1.clone(), key.0.clone());
        if let Some(parents) = app.c3_indicator_parents.get(&lookup_key) {
            for parent in parents {
                // parent is (parent_query, parent_itype), convert to (itype, indicator)
                let parent_key = (parent.1.clone(), parent.0.clone());
                if indicator_results.contains_key(&parent_key) || indicator_order.contains(&parent_key) {
                    children_of.entry(parent_key).or_default().push(key.clone());
                    has_parent.insert(key.clone());
                }
            }
        }
    }

    // Recursive tree builder
    fn build_tree(
        key: &(String, String),
        depth: u16,
        children_of: &std::collections::HashMap<(String, String), Vec<(String, String)>>,
        indicator_results: &std::collections::HashMap<(String, String), Vec<usize>>,
        results: &[crate::api::Cont3xtResult],
        rows: &mut Vec<(u16, String, Option<usize>)>,
    ) {
        rows.push((depth, format!("{} {}", key.0.to_uppercase(), key.1), None));
        if let Some(indices) = indicator_results.get(key) {
            for &idx in indices {
                rows.push((depth + 2, results[idx].name.clone(), Some(idx)));
            }
        }
        if let Some(kids) = children_of.get(key) {
            for child in kids {
                build_tree(child, depth + 2, children_of, indicator_results, results, rows);
            }
        }
    }

    let mut display_rows: Vec<(u16, String, Option<usize>)> = Vec::new();
    // Only start from root indicators (not children)
    for key in &indicator_order {
        if !has_parent.contains(key) {
            build_tree(key, 0, &children_of, &indicator_results, &app.c3_results, &mut display_rows);
        }
    }

    // Build tree_order and tree_roots from display_rows
    let mut tree_order: Vec<usize> = Vec::new();
    let mut tree_roots: Vec<usize> = Vec::new();
    let mut last_root_depth = false;
    for (depth, _, idx) in &display_rows {
        if *depth == 0 {
            last_root_depth = true;
        }
        if let Some(result_idx) = idx {
            if last_root_depth {
                tree_roots.push(tree_order.len());
                last_root_depth = false;
            }
            tree_order.push(*result_idx);
        }
    }
    app.c3_tree_order = tree_order;
    app.c3_tree_roots = tree_roots;

    // The actual result index for the current selection
    let selected_result_idx = app.c3_tree_order.get(app.c3_selected).copied();

    // Find which display row corresponds to c3_selected
    let selected_display_row = display_rows.iter()
        .position(|(_, _, idx)| *idx == selected_result_idx)
        .unwrap_or(0);

    // Scroll to keep selected visible
    let scroll_offset = if selected_display_row >= visible_height {
        selected_display_row - visible_height + 1
    } else {
        0
    };

    for (row_i, (indent, label, result_idx)) in display_rows.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let y = inner.y + (row_i - scroll_offset) as u16;
        if y >= inner.y + inner.height { break; }

        let is_header = result_idx.is_none();
        let is_selected = *result_idx == selected_result_idx && result_idx.is_some();

        let style = if is_header {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if is_selected && results_focused {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = " ".repeat(*indent as usize);
        let full_label = format!("{prefix}{label}");
        let truncated = if full_label.len() > inner.width as usize {
            format!("{}…", &full_label[..inner.width as usize - 1])
        } else {
            format!("{:<width$}", full_label, width = inner.width as usize)
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

    let actual_result_idx = app.c3_tree_order.get(app.c3_selected).copied().unwrap_or(0);
    if let Some(result) = app.c3_results.get(actual_result_idx) {
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

fn c3_draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
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
        let style = if i == app.c3_stats_sort_col {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        };
        Cell::from(text).style(style)
    }).collect();
    let header = Row::new(header_cells).height(1);

    // Build rows
    let rows: Vec<Row> = filtered.iter().map(|item| {
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

        Row::new(cells)
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
        .row_highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    app.c3_stats_table_state.select(Some(app.c3_stats_selected));
    f.render_stateful_widget(table, chunks[1], &mut app.c3_stats_table_state);
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

    let link_result_idx = app.c3_tree_order.get(app.c3_selected).copied().unwrap_or(0);
    let (indicator, itype) = app.c3_results.get(link_result_idx)
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

    // Reserve bottom for selected link description
    let selected_info = app.c3_link_flat.get(app.c3_link_popup_selected)
        .map(|(_, _, url, info)| (url.clone(), info.clone()))
        .unwrap_or_default();
    let has_desc = !selected_info.0.is_empty();
    let desc_height = if has_desc { 3u16 } else { 0 };
    let list_area = Rect {
        x: content_area.x, y: content_area.y,
        width: content_area.width,
        height: content_area.height.saturating_sub(desc_height),
    };
    let desc_area = Rect {
        x: content_area.x, y: list_area.y + list_area.height,
        width: content_area.width,
        height: desc_height.min(content_area.height),
    };

    // Scrolling: keep selected in view
    let visible = list_area.height as usize;
    let selected = app.c3_link_popup_selected;
    let scroll_offset = if selected >= visible {
        selected - visible + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut last_group = String::new();
    for (i, (group, name, url, _info)) in app.c3_link_flat.iter().enumerate().skip(scroll_offset) {
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

    f.render_widget(Paragraph::new(lines), list_area);

    // Render description of selected link
    if has_desc {
        let mut desc_lines: Vec<Line> = Vec::new();
        desc_lines.push(Line::from(Span::styled(
            "─".repeat(desc_area.width as usize),
            Style::default().fg(Color::DarkGray),
        )));
        let url_line = format!("URL: {}", selected_info.0);
        let w = desc_area.width as usize;
        desc_lines.push(Line::from(Span::styled(
            if url_line.len() > w { format!("{}…", &url_line[..w.saturating_sub(1)]) } else { url_line },
            Style::default().fg(Color::Gray),
        )));
        if !selected_info.1.is_empty() {
            desc_lines.push(Line::from(Span::styled(
                selected_info.1.clone(),
                Style::default().fg(Color::Green),
            )));
        }
        f.render_widget(Paragraph::new(desc_lines), desc_area);
    }
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
