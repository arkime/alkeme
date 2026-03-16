use ratatui::{
    prelude::*,
    widgets::*,
};

use super::*;

pub fn draw_users_tab(f: &mut Frame, app: &mut App, area: Rect) {
    draw_users_list(f, app, area);
    // Editor and role popup are rendered as overlays in ui::draw() so they
    // aren't captured by the cont3xt popup background cache.
}

fn draw_users_list(f: &mut Frame, app: &mut App, area: Rect) {
    let page_end = (app.us_page_start + app.us_users.len()).min(app.us_filtered);
    let page_info = if app.us_filtered > 0 {
        format!(" [{}-{} of {}]", app.us_page_start + 1, page_end, app.us_filtered)
    } else {
        " [0 users]".to_string()
    };

    let filter_display = if app.input_mode == InputMode::Expression && app.active_tab == Tab::Users {
        format!(" Filter: {}", app.expression_edit)
    } else if !app.us_filter.is_empty() {
        format!(" Filter: {}", app.us_filter)
    } else {
        String::new()
    };

    let columns: Vec<(&str, &str, u16)> = vec![
        ("User ID", "userId", 20),
        ("User Name", "userName", 25),
        ("Enabled", "enabled", 8),
        ("Web", "webEnabled", 5),
        ("Roles", "roles", 30),
        ("Last Used", "lastUsed", 20),
    ];

    let header_cells: Vec<Cell> = columns.iter().map(|(label, field, _w)| {
        let is_sorted = *field == app.us_sort_field.as_str();
        let display = if is_sorted {
            format!("{} {}", label, if app.us_sort_desc { "▼" } else { "▲" })
        } else {
            label.to_string()
        };
        Cell::from(display).style(sort_header_style(is_sorted))
    }).collect();

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.us_users.iter().map(|user| {
        let user_id = user.get("userId").and_then(|v| v.as_str()).unwrap_or("");
        let user_name = user.get("userName").and_then(|v| v.as_str()).unwrap_or("");
        let enabled = user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let web = user.get("webEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let roles = user.get("roles").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let last_used = user.get("lastUsed").and_then(|v| v.as_f64())
            .map(|ms| {
                chrono::DateTime::from_timestamp((ms / 1000.0) as i64, 0)
                    .map(|dt| {
                        let local: chrono::DateTime<chrono::Local> = dt.into();
                        local.format("%Y/%m/%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let enabled_style = if enabled { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
        let web_style = if web { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };

        Row::new(vec![
            Cell::from(user_id.to_string()),
            Cell::from(user_name.to_string()),
            Cell::from(if enabled { "Yes" } else { "No" }).style(enabled_style),
            Cell::from(if web { "Yes" } else { "No" }).style(web_style),
            Cell::from(roles),
            Cell::from(last_used),
        ])
    }).collect();

    let title = format!(" Users{}{} ", page_info, filter_display);

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(25),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Min(20),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol("▸ ");

    let mut table_state = TableState::default().with_selected(app.us_selected);
    f.render_stateful_widget(table, area, &mut table_state);
    app.us_table_state = table_state;
}

pub(super) fn draw_users_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let fields = crate::app::App::us_editor_fields();
    let popup_height = (fields.len() as u16 + 4).min(area.height.saturating_sub(2));
    let popup_width = 72u16.min(area.width.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let user_id = app.us_editor_user.get("userId").and_then(|v| v.as_str()).unwrap_or("?");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Edit User: {} ", user_id))
        .title_bottom(Line::from(" Tab/↑↓:navigate  Space:toggle  Ctrl+S:save  Esc:cancel ").fg(Color::DarkGray));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let visible_rows = inner.height as usize;
    let scroll = if app.us_editor_field >= visible_rows {
        app.us_editor_field - visible_rows + 1
    } else {
        0
    };

    let label_w = 22u16;

    for (i, (field_name, field_type)) in fields.iter().enumerate().skip(scroll).take(visible_rows) {
        let y = inner.y + (i - scroll) as u16;
        if y >= inner.y + inner.height { break; }
        let row_area = Rect::new(inner.x, y, inner.width, 1);
        let is_active = i == app.us_editor_field;
        let is_readonly = *field_name == "userId";

        let friendly = match *field_name {
            "userId" => "User ID",
            "userName" => "User Name",
            "enabled" => "Enabled",
            "webEnabled" => "Web Enabled",
            "headerAuthEnabled" => "Header Auth",
            "emailSearch" => "Email Search",
            "removeEnabled" => "Remove Enabled",
            "packetSearch" => "Packet Search",
            "hideStats" => "Hide Stats",
            "hideFiles" => "Hide Files",
            "hidePcap" => "Hide PCAP",
            "disablePcapDownload" => "Disable PCAP DL",
            "expression" => "Expression",
            "timeLimit" => "Time Limit (hrs)",
            "roles" => "Roles",
            _ => field_name,
        };

        let indicator = if is_active { "▸ " } else { "  " };

        if *field_type == "bool" {
            let val = app.us_editor_user.get(field_name)
                .and_then(|v| v.as_bool()).unwrap_or(false);
            let checkbox = if val { "☑ Yes" } else { "☐ No" };
            let style = if is_active {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let text = format!("{}{:<20}{}", indicator, format!("{}:", friendly), checkbox);
            f.render_widget(Paragraph::new(text).style(style), row_area);
        } else if *field_type == "roles" {
            let roles_display = app.us_editor_user.get("roles")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            let val_style = if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let hint = if is_active { " (Enter to edit)" } else { "" };
            let padded_label = format!("{}{:<20}", indicator, format!("{}:", friendly));
            let line = Line::from(vec![
                Span::styled(padded_label, Style::default().fg(Color::Cyan)),
                Span::styled(roles_display, val_style),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ]);
            f.render_widget(Paragraph::new(line), row_area);
        } else {
            let value = if is_active && !is_readonly {
                app.us_editor_text.clone()
            } else if *field_name == "timeLimit" {
                app.us_editor_user.get("timeLimit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string())
                    .unwrap_or("0".to_string())
            } else {
                app.us_editor_user.get(field_name)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };

            let val_style = if is_readonly {
                Style::default().fg(Color::DarkGray)
            } else if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let padded_label = format!("{}{:<20}", indicator, format!("{}:", friendly));
            let line = Line::from(vec![
                Span::styled(padded_label, Style::default().fg(Color::Cyan)),
                Span::styled(value, val_style),
            ]);
            f.render_widget(Paragraph::new(line), row_area);

            if is_active && !is_readonly {
                let cursor = app.us_editor_cursor.min(app.us_editor_text.len());
                let cx = row_area.x + label_w + cursor as u16;
                if cx < row_area.x + row_area.width {
                    f.set_cursor_position((cx, y));
                }
            }
        }
    }
}

pub(super) fn draw_users_role_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = center_popup(44, 18, area);
    f.render_widget(Clear, popup_area);

    let filter_info = if app.us_role_popup_filtering {
        format!(" filter: {}_ ", app.us_role_popup_filter)
    } else if !app.us_role_popup_filter.is_empty() {
        format!(" filter: {} ", app.us_role_popup_filter)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" Edit Roles{} ", filter_info));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let filtered = app.us_role_popup_filtered();
    let visible = inner.height.saturating_sub(1) as usize;
    let scroll = if app.us_role_popup_selected >= visible {
        app.us_role_popup_selected - visible + 1
    } else { 0 };

    for (row_idx, &real_idx) in filtered.iter().skip(scroll).enumerate() {
        if row_idx >= visible { break; }
        let (name, selected) = &app.us_editor_roles[real_idx];
        let marker = if *selected { "✓" } else { "✗" };
        let is_highlighted = row_idx + scroll == app.us_role_popup_selected;
        let style = if is_highlighted {
            Style::default().bg(Color::DarkGray).fg(if *selected { Color::Green } else { Color::Red })
        } else {
            Style::default().fg(if *selected { Color::Green } else { Color::Red })
        };
        f.render_widget(
            Paragraph::new(format!(" {marker} {name}")).style(style),
            Rect::new(inner.x, inner.y + row_idx as u16, inner.width, 1),
        );
    }

    let help_y = inner.y + inner.height - 1;
    f.render_widget(
        Paragraph::new(" Space:toggle  a:all  n:none  !:invert  /:filter  Esc:done")
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, help_y, inner.width, 1),
    );
}
