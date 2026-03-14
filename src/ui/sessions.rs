use super::*;

pub(super) fn draw_sessions(f: &mut Frame, app: &mut App, area: Rect) {
    draw_session_list(f, app, area);
    if app.viewer.session_view == SessionView::Detail {
        draw_session_detail(f, app, area);
        if app.viewer.detail_action_menu.is_some() {
            draw_detail_action_menu(f, app, area);
        }
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect) {
    // header row + borders = 3 lines overhead
    app.visible_rows = area.height.saturating_sub(3) as usize;
    let header_cells = app.viewer.columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let is_sorted = i == app.viewer.sort_column;
            let label = sort_header_label(&col.label, is_sorted, app.viewer.sort_desc);
            Cell::from(label).style(sort_header_style(is_sorted))
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .viewer.sessions
        .iter()
        .map(|session| {
            let cells = app.viewer.columns.iter().enumerate().map(|(col_idx, col)| {
                let val = session.get(&col.field).unwrap_or(&serde_json::Value::Null);
                let text = if col.field == "ipProtocol" && col_idx == 0 {
                    ip_protocol_str(val)
                } else if let Some(field_type) = app.viewer.date_fields.get(col.field.as_str()) {
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

    let widths: Vec<Constraint> = app.viewer.columns.iter()
        .map(|col| Constraint::Length(col.width))
        .collect();

    let end = (app.viewer.page_start + app.viewer.sessions.len() as u64).min(app.viewer.sessions_filtered);
    let view_label = if let Some(ref v) = app.viewer.active_view_name {
        format!(" [view: {}]", v)
    } else {
        String::new()
    };
    let page_label = if app.viewer.sessions_filtered > 0 {
        format!(" Sessions{} [{}-{} of {}] ◄ ► ", view_label, app.viewer.page_start + 1, end, app.viewer.sessions_filtered)
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

    f.render_stateful_widget(table, area, &mut app.viewer.table_state);
}

fn draw_session_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let detail = match &app.viewer.session_detail {
        Some(d) => d,
        None => return,
    };

    // Centered overlay: 80% width, 80% height
    let popup_width = (area.width as f32 * 0.8) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_area = center_popup(popup_width, popup_height, area);

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
                let friendly = app.viewer.field_friendly_map.get(k.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(k.as_str());
                k.to_lowercase().contains(&filter_lower)
                    || friendly.to_lowercase().contains(&filter_lower)
            })
            .collect();
        keys.sort();
        for (i, db_field) in keys.iter().enumerate() {
            let val = &obj[*db_field];
            let val_str = if let Some(field_type) = app.viewer.date_fields.get(db_field.as_str()) {
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
            let display_name = app.viewer.field_friendly_map.get(db_field.as_str())
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
            let pad_len = 30usize.saturating_sub(display_name.len());
            let mut spans = vec![Span::styled(" ".repeat(pad_len), key_style)];
            spans.extend(highlight_filter_spans(display_name, &filter_lower, key_style));
            spans.push(Span::styled(": ", key_style));
            spans.extend(highlight_filter_spans(&val_str, &filter_lower, val_style));
            lines.push(Line::from(spans));
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
    if let Some(ref mut d) = app.viewer.session_detail {
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

pub(super) fn draw_detail_action_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu = match &app.viewer.detail_action_menu {
        Some(m) => m,
        None => return,
    };

    if let Some(ref values) = menu.values {
        // Value selection sub-menu
        let popup_width = 40u16;
        let popup_height = (values.len() as u16) + 3;
        let popup_area = center_popup(popup_width, popup_height, area);

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
    let popup_area = center_popup(popup_width, popup_height, area);

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
