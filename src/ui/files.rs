use super::*;
use super::stats::{get_nested_value, format_stats_cell_dynamic, format_stats_value};

pub(super) fn draw_files_toolbar(f: &mut Frame, app: &App, area: Rect) {
    let total = app.viewer.files_filtered as usize;
    let start = app.viewer.files_page_start;
    let end = (start + app.viewer.files_data.len()).min(total);
    let page_info = if total > 0 {
        format!(" Files [{}-{} of {}] ", start + 1, end, total)
    } else {
        " Files ".to_string()
    };

    let toolbar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),    // filter
            Constraint::Length(page_info.len() as u16 + 4), // pagination
        ])
        .split(area);

    let filter_display = if app.input_mode == InputMode::Expression {
        &app.viewer.files_filter_edit
    } else {
        &app.viewer.files_filter
    };
    let is_editing = app.input_mode == InputMode::Expression;
    render_text_input(f, filter_display, app.expression_cursor, is_editing, " Filter (/) ", toolbar_chunks[0]);

    let nav = Paragraph::new(Line::from(vec![
        Span::styled("◄ ", Style::default().fg(if start > 0 { Color::Cyan } else { Color::DarkGray })),
        Span::raw(page_info.trim()),
        Span::styled(" ►", Style::default().fg(if end < total { Color::Cyan } else { Color::DarkGray })),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(nav, toolbar_chunks[1]);
}

pub(super) fn draw_files(f: &mut Frame, app: &mut App, area: Rect) {
    draw_files_list(f, app, area);
    if app.viewer.files_view == crate::app::StatsView::Detail {
        draw_files_detail(f, app, area);
    }
}

fn draw_files_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = app.viewer.files_columns.clone();

    let header_cells = columns.iter().enumerate().map(|(i, col)| {
        let is_sorted = i == app.viewer.files_sort_column;
        let text = sort_header_label(&col.label, is_sorted, app.viewer.files_sort_desc);
        let style = sort_header_style(is_sorted);
        let line = if col.is_numeric() {
            Line::from(text).alignment(Alignment::Right)
        } else {
            Line::from(text)
        };
        Cell::from(line).style(style)
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app.viewer.files_data.iter().map(|item| {
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

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Files [{} total] ", app.viewer.files_total)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    app.visible_rows = area.height.saturating_sub(4) as usize;
    f.render_stateful_widget(table, area, &mut app.viewer.files_table_state);
}

fn draw_files_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail = match &app.viewer.files_detail {
        Some(d) => d,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.8) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    let filter_lower = detail.filter.to_lowercase();

    let all_cols = crate::app::files_all_columns();
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
        format!(" File Detail [filter: {}] ", detail.filter)
    } else if app.input_mode == crate::app::InputMode::DetailFilter {
        " File Detail [filter: ] ".to_string()
    } else {
        " File Detail (/ filter, Esc close) ".to_string()
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
