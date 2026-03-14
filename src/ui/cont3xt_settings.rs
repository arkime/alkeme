use super::*;
use crate::app::{C3SettingsTab, C3LinkGroupLevel, C3LinkEditorField, C3GroupEditorField, C3OverviewLevel, C3OverviewEditorField, C3OvFieldEditorField};

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
        .select(C3SettingsTab::ALL.iter().position(|t| *t == app.cont3xt.settings_tab).unwrap_or(0));
    f.render_widget(tabs, chunks[0]);

    match app.cont3xt.settings_tab {
        C3SettingsTab::Views => c3_draw_settings_views(f, app, chunks[1]),
        C3SettingsTab::Integrations => c3_draw_settings_integrations(f, app, chunks[1]),
        C3SettingsTab::LinkGroups => c3_draw_settings_link_groups(f, app, chunks[1]),
        C3SettingsTab::Overviews => c3_draw_settings_overviews(f, app, chunks[1]),
    }

    // Backup filename prompt overlay (shared across all settings tabs)
    if let Some(ref filename) = app.cont3xt.backup_prompt {
        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 3u16;
        let popup_area = center_popup(popup_width, popup_height, area);
        f.render_widget(Clear, popup_area);

        let title = app.cont3xt.backup_kind.title();
        let line = Line::from(vec![
            Span::styled("Filename: ", Style::default().fg(Color::Yellow)),
            Span::styled(filename, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ]);
        let paragraph = Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(title),
            );
        f.render_widget(paragraph, popup_area);
    }
}

fn c3_draw_settings_views(f: &mut Frame, app: &mut App, area: Rect) {
    // Filter bar (2 lines) + table
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Filter bar
    let filter_display = if app.cont3xt.settings_views_filtering {
        format!("Filter: {}_", app.cont3xt.settings_views_filter)
    } else if !app.cont3xt.settings_views_filter.is_empty() {
        format!("Filter: {}", app.cont3xt.settings_views_filter)
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

    if app.cont3xt.settings_views_filtering {
        let cursor_x = chunks[0].x + 9 + app.cont3xt.settings_views_filter.len() as u16;
        f.set_cursor_position((cursor_x, chunks[0].y));
    }

    // Table headers with sort indicators
    let col_names = [" Name", "Creator", "Integrations", "View Roles", "Edit Roles"];
    let header_cells: Vec<Cell> = col_names.iter().enumerate().map(|(i, &name)| {
        let label = sort_header_label(name, app.cont3xt.settings_views_sort as usize == i, app.cont3xt.settings_views_sort_desc);
        Cell::from(label).style(sort_header_style(app.cont3xt.settings_views_sort as usize == i))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let view = &app.cont3xt.settings_views[idx];
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

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.settings_views_table_state);

    // Confirm dialog overlay
    if let Some((_action, msg)) = &app.cont3xt.settings_confirm {
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

    let is_editable = app.cont3xt.view_editor_id.as_ref()
        .and_then(|id| app.cont3xt.settings_views.iter().find(|v| v.id == *id))
        .map(|v| v.editable).unwrap_or(true);
    let title = if app.cont3xt.view_editor_id.is_some() {
        if is_editable { " Edit View " } else { " View (read-only) " }
    } else { " New View " };
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

    let active = app.cont3xt.view_editor_field;

    // Name field
    let name_style = if active == C3ViewEditorField::Name {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_label = if active == C3ViewEditorField::Name {
        format!("▸ Name: {}", app.cont3xt.view_editor_name)
    } else {
        format!("  Name: {}", app.cont3xt.view_editor_name)
    };
    f.render_widget(Paragraph::new(name_label).style(name_style), sections[0]);

    if active == C3ViewEditorField::Name {
        let cursor_x = sections[0].x + 8 + app.cont3xt.view_editor_name_cursor as u16;
        f.set_cursor_position((cursor_x, sections[0].y));
    }

    // Integrations section
    let int_style = if active == C3ViewEditorField::Integrations {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let enabled_count = app.cont3xt.view_editor_integrations.iter().filter(|(_, e)| *e).count();
    let int_title = if app.cont3xt.view_editor_integration_filtering {
        format!(" Integrations ({enabled_count}) — filter: {}_ ", app.cont3xt.view_editor_integration_filter)
    } else if !app.cont3xt.view_editor_integration_filter.is_empty() {
        format!(" Integrations ({enabled_count}) — filter: {} ", app.cont3xt.view_editor_integration_filter)
    } else {
        format!(" Integrations ({enabled_count}/{}) ", app.cont3xt.view_editor_integrations.len())
    };
    let int_block = Block::default()
        .borders(Borders::ALL)
        .border_style(int_style)
        .title(int_title);

    let int_inner = int_block.inner(sections[1]);
    f.render_widget(int_block, sections[1]);

    let filtered_ints = app.c3_view_editor_filtered_integrations();
    let scroll_offset = if active == C3ViewEditorField::Integrations {
        let selected = app.cont3xt.view_editor_integration_selected;
        let visible = int_inner.height as usize;
        if selected >= visible { selected - visible + 1 } else { 0 }
    } else { 0 };

    for (row_idx, &real_idx) in filtered_ints.iter().skip(scroll_offset).enumerate() {
        if row_idx >= int_inner.height as usize { break; }
        let (name, enabled) = &app.cont3xt.view_editor_integrations[real_idx];
        let marker = if *enabled { "✓" } else { "✗" };
        let is_selected = active == C3ViewEditorField::Integrations
            && app.cont3xt.view_editor_integration_selected == scroll_offset + row_idx;
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
    let vr_selected: Vec<&str> = app.cont3xt.view_editor_view_roles.iter()
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
    let er_selected: Vec<&str> = app.cont3xt.view_editor_edit_roles.iter()
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

    let title = if app.cont3xt.role_popup_for_edit { " Edit Roles " } else { " View Roles " };
    let filter_info = if app.cont3xt.role_popup_filtering {
        format!(" — filter: {}_ ", app.cont3xt.role_popup_filter)
    } else if !app.cont3xt.role_popup_filter.is_empty() {
        format!(" — filter: {} ", app.cont3xt.role_popup_filter)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!("{title}{filter_info}"));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let roles = if app.cont3xt.role_popup_for_edit {
        &app.cont3xt.view_editor_edit_roles
    } else {
        &app.cont3xt.view_editor_view_roles
    };

    let filtered = app.c3_role_popup_filtered_roles();
    let visible = inner.height.saturating_sub(1) as usize; // reserve 1 for help
    let scroll = if app.cont3xt.role_popup_selected >= visible {
        app.cont3xt.role_popup_selected - visible + 1
    } else { 0 };

    for (row_idx, &real_idx) in filtered.iter().skip(scroll).enumerate() {
        if row_idx >= visible { break; }
        let (name, selected) = &roles[real_idx];
        let marker = if *selected { "✓" } else { "✗" };
        let is_highlighted = row_idx + scroll == app.cont3xt.role_popup_selected;
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

    let filter_display = if app.cont3xt.int_settings_filtering {
        format!("Filter: {}_", app.cont3xt.int_settings_filter)
    } else if !app.cont3xt.int_settings_filter.is_empty() {
        format!("Filter: {}", app.cont3xt.int_settings_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_int_settings_filtered();

    let dirty_indicator = if app.cont3xt.int_settings_dirty { " [UNSAVED]" } else { "" };
    let toolbar_text = format!(
        " {} integrations  {}  [d]isable  [/]filter  [r]efresh  Ctrl+S:save{}",
        filtered.len(),
        filter_display,
        dirty_indicator,
    );
    let toolbar = Paragraph::new(toolbar_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    if app.cont3xt.int_settings_filtering {
        let cursor_x = chunks[0].x + 1 + filtered.len().to_string().len() as u16 + 16 + app.cont3xt.int_settings_filter.len() as u16;
        f.set_cursor_position((cursor_x, chunks[0].y));
    }

    let col_names = [" Name", "Status", "Fields"];
    let header_cells: Vec<Cell> = col_names.iter().enumerate().map(|(i, &name)| {
        let is_sorted = (i < 2) && app.cont3xt.int_settings_sort as usize == i;
        let label = sort_header_label(name, is_sorted, app.cont3xt.int_settings_sort_desc);
        Cell::from(label).style(sort_header_style(is_sorted))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let int = &app.cont3xt.int_settings[idx];
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

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.int_settings_table_state);
}

pub(super) fn c3_draw_int_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let idx = app.cont3xt.int_editor_idx;
    let int = match app.cont3xt.int_settings.get(idx) {
        Some(i) => i,
        None => return,
    };

    let field_count = app.cont3xt.int_editor_values.len();
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
    let scroll_offset = if app.cont3xt.int_editor_selected >= max_visible {
        app.cont3xt.int_editor_selected - max_visible + 1
    } else {
        0
    };

    for (i, (field_name, value, is_password, is_boolean, required, _help)) in
        app.cont3xt.int_editor_values.iter().enumerate().skip(scroll_offset).take(max_visible)
    {
        let y = fields_area.y + (i - scroll_offset) as u16;
        if y >= fields_area.y + fields_area.height {
            break;
        }
        let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);
        let is_selected = i == app.cont3xt.int_editor_selected;
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
            let display_value = if *is_password && !app.cont3xt.int_editor_show_password {
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
                let cx = cursor_x + app.cont3xt.int_editor_cursor.min(max_val_width) as u16;
                f.set_cursor_position((cx, y));
            }
        }
    }

    // Help text for selected field
    if let Some((_, _, _, _, _, help)) = app.cont3xt.int_editor_values.get(app.cont3xt.int_editor_selected) {
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

fn c3_draw_settings_link_groups(f: &mut Frame, app: &mut App, area: Rect) {
    match app.cont3xt.lg_level {
        C3LinkGroupLevel::GroupList => c3_draw_lg_group_list(f, app, area),
        C3LinkGroupLevel::GroupEditor => {
            c3_draw_lg_group_list(f, app, area);
            c3_draw_lg_group_editor(f, app, area);
        }
        C3LinkGroupLevel::LinkList => c3_draw_lg_link_list(f, app, area),
        C3LinkGroupLevel::LinkEditor => {
            c3_draw_lg_link_list(f, app, area);
            c3_draw_lg_link_editor(f, app, area);
        }
    }
}

fn c3_draw_lg_group_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.cont3xt.lg_filtering {
        format!("Filter: {}_", app.cont3xt.lg_filter)
    } else if !app.cont3xt.lg_filter.is_empty() {
        format!("Filter: {}", app.cont3xt.lg_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_lg_filtered_groups();

    let toolbar_text = format!(
        " {} link groups  {}  Enter:edit  n:new  d:delete  s/S:sort  /:filter  r:refresh",
        filtered.len(),
        filter_display,
    );
    let toolbar = Paragraph::new(toolbar_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    if app.cont3xt.lg_filtering {
        let cursor_x = chunks[0].x + 1 + filtered.len().to_string().len() as u16 + 16 + app.cont3xt.lg_filter.len() as u16;
        f.set_cursor_position((cursor_x, chunks[0].y));
    }

    let col_names = [" Name", "Creator", "Links", "Editable"];
    let header_cells: Vec<Cell> = col_names.iter().enumerate().map(|(i, &name)| {
        let is_sorted = app.cont3xt.lg_sort_col == i;
        let label = sort_header_label(name, is_sorted, app.cont3xt.lg_sort_desc);
        Cell::from(label).style(sort_header_style(is_sorted))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let g = &app.cont3xt.lg_groups[idx];
        let editable_str = if g.editable { "✓" } else { "✗" };
        let editable_style = if g.editable {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Row::new(vec![
            Cell::from(format!(" {}", g.name)),
            Cell::from(g.creator.clone()),
            Cell::from(format!("{}", g.links.len())),
            Cell::from(editable_str).style(editable_style),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(25),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Link Groups "))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.lg_table_state);
}

fn c3_draw_lg_group_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = center_popup(60, 12, area);
    f.render_widget(Clear, popup_area);

    let idx = app.cont3xt.lg_group_editor_idx;
    let is_editable = app.cont3xt.lg_groups.get(idx).map(|g| g.editable).unwrap_or(false);
    let title = if let Some(group) = app.cont3xt.lg_groups.get(idx) {
        if is_editable {
            format!(" Edit Group: {} ", group.name)
        } else {
            format!(" Group: {} (read-only) ", group.name)
        }
    } else {
        " Edit Group ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    f.render_widget(block, popup_area);

    let inner = Rect::new(popup_area.x + 1, popup_area.y + 1, popup_area.width - 2, popup_area.height - 2);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Name
            Constraint::Length(2),  // ViewRoles
            Constraint::Length(2),  // EditRoles
            Constraint::Min(1),    // Help
        ])
        .split(inner);

    let active = app.cont3xt.lg_group_editor_field;

    // Name field
    let name_style = if active == C3GroupEditorField::Name {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_label = if active == C3GroupEditorField::Name {
        format!("▸ Name: {}", app.cont3xt.lg_group_editor_name)
    } else {
        format!("  Name: {}", app.cont3xt.lg_group_editor_name)
    };
    f.render_widget(Paragraph::new(name_label).style(name_style), sections[0]);
    if active == C3GroupEditorField::Name {
        let cursor_x = sections[0].x + 8 + app.cont3xt.lg_group_editor_cursor as u16;
        f.set_cursor_position((cursor_x, sections[0].y));
    }

    // View Roles
    let vr_style = if active == C3GroupEditorField::ViewRoles {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let vr_display = if app.cont3xt.lg_group_editor_view_roles.is_empty() {
        "(none)".to_string()
    } else {
        app.cont3xt.lg_group_editor_view_roles.join(", ")
    };
    let vr_label = if active == C3GroupEditorField::ViewRoles {
        format!("▸ View Roles: {} [Enter to edit]", vr_display)
    } else {
        format!("  View Roles: {}", vr_display)
    };
    f.render_widget(Paragraph::new(vr_label).style(vr_style), sections[1]);

    // Edit Roles
    let er_style = if active == C3GroupEditorField::EditRoles {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let er_display = if app.cont3xt.lg_group_editor_edit_roles.is_empty() {
        "(none)".to_string()
    } else {
        app.cont3xt.lg_group_editor_edit_roles.join(", ")
    };
    let er_label = if active == C3GroupEditorField::EditRoles {
        format!("▸ Edit Roles: {} [Enter to edit]", er_display)
    } else {
        format!("  Edit Roles: {}", er_display)
    };
    f.render_widget(Paragraph::new(er_label).style(er_style), sections[2]);

    // Help
    f.render_widget(
        Paragraph::new(" ↑/↓:field  Enter:edit roles  Ctrl+S:save  Esc:cancel")
            .style(Style::default().fg(Color::DarkGray)),
        sections[3],
    );

}

pub(super) fn c3_draw_group_role_popup(f: &mut Frame, app: &App, area: Rect, selected_roles: &[String]) {
    let popup_area = center_popup(40, 16, area);
    f.render_widget(Clear, popup_area);

    let filtered = app.c3_all_roles_filtered();
    let title = if app.cont3xt.role_popup_for_edit { " Edit Roles " } else { " View Roles " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Filter bar at top
    let filter_style = if app.cont3xt.role_popup_filtering {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let filter_text = if app.cont3xt.role_popup_filter.is_empty() && !app.cont3xt.role_popup_filtering {
        " /:filter".to_string()
    } else {
        format!(" /{}", app.cont3xt.role_popup_filter)
    };
    f.render_widget(
        Paragraph::new(filter_text).style(filter_style),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
    let visible = list_area.height as usize;
    let offset = if app.cont3xt.role_popup_selected >= visible {
        app.cont3xt.role_popup_selected - visible + 1
    } else { 0 };

    for (row_idx, &real_idx) in filtered.iter().skip(offset).enumerate() {
        if row_idx >= visible { break; }
        if let Some(role) = app.cont3xt.all_roles.get(real_idx) {
            let checked = selected_roles.contains(role);
            let marker = if checked { "[x]" } else { "[ ]" };
            let is_selected = app.cont3xt.role_popup_selected == row_idx + offset;
            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let text = format!(" {} {}", marker, role);
            f.render_widget(
                Paragraph::new(text).style(style),
                Rect::new(list_area.x, list_area.y + row_idx as u16, list_area.width, 1),
            );
        }
    }
}

fn c3_draw_lg_link_list(f: &mut Frame, app: &mut App, area: Rect) {
    let gi = app.cont3xt.lg_editing_group_idx;
    let group = match app.cont3xt.lg_groups.get(gi) {
        Some(g) => g,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.cont3xt.lg_links_filtering {
        format!("  Filter: {}_", app.cont3xt.lg_links_filter)
    } else if !app.cont3xt.lg_links_filter.is_empty() {
        format!("  Filter: {}", app.cont3xt.lg_links_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_lg_filtered_links();
    let toolbar_text = format!(
        " Links in: {}  ({}{} links){}",
        group.name,
        if !app.cont3xt.lg_links_filter.is_empty() { format!("{}/", filtered.len()) } else { String::new() },
        group.links.len(),
        filter_display,
    );
    let toolbar = Paragraph::new(toolbar_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    let col_names = [" Name", "URL", "Types"];
    let header_cells: Vec<Cell> = col_names.iter().map(|&name| {
        Cell::from(name).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let link = &group.links[idx];
        if link.is_separator() {
            Row::new(vec![
                Cell::from(" ──── separator ────").style(Style::default().fg(Color::DarkGray)),
                Cell::from("").style(Style::default().fg(Color::DarkGray)),
                Cell::from("").style(Style::default().fg(Color::DarkGray)),
            ])
        } else {
            let types_str = link.itypes.join(", ");
            let url_display = if link.url.len() > 50 {
                format!("{}…", &link.url[..49])
            } else {
                link.url.clone()
            };
            let name_style = parse_hex_color(&link.color)
                .map(|c| Style::default().fg(c))
                .unwrap_or_default();
            Row::new(vec![
                Cell::from(format!(" {}", link.name)).style(name_style),
                Cell::from(url_display),
                Cell::from(types_str),
            ])
        }
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Percentage(40),
            Constraint::Length(30),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(
        if group.editable {
            format!(" Links: {} ", group.name)
        } else {
            format!(" Links: {} (read-only) ", group.name)
        }
    ))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.lg_links_table_state);
}

fn c3_draw_lg_link_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let all_itypes = ["domain", "ip", "url", "email", "hash", "phone", "text"];

    let popup_width = (area.width * 60 / 100).max(50).min(area.width.saturating_sub(4));
    let popup_height = (area.height * 70 / 100).max(16).min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let lg_editable = app.cont3xt.lg_groups.get(app.cont3xt.lg_editing_group_idx)
        .map(|g| g.editable).unwrap_or(false);
    let title = if app.cont3xt.lg_editor_link.name.is_empty() {
        if lg_editable { " Edit Link ".to_string() } else { " Link (read-only) ".to_string() }
    } else {
        if lg_editable {
            format!(" Edit Link: {} ", app.cont3xt.lg_editor_link.name)
        } else {
            format!(" Link: {} (read-only) ", app.cont3xt.lg_editor_link.name)
        }
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let fields_area = content_chunks[0];
    let footer_area = content_chunks[1];

    let mut y = fields_area.y;
    let fields = C3LinkEditorField::all();

    for &field in fields {
        if y >= fields_area.y + fields_area.height {
            break;
        }
        let is_selected = field == app.cont3xt.lg_editor_field;
        let label_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let label = field.label();

        if field == C3LinkEditorField::Itypes {
            // Label line
            let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);
            f.render_widget(Paragraph::new(format!("  {}:", label)).style(label_style), row_area);
            y += 1;

            // Render each itype as a checkbox
            for (ti, &itype) in all_itypes.iter().enumerate() {
                if y >= fields_area.y + fields_area.height {
                    break;
                }
                let checked = app.cont3xt.lg_editor_link.itypes.iter().any(|t| t == itype);
                let check_char = if checked { "x" } else { " " };
                let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);
                let itype_style = if is_selected && ti == app.cont3xt.lg_editor_itype_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                f.render_widget(
                    Paragraph::new(format!("    [{}] {}", check_char, itype)).style(itype_style),
                    row_area,
                );
                y += 1;
            }
        } else {
            let value = match field {
                C3LinkEditorField::Name => &app.cont3xt.lg_editor_link.name,
                C3LinkEditorField::Url => &app.cont3xt.lg_editor_link.url,
                C3LinkEditorField::Color => &app.cont3xt.lg_editor_link.color,
                C3LinkEditorField::InfoField => &app.cont3xt.lg_editor_link.info,
                C3LinkEditorField::ExternalDocName => &app.cont3xt.lg_editor_link.external_doc_name,
                C3LinkEditorField::ExternalDocUrl => &app.cont3xt.lg_editor_link.external_doc_url,
                _ => "",
            };
            let label_len = label.len() + 4; // "  Label: "
            let max_val_width = fields_area.width as usize - label_len.min(fields_area.width as usize);
            let truncated = if value.len() > max_val_width {
                &value[..max_val_width]
            } else {
                value
            };
            let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);
            if field == C3LinkEditorField::Color {
                let swatch = parse_hex_color(value).map(|c| {
                    Span::styled(" █████", Style::default().fg(c))
                });
                let mut spans = vec![Span::styled(format!("  {}: {}", label, truncated), label_style)];
                if let Some(sw) = swatch {
                    spans.push(sw);
                }
                f.render_widget(Paragraph::new(Line::from(spans)), row_area);
            } else {
                let text = format!("  {}: {}", label, truncated);
                f.render_widget(Paragraph::new(text).style(label_style), row_area);
            }

            if is_selected {
                let cursor_x = row_area.x + label_len as u16 + app.cont3xt.lg_editor_cursor.min(max_val_width) as u16;
                f.set_cursor_position((cursor_x, y));
            }
            y += 1;
        }
    }

    f.render_widget(
        Paragraph::new(" ↑/↓:field  Space:toggle(itypes)  Ctrl+S:apply  Esc:cancel")
            .style(Style::default().fg(Color::DarkGray)),
        footer_area,
    );
}

// ============== Overview Settings ==============

fn c3_draw_settings_overviews(f: &mut Frame, app: &mut App, area: Rect) {
    match app.cont3xt.ov_level {
        C3OverviewLevel::List => c3_draw_ov_list(f, app, area),
        C3OverviewLevel::Editor => {
            c3_draw_ov_list(f, app, area);
            c3_draw_ov_editor(f, app, area);
        }
        C3OverviewLevel::FieldList => c3_draw_ov_field_list(f, app, area),
        C3OverviewLevel::FieldEditor => {
            c3_draw_ov_field_list(f, app, area);
            c3_draw_ov_field_editor(f, app, area);
        }
    }
}

fn c3_draw_ov_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.cont3xt.ov_filtering {
        format!("  Filter: {}_", app.cont3xt.ov_filter)
    } else if !app.cont3xt.ov_filter.is_empty() {
        format!("  Filter: {}", app.cont3xt.ov_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_ov_filtered_list();
    let toolbar_text = format!(
        " Overviews  ({}{} items){}",
        if !app.cont3xt.ov_filter.is_empty() { format!("{}/", filtered.len()) } else { String::new() },
        app.cont3xt.ov_list.len(),
        filter_display,
    );
    let toolbar = Paragraph::new(toolbar_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    let sort_cols = ["Name", "IType", "Default", "Creator", "Fields"];
    let header_cells: Vec<Cell> = sort_cols.iter().enumerate().map(|(i, &name)| {
        let label = sort_header_label(name, i == app.cont3xt.ov_sort_col, app.cont3xt.ov_sort_desc);
        Cell::from(format!(" {label}")).style(sort_header_style(i == app.cont3xt.ov_sort_col))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&idx| {
        let ov = &app.cont3xt.ov_list[idx];
        let default_str = if ov.is_default { "★" } else { "" };
        let shared = if !ov.editable { " 🔗" } else { "" };
        Row::new(vec![
            Cell::from(format!(" {}{}", ov.name, shared)),
            Cell::from(ov.itype.clone()),
            Cell::from(default_str.to_string()),
            Cell::from(ov.creator.clone()),
            Cell::from(format!("{}", ov.fields.len())),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Overviews "))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.ov_table_state);
}

fn c3_draw_ov_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let idx = app.cont3xt.ov_editor_idx;
    let ov = match app.cont3xt.ov_list.get(idx) {
        Some(o) => o,
        None => return,
    };

    let popup_width = (area.width * 60 / 100).max(50).min(area.width.saturating_sub(4));
    let popup_height = 12u16.min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let title = if ov.editable {
        format!(" Edit Overview: {} ", ov.name)
    } else {
        format!(" Overview: {} (read-only) ", ov.name)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let fields_area = content_chunks[0];
    let footer_area = content_chunks[1];

    let mut y = fields_area.y;
    for &field in C3OverviewEditorField::all() {
        if y >= fields_area.y + fields_area.height { break; }
        let is_selected = field == app.cont3xt.ov_editor_field;
        let label_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let label = field.label();
        let value = match field {
            C3OverviewEditorField::Name => app.cont3xt.ov_editor_name.clone(),
            C3OverviewEditorField::Title => app.cont3xt.ov_editor_title.clone(),
            C3OverviewEditorField::Itype => app.cont3xt.ov_editor_itype.clone(),
            C3OverviewEditorField::ViewRoles => app.cont3xt.ov_editor_view_roles.join(", "),
            C3OverviewEditorField::EditRoles => app.cont3xt.ov_editor_edit_roles.join(", "),
        };

        let label_len = label.len() + 4;
        let max_val_width = fields_area.width as usize - label_len.min(fields_area.width as usize);
        let truncated = if value.len() > max_val_width {
            &value[..max_val_width]
        } else {
            &value
        };
        let row_area = Rect::new(fields_area.x, y, fields_area.width, 1);

        let is_role_field = field == C3OverviewEditorField::ViewRoles || field == C3OverviewEditorField::EditRoles;
        if is_role_field {
            let role_hint = if is_selected { " (Enter to edit)" } else { "" };
            let text = format!("  {}: {}{}", label, truncated, role_hint);
            f.render_widget(Paragraph::new(text).style(label_style), row_area);
        } else {
            let text = format!("  {}: {}", label, truncated);
            f.render_widget(Paragraph::new(text).style(label_style), row_area);
            if is_selected {
                let cursor_x = row_area.x + label_len as u16 + app.cont3xt.ov_editor_cursor.min(max_val_width) as u16;
                f.set_cursor_position((cursor_x, y));
            }
        }
        y += 1;
    }

    f.render_widget(
        Paragraph::new(" ↑/↓:field  Enter:edit roles/fields  Ctrl+S:save  Esc:cancel")
            .style(Style::default().fg(Color::DarkGray)),
        footer_area,
    );
}

fn c3_draw_ov_field_list(f: &mut Frame, app: &mut App, area: Rect) {
    let idx = app.cont3xt.ov_editor_idx;
    let ov = match app.cont3xt.ov_list.get(idx) {
        Some(o) => o,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.cont3xt.ov_fields_filtering {
        format!("  Filter: {}_", app.cont3xt.ov_fields_filter)
    } else if !app.cont3xt.ov_fields_filter.is_empty() {
        format!("  Filter: {}", app.cont3xt.ov_fields_filter)
    } else {
        String::new()
    };

    let filtered = app.c3_ov_filtered_fields();
    let toolbar_text = format!(
        " Fields in: {}  ({}{} fields){}",
        ov.name,
        if !app.cont3xt.ov_fields_filter.is_empty() { format!("{}/", filtered.len()) } else { String::new() },
        ov.fields.len(),
        filter_display,
    );
    let toolbar = Paragraph::new(toolbar_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(toolbar, chunks[0]);

    let col_names = [" Integration", "Field", "Label", "Type"];
    let header_cells: Vec<Cell> = col_names.iter().map(|&name| {
        Cell::from(name).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    }).collect();
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = filtered.iter().map(|&fi| {
        let field = &ov.fields[fi];
        if field.field_type == "custom" {
            let custom_label = field.custom.as_ref()
                .and_then(|v| v.get("custom"))
                .and_then(|v| v.get("label"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Row::new(vec![
                Cell::from(format!(" {}", field.from)),
                Cell::from(custom_label.to_string()).style(Style::default().fg(Color::Yellow)),
                Cell::from(""),
                Cell::from("custom").style(Style::default().fg(Color::Yellow)),
            ])
        } else {
            let alias_str = field.alias.as_deref().unwrap_or("");
            Row::new(vec![
                Cell::from(format!(" {}", field.from)),
                Cell::from(field.field.clone()),
                Cell::from(alias_str.to_string()),
                Cell::from("linked"),
            ])
        }
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(
        if ov.editable {
            format!(" Fields: {} ", ov.name)
        } else {
            format!(" Fields: {} (read-only) ", ov.name)
        }
    ))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.ov_fields_table_state);
}

fn c3_draw_ov_field_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let is_custom = app.cont3xt.ov_field_editor_is_custom;
    let popup_width = (area.width * 60 / 100).max(50).min(area.width.saturating_sub(4));
    let popup_height = if is_custom {
        (app.cont3xt.ov_fe_json_lines.len() as u16 + 6).max(10).min(area.height.saturating_sub(4))
    } else {
        7u16.min(area.height.saturating_sub(4))
    };
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let ov_editable = app.cont3xt.ov_list.get(app.cont3xt.ov_editor_idx)
        .map(|ov| ov.editable).unwrap_or(false);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(if ov_editable { " Edit Field " } else { " Field (read-only) " });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let fields_area = content_chunks[0];
    let footer_area = content_chunks[1];

    let mut y = fields_area.y;

    // Integration (From) row - always shown
    let from_selected = app.cont3xt.ov_field_editor_field == C3OvFieldEditorField::From;
    let from_style = if from_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let from_val = if app.cont3xt.ov_field_editor_from.is_empty() { "<Enter to select>" } else { &app.cont3xt.ov_field_editor_from };
    f.render_widget(
        Paragraph::new(format!("  Integration: {} ▸", from_val)).style(from_style),
        Rect::new(fields_area.x, y, fields_area.width, 1),
    );
    y += 1;

    if is_custom {
        // Custom JSON multiline editor
        let json_selected = app.cont3xt.ov_field_editor_field == C3OvFieldEditorField::CustomJson;
        let label_style = if json_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new("  Custom JSON:").style(label_style),
            Rect::new(fields_area.x, y, fields_area.width, 1),
        );
        y += 1;

        let json_area_height = (fields_area.y + fields_area.height).saturating_sub(y) as usize;
        // Scroll to keep cursor visible
        if app.cont3xt.ov_fe_json_line < app.cont3xt.ov_fe_json_scroll {
            app.cont3xt.ov_fe_json_scroll = app.cont3xt.ov_fe_json_line;
        } else if app.cont3xt.ov_fe_json_line >= app.cont3xt.ov_fe_json_scroll + json_area_height {
            app.cont3xt.ov_fe_json_scroll = app.cont3xt.ov_fe_json_line + 1 - json_area_height;
        }

        for (vi, li) in (app.cont3xt.ov_fe_json_scroll..).enumerate() {
            if vi >= json_area_height { break; }
            if let Some(line) = app.cont3xt.ov_fe_json_lines.get(li) {
                let row_y = y + vi as u16;
                let max_w = fields_area.width.saturating_sub(4) as usize;
                let display = if line.len() > max_w { &line[..max_w] } else { line.as_str() };
                let style = if json_selected && li == app.cont3xt.ov_fe_json_line {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                f.render_widget(
                    Paragraph::new(format!("    {}", display)).style(style),
                    Rect::new(fields_area.x, row_y, fields_area.width, 1),
                );
                if json_selected && li == app.cont3xt.ov_fe_json_line {
                    let col = app.cont3xt.ov_fe_json_col.min(max_w);
                    f.set_cursor_position((fields_area.x + 4 + col as u16, row_y));
                }
            }
        }
    } else {
        // Linked mode: Field + Label rows
        let field_selected = app.cont3xt.ov_field_editor_field == C3OvFieldEditorField::Field;
        let field_style = if field_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let field_val = if app.cont3xt.ov_field_editor_field_name.is_empty() { "<Enter to select>" } else { &app.cont3xt.ov_field_editor_field_name };
        f.render_widget(
            Paragraph::new(format!("  Field: {} ▸", field_val)).style(field_style),
            Rect::new(fields_area.x, y, fields_area.width, 1),
        );
        y += 1;

        let label_selected = app.cont3xt.ov_field_editor_field == C3OvFieldEditorField::Label;
        let label_style = if label_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let label_prefix = "  Label: ";
        f.render_widget(
            Paragraph::new(format!("{}{}", label_prefix, app.cont3xt.ov_field_editor_label)).style(label_style),
            Rect::new(fields_area.x, y, fields_area.width, 1),
        );
        if label_selected {
            let max_w = fields_area.width as usize - label_prefix.len();
            let col = app.cont3xt.ov_field_editor_cursor.min(max_w);
            f.set_cursor_position((fields_area.x + label_prefix.len() as u16 + col as u16, y));
        }
    }

    f.render_widget(
        Paragraph::new(" ↑/↓:nav  Enter:select  Ctrl+S:save  Esc:cancel")
            .style(Style::default().fg(Color::DarkGray)),
        footer_area,
    );
}

pub(super) fn c3_draw_ov_fe_selector_popup(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.cont3xt.ov_fe_popup_for_field { " Select Field " } else { " Select Integration " };
    let popup_area = center_popup(45, 18, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Filter bar
    let filter_style = if app.cont3xt.ov_fe_popup_filtering {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let filter_text = if app.cont3xt.ov_fe_popup_filter.is_empty() && !app.cont3xt.ov_fe_popup_filtering {
        " /:filter".to_string()
    } else {
        format!(" /{}", app.cont3xt.ov_fe_popup_filter)
    };
    f.render_widget(
        Paragraph::new(filter_text).style(filter_style),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
    let filtered = app.c3_ov_fe_popup_filtered();
    let visible = list_area.height as usize;
    let offset = if app.cont3xt.ov_fe_popup_selected >= visible {
        app.cont3xt.ov_fe_popup_selected - visible + 1
    } else { 0 };

    for (row_idx, &real_idx) in filtered.iter().skip(offset).enumerate() {
        if row_idx >= visible { break; }
        if let Some(name) = app.cont3xt.ov_fe_popup_items.get(real_idx) {
            let is_selected = app.cont3xt.ov_fe_popup_selected == row_idx + offset;
            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if name == "Custom" {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let text = format!("  {}", name);
            f.render_widget(
                Paragraph::new(text).style(style),
                Rect::new(list_area.x, list_area.y + row_idx as u16, list_area.width, 1),
            );
        }
    }
}
