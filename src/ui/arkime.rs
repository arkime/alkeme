use super::*;

pub(super) fn draw_arkime(f: &mut Frame, app: &mut App, area: Rect) {
    let arkime_title = if let Some(ref v) = app.vr_active_view_name {
        format!(" Arkime Summary [view: {}] ", v)
    } else {
        " Arkime Summary ".to_string()
    };
    if app.vr_summary_field.is_empty() {
        // Show prompt to select a field
        let block = Block::default()
            .borders(Borders::ALL)
            .title(arkime_title);
        let text = Paragraph::new(Line::from(vec![
            Span::raw("Press "),
            Span::styled("f", Style::default().fg(Color::Yellow)),
            Span::raw(" to select a field"),
        ]))
        .alignment(Alignment::Center)
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // bar chart
            Constraint::Min(0),    // table
        ])
        .split(area);

    draw_summary_bar_chart(f, app, chunks[0]);
    draw_summary_table(f, app, chunks[1]);
}

fn draw_summary_bar_chart(f: &mut Frame, app: &App, area: Rect) {
    let metric = app.vr_summary_metric;
    let data: Vec<(&str, u64)> = app.vr_summary_data.iter()
        .map(|item| {
            let label = item.item.as_str().unwrap_or("");
            let val = match metric {
                SummaryMetric::Sessions => item.sessions,
                SummaryMetric::Packets => item.packets,
                SummaryMetric::Bytes => item.bytes,
            };
            (label, val)
        })
        .collect();

    if data.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} - {} (no data) [G]raph type ", app.vr_summary_field, metric.label()));
        f.render_widget(block, area);
        return;
    }

    let bar_width = if data.is_empty() { 1 } else {
        let w = (area.width.saturating_sub(2)) / data.len() as u16;
        w.clamp(1, 12)
    };

    let bars: Vec<Bar> = data.iter()
        .map(|(label, val)| {
            let truncated: String = if label.len() > bar_width as usize {
                label.chars().take(bar_width as usize).collect()
            } else {
                label.to_string()
            };
            Bar::default()
                .value(*val)
                .label(Line::from(truncated))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let chart = BarChart::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} - {} [G]raph type ", app.vr_summary_field, metric.label())))
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    f.render_widget(chart, area);
}

fn draw_summary_table(f: &mut Frame, app: &mut App, area: Rect) {
    let sort_lbl = |sort: SummarySort, label: &str| -> String {
        sort_header_label(label, app.vr_summary_sort == sort, app.vr_summary_sort_desc)
    };
    let sort_sty = |sort: SummarySort| -> Style {
        sort_header_style(app.vr_summary_sort == sort)
    };
    let header = Row::new(vec![
        Cell::from(sort_lbl(SummarySort::Value, "Value")).style(sort_sty(SummarySort::Value)),
        Cell::from(sort_lbl(SummarySort::Sessions, "Sessions")).style(sort_sty(SummarySort::Sessions)),
        Cell::from(sort_lbl(SummarySort::Packets, "Packets")).style(sort_sty(SummarySort::Packets)),
        Cell::from(sort_lbl(SummarySort::Bytes, "Bytes")).style(sort_sty(SummarySort::Bytes)),
    ])
    .height(1)
    .bottom_margin(0);

    let rows: Vec<Row> = app.vr_summary_data.iter().map(|item| {
        let label = match &item.item {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Row::new(vec![
            Cell::from(label),
            Cell::from(format_number(item.sessions)).style(Style::default().fg(Color::White)),
            Cell::from(format_number(item.packets)).style(Style::default().fg(Color::White)),
            Cell::from(format_human_bytes(item.bytes as f64)).style(Style::default().fg(Color::White)),
        ])
    }).collect();

    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(" {} [f]ield [s]ort [S]ort dir ", app.vr_summary_field)))
    .row_highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.vr_summary_table_state);
}

pub(super) fn draw_field_selector(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // filter input
            Constraint::Min(0),   // field list
        ])
        .split(popup_area);

    // Filter input
    let filter_style = Style::default().fg(Color::Yellow);
    let filter_display = if app.vr_field_filter.is_empty() {
        "Type to filter fields...".to_string()
    } else {
        app.vr_field_filter.clone()
    };
    let filter_input = Paragraph::new(Span::styled(&filter_display,
        if app.vr_field_filter.is_empty() { Style::default().fg(Color::DarkGray) } else { filter_style }))
        .block(Block::default().borders(Borders::ALL).title(" Select Field "));
    f.render_widget(filter_input, chunks[0]);

    // Field list
    let filtered = app.vr_filtered_fields();
    let items: Vec<ListItem> = filtered.iter().enumerate().map(|(i, field)| {
        let style = if i == app.vr_field_filter_selected {
            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
        } else {
            Style::default()
        };
        let line = if field.friendly_name.is_empty() {
            field.exp.clone()
        } else {
            format!("{} ({})", field.exp, field.friendly_name)
        };
        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(list, chunks[1]);
}

pub(super) fn draw_under_construction(f: &mut Frame, app: &App, area: Rect) {
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    if inner.width < 40 || inner.height < 12 {
        return;
    }

    // Blinking color based on tick
    let tick = (app.anim_start.elapsed().as_millis() / 300) as usize;
    let colors = [Color::Yellow, Color::Red, Color::Magenta, Color::Cyan, Color::Green];

    // Animated construction barricade
    let shift = tick % 6;
    let barricade: String = (0..inner.width as usize)
        .map(|i| if (i + shift) % 6 < 3 { '▓' } else { '░' })
        .collect();

    let cy = inner.y + 1;
    let buf = f.buffer_mut();

    // Top barricade
    for (i, ch) in barricade.chars().enumerate() {
        let x = inner.x + i as u16;
        if x < inner.x + inner.width {
            buf[(x, cy)].set_char(ch).set_style(Style::default().fg(Color::Yellow).bg(Color::Black));
        }
    }

    let banner = [
        " ██╗   ██╗███╗   ██╗██████╗ ███████╗██████╗  ",
        " ██║   ██║████╗  ██║██╔══██╗██╔════╝██╔══██╗ ",
        " ██║   ██║██╔██╗ ██║██║  ██║█████╗  ██████╔╝ ",
        " ██║   ██║██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗ ",
        " ╚██████╔╝██║ ╚████║██████╔╝███████╗██║  ██║ ",
        "  ╚═════╝ ╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝ ",
    ];

    let construction = "★ ☆ CONSTRUCTION ☆ ★";
    let visitor_line = "You are visitor #000,001";
    let best_viewed = "Best viewed in alkeme TUI";

    // Draw banner
    let banner_y = cy + 2;
    for (row, line) in banner.iter().enumerate() {
        let y = banner_y + row as u16;
        if y >= inner.y + inner.height { break; }
        let bx = inner.x + (inner.width.saturating_sub(line.chars().count() as u16)) / 2;
        let color = colors[(row + tick) % colors.len()];
        for (col, ch) in line.chars().enumerate() {
            let x = bx + col as u16;
            if x < inner.x + inner.width && ch != ' ' {
                buf[(x, y)].set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }

    // "CONSTRUCTION" line
    let con_y = banner_y + banner.len() as u16 + 1;
    if con_y < inner.y + inner.height {
        let con_x = inner.x + (inner.width.saturating_sub(construction.len() as u16)) / 2;
        let blink_color = colors[tick % colors.len()];
        for (col, ch) in construction.chars().enumerate() {
            let x = con_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, con_y)].set_char(ch).set_style(
                    Style::default().fg(blink_color).add_modifier(Modifier::BOLD)
                );
            }
        }
    }

    // Visitor counter
    let vis_y = con_y + 2;
    if vis_y < inner.y + inner.height {
        let vis_x = inner.x + (inner.width.saturating_sub(visitor_line.len() as u16)) / 2;
        for (col, ch) in visitor_line.chars().enumerate() {
            let x = vis_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, vis_y)].set_char(ch).set_style(Style::default().fg(Color::Green));
            }
        }
    }

    // Best viewed line
    let bv_y = vis_y + 1;
    if bv_y < inner.y + inner.height {
        let bv_x = inner.x + (inner.width.saturating_sub(best_viewed.len() as u16)) / 2;
        for (col, ch) in best_viewed.chars().enumerate() {
            let x = bv_x + col as u16;
            if x < inner.x + inner.width {
                buf[(x, bv_y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }

    // Bottom barricade
    let bot_y = bv_y + 2;
    if bot_y < inner.y + inner.height {
        for (i, ch) in barricade.chars().enumerate() {
            let x = inner.x + i as u16;
            if x < inner.x + inner.width {
                buf[(x, bot_y)].set_char(ch).set_style(Style::default().fg(Color::Yellow).bg(Color::Black));
            }
        }
    }
}

pub(super) fn draw_owl(f: &mut Frame, app: &mut App, area: Rect) {
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    if inner.width < 12 || inner.height < 6 {
        return;
    }

    // Owl walking frames (facing right and left)
    let owl_right: [&[&str]; 2] = [
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  /)  )  ",
            " / \" \"   ",
            " _|  |_  ",
        ],
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  /)  )  ",
            "   \" \"   ",
            "  _| |_  ",
        ],
    ];
    let owl_left: [&[&str]; 2] = [
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  (  (\\  ",
            "   \" \" \\ ",
            "  _|  |_ ",
        ],
        &[
            "  ,___,  ",
            "  (O,O)  ",
            "  (  (\\  ",
            "   \" \"   ",
            "  _| |_  ",
        ],
    ];

    let owl_w = 10u16;
    let owl_h = 5u16;

    // Update position every 150ms
    if app.owl_tick.elapsed() >= std::time::Duration::from_millis(75) {
        app.owl_tick = std::time::Instant::now();
        app.owl_frame = (app.owl_frame + 1) % 2;

        app.owl_x += app.owl_dx;
        app.owl_y += app.owl_dy;

        let max_x = (inner.width.saturating_sub(owl_w)) as f32;
        let max_y = (inner.height.saturating_sub(owl_h)) as f32;

        if app.owl_x <= 0.0 {
            app.owl_x = 0.0;
            app.owl_dx = app.owl_dx.abs();
        } else if app.owl_x >= max_x {
            app.owl_x = max_x;
            app.owl_dx = -app.owl_dx.abs();
        }

        if app.owl_y <= 0.0 {
            app.owl_y = 0.0;
            app.owl_dy = app.owl_dy.abs();
        } else if app.owl_y >= max_y {
            app.owl_y = max_y;
            app.owl_dy = -app.owl_dy.abs();
        }
    }

    let frames = if app.owl_dx > 0.0 { &owl_right } else { &owl_left };
    let owl = frames[app.owl_frame % 2];

    let ox = inner.x + app.owl_x as u16;
    let oy = inner.y + app.owl_y as u16;
    let buf = f.buffer_mut();

    for (row, line) in owl.iter().enumerate() {
        let y = oy + row as u16;
        if y >= inner.y + inner.height { break; }
        for (col, ch) in line.chars().enumerate() {
            let x = ox + col as u16;
            if x >= inner.x + inner.width { break; }
            if ch != ' ' {
                buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::Yellow));
            }
        }
    }
}
