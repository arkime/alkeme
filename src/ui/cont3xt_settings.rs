use super::*;
use super::arkime;
use crate::app::C3SettingsTab;

pub(super) fn c3_draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    // Split: sub-tab bar (3 lines) + content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Sub-tab bar
    let tab_titles: Vec<Line> = C3SettingsTab::ALL.iter()
        .map(|t| Line::from(t.name()))
        .collect();
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" Settings "))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .select(C3SettingsTab::ALL.iter().position(|t| *t == app.c3_settings_tab).unwrap_or(0));
    f.render_widget(tabs, chunks[0]);

    match app.c3_settings_tab {
        C3SettingsTab::Views => c3_draw_settings_views(f, app, chunks[1]),
        C3SettingsTab::Integrations => c3_draw_settings_integrations(f, app, chunks[1]),
        _ => {
            arkime::draw_under_construction(f, app, chunks[1]);
            arkime::draw_owl(f, app, chunks[1]);
        }
    }
}

fn c3_draw_settings_views(f: &mut Frame, app: &mut App, area: Rect) {
    // Filter bar (2 lines) + table
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Filter bar
    let filter_display = if app.c3_settings_views_filtering {
        format!("Filter: {}_", app.c3_settings_views_filter)
    } else if !app.c3_settings_views_filter.is_empty() {
        format!("Filter: {}", app.c3_settings_views_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_settings_filtered_views();

    let toolbar_text = format!(
        " {} views  {}  [n]ew  [e]dit  [d]elete  [/]filter  [r]efresh",
        filtered.len(),
        filter_display,
    );
    let toolbar = Paragraph::new(toolbar_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    if app.c3_settings_views_filtering {
        let cursor_x = chunks[0].x + 9 + app.c3_settings_views_filter.len() as u16;
        f.set_cursor_position((cursor_x, chunks[0].y));
    }

    // Table headers with sort indicators
    let col_names = [" Name", "Creator", "Integrations", "View Roles", "Edit Roles"];
    let header_cells: Vec<Cell> = col_names.iter().enumerate().map(|(i, &name)| {
        let label = sort_header_label(name, app.c3_settings_views_sort as usize == i, app.c3_settings_views_sort_desc);
        Cell::from(label).style(sort_header_style(app.c3_settings_views_sort as usize == i))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let view = &app.c3_settings_views[idx];
        let name_display = if view.editable {
            format!(" {}", view.name)
        } else {
            format!(" 🔗 {}", view.name)
        };
        Row::new(vec![
            Cell::from(name_display),
            Cell::from(view.creator.clone()),
            Cell::from(format!("{}", view.integrations.len())),
            Cell::from(if view.view_roles.is_empty() { "—".to_string() } else { view.view_roles.join(", ") }),
            Cell::from(if view.edit_roles.is_empty() { "—".to_string() } else { view.edit_roles.join(", ") }),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(20),
            Constraint::Length(14),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.c3_settings_views_table_state);

    // Confirm dialog overlay
    if let Some((_action, msg)) = &app.c3_settings_confirm {
        let popup_area = center_popup(50, 5, area);
        f.render_widget(Clear, popup_area);
        let text = format!("{}\n\n(y)es / (n)o", msg);
        let paragraph = Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" Confirm "),
            );
        f.render_widget(paragraph, popup_area);
    }
}

pub(super) fn c3_draw_view_editor(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::C3ViewEditorField;

    let popup_w = area.width.min(70);
    let popup_h = area.height.min(30);
    let popup_area = center_popup(popup_w, popup_h, area);
    f.render_widget(Clear, popup_area);

    let title = if app.c3_view_editor_id.is_some() { " Edit View " } else { " New View " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    f.render_widget(block, popup_area);

    let inner = Rect::new(popup_area.x + 1, popup_area.y + 1, popup_area.width - 2, popup_area.height - 2);

    // Layout: Name (2) + Integrations (remaining) + ViewRoles (2) + EditRoles (2) + Help (1)
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Name
            Constraint::Min(4),    // Integrations
            Constraint::Length(2),  // View Roles
            Constraint::Length(2),  // Edit Roles
            Constraint::Length(1),  // Help
        ])
        .split(inner);

    let active = app.c3_view_editor_field;

    // Name field
    let name_style = if active == C3ViewEditorField::Name {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_label = if active == C3ViewEditorField::Name {
        format!("▸ Name: {}", app.c3_view_editor_name)
    } else {
        format!("  Name: {}", app.c3_view_editor_name)
    };
    f.render_widget(Paragraph::new(name_label).style(name_style), sections[0]);

    if active == C3ViewEditorField::Name {
        let cursor_x = sections[0].x + 8 + app.c3_view_editor_name_cursor as u16;
        f.set_cursor_position((cursor_x, sections[0].y));
    }

    // Integrations section
    let int_style = if active == C3ViewEditorField::Integrations {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let enabled_count = app.c3_view_editor_integrations.iter().filter(|(_, e)| *e).count();
    let int_title = if app.c3_view_editor_integration_filtering {
        format!(" Integrations ({enabled_count}) — filter: {}_ ", app.c3_view_editor_integration_filter)
    } else if !app.c3_view_editor_integration_filter.is_empty() {
        format!(" Integrations ({enabled_count}) — filter: {} ", app.c3_view_editor_integration_filter)
    } else {
        format!(" Integrations ({enabled_count}/{}) ", app.c3_view_editor_integrations.len())
    };
    let int_block = Block::default()
        .borders(Borders::ALL)
        .border_style(int_style)
        .title(int_title);

    let int_inner = int_block.inner(sections[1]);
    f.render_widget(int_block, sections[1]);

    let filtered_ints = app.c3_view_editor_filtered_integrations();
    let scroll_offset = if active == C3ViewEditorField::Integrations {
        let selected = app.c3_view_editor_integration_selected;
        let visible = int_inner.height as usize;
        if selected >= visible { selected - visible + 1 } else { 0 }
    } else { 0 };

    for (row_idx, &real_idx) in filtered_ints.iter().skip(scroll_offset).enumerate() {
        if row_idx >= int_inner.height as usize { break; }
        let (name, enabled) = &app.c3_view_editor_integrations[real_idx];
        let marker = if *enabled { "✓" } else { "✗" };
        let is_selected = active == C3ViewEditorField::Integrations
            && app.c3_view_editor_integration_selected == scroll_offset + row_idx;
        let style = if is_selected {
            Style::default().bg(Color::DarkGray).fg(if *enabled { Color::Green } else { Color::Red })
        } else {
            Style::default().fg(if *enabled { Color::Green } else { Color::Red })
        };
        let line = format!(" {marker} {name}");
        f.render_widget(
            Paragraph::new(line).style(style),
            Rect::new(int_inner.x, int_inner.y + row_idx as u16, int_inner.width, 1),
        );
    }

    // View Roles
    let vr_style = if active == C3ViewEditorField::ViewRoles {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let vr_selected: Vec<&str> = app.c3_view_editor_view_roles.iter()
        .filter(|(_, s)| *s).map(|(r, _)| r.as_str()).collect();
    let vr_display = if vr_selected.is_empty() { "—".to_string() } else { vr_selected.join(", ") };
    let vr_label = if active == C3ViewEditorField::ViewRoles {
        format!("▸ View Roles: {vr_display}  [Enter to edit]")
    } else {
        format!("  View Roles: {vr_display}")
    };
    f.render_widget(Paragraph::new(vr_label).style(vr_style), sections[2]);

    // Edit Roles
    let er_style = if active == C3ViewEditorField::EditRoles {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let er_selected: Vec<&str> = app.c3_view_editor_edit_roles.iter()
        .filter(|(_, s)| *s).map(|(r, _)| r.as_str()).collect();
    let er_display = if er_selected.is_empty() { "—".to_string() } else { er_selected.join(", ") };
    let er_label = if active == C3ViewEditorField::EditRoles {
        format!("▸ Edit Roles: {er_display}  [Enter to edit]")
    } else {
        format!("  Edit Roles: {er_display}")
    };
    f.render_widget(Paragraph::new(er_label).style(er_style), sections[3]);

    // Help line
    let help = " Tab: next field  Ctrl+S: save  Esc: cancel";
    f.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        sections[4],
    );
}

pub(super) fn c3_draw_role_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = center_popup(40, 15, area);
    f.render_widget(Clear, popup_area);

    let title = if app.c3_role_popup_for_edit { " Edit Roles " } else { " View Roles " };
    let filter_info = if app.c3_role_popup_filtering {
        format!(" — filter: {}_ ", app.c3_role_popup_filter)
    } else if !app.c3_role_popup_filter.is_empty() {
        format!(" — filter: {} ", app.c3_role_popup_filter)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!("{title}{filter_info}"));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let roles = if app.c3_role_popup_for_edit {
        &app.c3_view_editor_edit_roles
    } else {
        &app.c3_view_editor_view_roles
    };

    let filtered = app.c3_role_popup_filtered_roles();
    let visible = inner.height.saturating_sub(1) as usize; // reserve 1 for help
    let scroll = if app.c3_role_popup_selected >= visible {
        app.c3_role_popup_selected - visible + 1
    } else { 0 };

    for (row_idx, &real_idx) in filtered.iter().skip(scroll).enumerate() {
        if row_idx >= visible { break; }
        let (name, selected) = &roles[real_idx];
        let marker = if *selected { "✓" } else { "✗" };
        let is_highlighted = row_idx + scroll == app.c3_role_popup_selected;
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

    // Help at bottom
    let help_y = inner.y + inner.height - 1;
    f.render_widget(
        Paragraph::new(" Space:toggle  a:all  n:none  /:filter  Esc:done")
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, help_y, inner.width, 1),
    );
}

fn c3_draw_settings_integrations(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.c3_int_settings_filtering {
        format!("Filter: {}_", app.c3_int_settings_filter)
    } else if !app.c3_int_settings_filter.is_empty() {
        format!("Filter: {}", app.c3_int_settings_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_int_settings_filtered();

    let dirty_indicator = if app.c3_int_settings_dirty { " [UNSAVED]" } else { "" };
    let toolbar_text = format!(
        " {} integrations  {}  [d]isable  [/]filter  [r]efresh  Ctrl+S:save{}",
        filtered.len(),
        filter_display,
        dirty_indicator,
    );
    let toolbar = Paragraph::new(toolbar_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    if app.c3_int_settings_filtering {
        let cursor_x = chunks[0].x + 1 + filtered.len().to_string().len() as u16 + 16 + app.c3_int_settings_filter.len() as u16;
        f.set_cursor_position((cursor_x, chunks[0].y));
    }

    let col_names = [" Name", "Status", "Fields"];
    let header_cells: Vec<Cell> = col_names.iter().enumerate().map(|(i, &name)| {
        let is_sorted = (i < 2) && app.c3_int_settings_sort as usize == i;
        let label = sort_header_label(name, is_sorted, app.c3_int_settings_sort_desc);
        Cell::from(label).style(sort_header_style(is_sorted))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let int = &app.c3_int_settings[idx];
        let status = if int.locked {
            "🔒 locked".to_string()
        } else if int.disabled {
            "🚫 disabled".to_string()
        } else if int.global_configed {
            "🌍 global".to_string()
        } else {
            let has_unset_required = int.fields.iter().any(|f| {
                f.required && int.values.get(&f.name).map_or(true, |v| v.is_empty())
            });
            if has_unset_required {
                " ✗ not configured".to_string()
            } else {
                " ✓ configured".to_string()
            }
        };
        let status_style = if int.locked {
            Style::default().fg(Color::DarkGray)
        } else if int.disabled {
            Style::default().fg(Color::Red)
        } else if int.global_configed {
            Style::default().fg(Color::Blue)
        } else {
            let has_unset_required = int.fields.iter().any(|f| {
                f.required && int.values.get(&f.name).map_or(true, |v| v.is_empty())
            });
            if has_unset_required {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        };
        Row::new(vec![
            Cell::from(format!(" {}", int.name)),
            Cell::from(status).style(status_style),
            Cell::from(format!("{}", int.fields.len())),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(20),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.c3_int_settings_table_state);
}

pub(super) fn c3_draw_int_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let idx = app.c3_int_editor_idx;
    let int = match app.c3_int_settings.get(idx) {
        Some(i) => i,
        None => return,
    };

    let field_count = app.c3_int_editor_values.len();
    let popup_height = (field_count as u16 + 6).min(area.height.saturating_sub(4)).max(8);
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let title = if int.locked {
        format!(" {} 🔒 ", int.name)
    } else {
        format!(" {} Settings ", int.name)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    if int.locked {
        let msg = Paragraph::new("This integration is locked by your administrator.\n\nPress Esc to close.")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, inner);
        return;
    }

    if field_count == 0 {
        let msg = Paragraph::new("No configurable fields.\n\nPress Esc to close.")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, inner);
        return;
    }

    // Layout: fields area + help line + footer
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let fields_area = content_chunks[0];
    let help_area = content_chunks[1];
    let footer_area = content_chunks[2];

    // Render fields
    let max_visible = fields_area.height as usize;
    let scroll_offset = if app.c3_int_editor_selected >= max_visible {
        app.c3_int_editor_selected - max_visible + 1
    } else {
        0
    };

    for (i, (field_name, value, is_password, is_boolean, required, _help)) in
        app.c3_int_editor_values.iter().enumerate().skip(scroll_offset).take(max_visible)
    {
        let y = fields_area.y + (i - scroll_offset) as u16;
        if y >= fields_area.y + fields_area.height {
            break;
        }
        let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);
        let is_selected = i == app.c3_int_editor_selected;
        let label_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let required_mark = if *required { "*" } else { " " };

        if *is_boolean {
            let check = if value == "true" { "x" } else { " " };
            let text = format!("{} [{}] {}", required_mark, check, field_name);
            f.render_widget(Paragraph::new(text).style(label_style), row_area);
        } else {
            let display_value = if *is_password && !app.c3_int_editor_show_password {
                "●".repeat(value.len())
            } else {
                value.clone()
            };
            let label_len = required_mark.len() + field_name.len() + 3; // "* field: "
            let max_val_width = row_area.width as usize - label_len.min(row_area.width as usize);
            let truncated = if display_value.len() > max_val_width {
                display_value[..max_val_width].to_string()
            } else {
                display_value.clone()
            };
            let text = format!("{} {}: {}", required_mark, field_name, truncated);
            f.render_widget(Paragraph::new(text).style(label_style), row_area);

            if is_selected && !*is_boolean {
                let cursor_x = row_area.x + label_len as u16;
                let cx = cursor_x + app.c3_int_editor_cursor.min(max_val_width) as u16;
                f.set_cursor_position((cx, y));
            }
        }
    }

    // Help text for selected field
    if let Some((_, _, _, _, _, help)) = app.c3_int_editor_values.get(app.c3_int_editor_selected) {
        f.render_widget(
            Paragraph::new(help.as_str()).style(Style::default().fg(Color::DarkGray)),
            help_area,
        );
    }

    // Footer
    f.render_widget(
        Paragraph::new(" ↑/↓:navigate  Space:toggle  p:passwords  Ctrl+S:save  Esc:close")
            .style(Style::default().fg(Color::DarkGray)),
        footer_area,
    );
}
