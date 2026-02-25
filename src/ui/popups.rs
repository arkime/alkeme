use super::*;

pub(super) fn draw_action_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu = match &app.action_menu {
        Some(m) => m,
        None => return,
    };

    if menu.scope.is_some() {
        // Scope selection sub-menu
        let kind_label = menu.pending_kind.map(|k| k.label()).unwrap_or("");
        let title = format!(" {} ", kind_label);
        let scope_options = ["Visible", "Matching"];

        let popup_width = 30u16;
        let popup_height = scope_options.len() as u16 + 2;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let lines: Vec<Line> = scope_options.iter().enumerate().map(|(i, label)| {
            let is_selected = i == menu.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "▸ " } else { "  " };
            Line::from(Span::styled(format!("{prefix}{label}"), style))
        }).collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(title.as_str()),
            );
        f.render_widget(paragraph, popup_area);
        return;
    }

    let options = menu.options(app.vr_remove_enabled());
    let title = match menu.target {
        crate::app::ActionTarget::Single => " Session Action ",
        crate::app::ActionTarget::All => " All Sessions Action ",
    };

    let popup_width = 30u16;
    let popup_height = options.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines: Vec<Line> = options.iter().enumerate().map(|(i, kind)| {
        let is_selected = i == menu.selected;
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▸ " } else { "  " };
        Line::from(Span::styled(format!("{prefix}{}", kind.label()), style))
    }).collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title),
        );
    f.render_widget(paragraph, popup_area);
}

pub(super) fn draw_action_prompt(f: &mut Frame, app: &App, area: Rect) {
    let prompt = match &app.action_prompt {
        Some(p) => p,
        None => return,
    };

    let label = prompt.kind.prompt_label();
    let popup_width = 50u16;
    let popup_height = 3u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let line = Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Yellow)),
        Span::styled(&prompt.input, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Gray)),
    ]);

    let title = format!(" {} ", prompt.kind.label());
    let paragraph = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );
    f.render_widget(paragraph, popup_area);
}

pub(super) fn draw_debug(f: &mut Frame, app: &App, area: Rect) {
    let entries = app.http_log.lock().unwrap();
    let total = entries.len();

    let mut lines: Vec<Line> = Vec::new();

    // Header line
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<23}", "Timestamp"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:<6}", "Method"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>4}", "Code"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>6}", "First"), Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {:>6}", "Last"), Style::default().fg(Color::Yellow)),
        Span::styled("  URL", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    // Newest first
    let visible_height = area.height.saturating_sub(6) as usize; // borders + header + hints
    let scroll = app.debug_scroll.min(total.saturating_sub(1));
    let start = total.saturating_sub(scroll + visible_height);
    let end = total.saturating_sub(scroll);

    for entry in entries[start..end].iter().rev() {
        let ts = entry.timestamp.format("%Y/%m/%d %H:%M:%S%.3f").to_string();
        let status_color = if entry.status >= 400 { Color::Red }
            else if entry.status >= 300 { Color::Yellow }
            else { Color::Green };

        lines.push(Line::from(vec![
            Span::raw(format!(" {:<23}", ts)),
            Span::styled(format!(" {:<6}", entry.method), Style::default().fg(Color::Cyan)),
            Span::styled(format!(" {:>4}", entry.status), Style::default().fg(status_color)),
            Span::raw(format!(" {:>5}ms", entry.first_byte_ms)),
            Span::raw(format!(" {:>5}ms", entry.last_byte_ms)),
            Span::raw(format!("  {}", entry.url)),
        ]));

        if let Some(ref data) = entry.post_data {
            let truncated = if data.len() > 120 { &data[..120] } else { data.as_str() };
            lines.push(Line::from(vec![
                Span::raw("                          "),
                Span::styled(format!("↳ {}", truncated), Style::default().fg(Color::DarkGray)),
            ]));
        }

        if let Some(ref resp) = entry.response_body {
            let truncated = if resp.len() > 120 { &resp[..120] } else { resp.as_str() };
            lines.push(Line::from(vec![
                Span::raw("                          "),
                Span::styled(format!("← {}", truncated), Style::default().fg(Color::Red)),
            ]));
        }
    }

    drop(entries);

    let popup_width = area.width.saturating_sub(4).min(140);
    let popup_height = area.height.saturating_sub(4);
    let popup_area = Rect::new(
        area.x + (area.width.saturating_sub(popup_width)) / 2,
        area.y + (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, popup_area);
    let title = format!(" HTTP Debug Log ({} requests) ", total);
    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(" Esc:close  ↑↓:scroll  Home:top ").centered())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

pub(super) fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let key = |k: &str| Span::styled(format!("  {k:19}"), Style::default().fg(Color::Yellow));
    let blank = || Line::from("");

    macro_rules! hdr {
        ($s:expr) => { Line::from(Span::styled($s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))) };
    }

    let (title, help_text) = if app.vr_packets_view.is_some() {
        ("Packets", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Scroll one page")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Scroll one page")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Options"),
            blank(),
            Line::from(vec![key("r"), Span::raw("Toggle raw packets")]),
            Line::from(vec![key("l"), Span::raw("Cycle line numbers: hex/dec/off")]),
            blank(),
            Line::from(vec![key("Esc / p / q"), Span::raw("Close packets view")]),
            blank(),
            hdr!("Colors"),
            blank(),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Cyan)), Span::raw("Source packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Green)), Span::raw("Destination packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::DarkGray)), Span::raw("Hex offset")]),
        ])
    } else if app.vr_session_view == SessionView::Detail && app.active_tab == Tab::Sessions {
        ("Session Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Add field to expression")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("E"), Span::raw("Edit expression")]),
            Line::from(vec![key("a"), Span::raw("Session actions")]),
            Line::from(vec![key("A"), Span::raw("All sessions actions")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
        ])
    } else if app.active_tab == Tab::Stats && app.vr_stats_view == StatsView::Detail {
        ("Stats Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("E"), Span::raw("Edit expression")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
        ])
    } else if app.active_tab == Tab::Stats {
        ("Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("1 / 2 / 3"), Span::raw("Switch sub-tab")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open detail")]),
            Line::from(vec![key("/ / E"), Span::raw("Filter / edit expression")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("Esc"), Span::raw("Close overlay")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.active_tab == Tab::Arkime {
        ("Arkime Summary", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Add to expression")]),
            Line::from(vec![key("/ / E"), Span::raw("Edit expression")]),
            Line::from(vec![key("f"), Span::raw("Select field")]),
            Line::from(vec![key("G"), Span::raw("Cycle graph metric")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("t / T"), Span::raw("Cycle time range")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("v"), Span::raw("Views")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.vr_show_column_editor {
        ("Column Editor", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Space / Enter"), Span::raw("Toggle field on/off")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("m"), Span::raw("Reorder mode (↑/↓ to move)")]),
            Line::from(vec![key("a"), Span::raw("Apply changes")]),
            Line::from(vec![key("d"), Span::raw("Reset to defaults")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
        ])
    } else if app.vr_show_layout_popup {
        ("Layouts", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate layouts")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select / save / load layout")]),
            Line::from(vec![key("/"), Span::raw("Filter layouts")]),
            Line::from(vec![key("x / Delete"), Span::raw("Delete selected layout")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
        ])
    } else if app.vr_show_view_popup {
        ("Views", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate views")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select view / save new view")]),
            Line::from(vec![key("/"), Span::raw("Filter views")]),
            Line::from(vec![key("x"), Span::raw("Delete selected view")]),
            Line::from(vec![key("Tab"), Span::raw("Toggle save columns (in save dialog)")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
        ])
    } else if app.app_mode == AppMode::Cont3xt && app.active_tab == Tab::C3Stats {
        ("Cont3xt Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("1 / 2"), Span::raw("Switch sub-tab (Integrations / iTypes)")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Filter by name")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh stats")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.c3_show_link_popup {
        ("Link Groups", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate links")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down (10)")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open link in browser")]),
            Line::from(vec![key("/"), Span::raw("Filter links by name")]),
            Line::from(vec![key("Esc / q / l"), Span::raw("Close popup")]),
        ])
    } else if app.app_mode == AppMode::Cont3xt {
        ("Cont3xt Search", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate results / scroll")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down (detail)")]),
            Line::from(vec![key("← / →"), Span::raw("Scroll detail left / right")]),
            Line::from(vec![key("Home"), Span::raw("Jump to top, reset scroll")]),
            Line::from(vec![key("End"), Span::raw("Jump to bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Edit search indicator")]),
            Line::from(vec![key("Tab (in results)"), Span::raw("Toggle results / detail focus")]),
            Line::from(vec![key("R"), Span::raw("Toggle raw JSON / card view")]),
            Line::from(vec![key("i"), Span::raw("Integrations popup (v:views inside)")]),
            Line::from(vec![key("l"), Span::raw("Link groups popup")]),
            Line::from(vec![key("r"), Span::raw("Re-run search")]),
            Line::from(vec![key("Ctrl+r"), Span::raw("Re-run search (no cache)")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
        ].into_iter().chain(
            if app.pl_saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ]).collect())
    } else if app.app_mode == AppMode::Parliament && app.active_tab == Tab::Dashboard {
        ("Parliament Dashboard", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate clusters")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open cluster in Viewer mode")]),
            Line::from(vec![key("i"), Span::raw("Cluster detail overlay")]),
            Line::from(vec![key("c"), Span::raw("Open Cont3xt (if configured)")]),
            Line::from(vec![key("w"), Span::raw("Open WISE (if configured)")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.app_mode == AppMode::Parliament && app.active_tab == Tab::Issues {
        ("Parliament Issues", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate issues")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/ / E"), Span::raw("Filter issues")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh issues")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.app_mode == AppMode::Wise && app.active_tab == Tab::WsStats {
        ("WISE Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("1 / 2"), Span::raw("Sources / Types sub-tab")]),
            Line::from(vec![key("/ / E"), Span::raw("Filter stats")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else if app.app_mode == AppMode::Wise && app.active_tab == Tab::WsQuery {
        ("WISE Query", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate results")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("s"), Span::raw("Cycle source")]),
            Line::from(vec![key("t"), Span::raw("Cycle type")]),
            Line::from(vec![key("/ / E"), Span::raw("Edit query value")]),
            Line::from(vec![key("Enter"), Span::raw("Run query")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
        ])
    } else {
        let mut ht = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate sessions")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
            Line::from(vec![key("Shift+← / Shift+→"), Span::raw("First / last page")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open session detail")]),
            Line::from(vec![key("p"), Span::raw("View packets")]),
            Line::from(vec![key("/ / E"), Span::raw("Edit expression")]),
            Line::from(vec![key("t / T"), Span::raw("Cycle time range")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("g"), Span::raw("Toggle graph")]),
            Line::from(vec![key("G"), Span::raw("Cycle graph type")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("a"), Span::raw("Session actions")]),
            Line::from(vec![key("A"), Span::raw("All sessions actions")]),
            Line::from(vec![key("c"), Span::raw("Columns & layouts")]),
            Line::from(vec![key("v"), Span::raw("Views")]),
        ];
        if app.pl_saved_client.is_some() {
            ht.push(Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")]));
        }
        ht.push(Line::from(vec![key("q"), Span::raw("Quit")]));
        ("Sessions", ht)
    };

    let mut lines = help_text;
    lines.push(blank());
    lines.push(Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))));

    let popup_width = 52;
    let popup_height = lines.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {title} Help ")),
        );
    f.render_widget(help, popup_area);
}

pub(super) fn draw_packets(f: &mut Frame, app: &mut App, area: Rect) {
    let pkt_data = match &app.vr_packets_view {
        Some(p) => p,
        None => return,
    };

    let popup_width = (area.width as f32 * 0.9) as u16;
    let popup_height = (area.height as f32 * 0.9) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let half_width = popup_area.width.saturating_sub(2) / 2;

    // Build rows: each row is (left_spans, right_spans)
    let mut rows: Vec<(Vec<Span>, Vec<Span>)> = Vec::new();

    // Column headers
    rows.push((
        vec![Span::styled(
            format!(" {}", pkt_data.src_label),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )],
        vec![Span::styled(
            format!(" {}", pkt_data.dst_label),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )],
    ));

    for pkt in &pkt_data.packets {
        let dir_color = if pkt.src { Color::Cyan } else { Color::Green };
        let mut header_parts = Vec::new();
        if let Some(ts) = pkt.timestamp {
            header_parts.push(Span::styled(
                format!("{} ", format_epoch_ms(ts)),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let mut info = format!("{} bytes", pkt.bytes);
        if !pkt.flags.is_empty() {
            info = format!("{} {}", pkt.flags, info);
        }
        header_parts.push(Span::styled(
            format!("── {} ──", info),
            Style::default().fg(dir_color).add_modifier(Modifier::BOLD),
        ));
        let mut pkt_rows = vec![header_parts];
        for (i, hex_line) in pkt.lines.iter().enumerate() {
            let offset = i * 16;
            let mut spans = Vec::new();
            match app.vr_packets_line {
                crate::app::LineMode::Hex => {
                    spans.push(Span::styled(
                        format!("{:04x}: ", offset),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                crate::app::LineMode::Decimal => {
                    spans.push(Span::styled(
                        format!("{:5}: ", offset),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                crate::app::LineMode::Off => {}
            }
            spans.push(Span::styled(
                hex_line.to_string(),
                Style::default().fg(dir_color),
            ));
            pkt_rows.push(spans);
        }
        for row in pkt_rows {
            if pkt.src {
                rows.push((row, Vec::new()));
            } else {
                rows.push((Vec::new(), row));
            }
        }
    }

    let visible = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = rows.len().saturating_sub(visible) as u16;
    app.vr_packets_scroll = app.vr_packets_scroll.min(max_scroll);
    let start = app.vr_packets_scroll as usize;

    let pct = if rows.is_empty() {
        100
    } else {
        ((start + visible).min(rows.len()) * 100) / rows.len()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Packets ({}) {}% [r]aw:{} [l]ine:{} ",
            pkt_data.total, pct,
            if app.vr_packets_raw { "on" } else { "off" },
            app.vr_packets_line.label(),
        ));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    for (i, (left, right)) in rows.iter().skip(start).enumerate() {
        if i >= visible {
            break;
        }
        let y = inner.y + i as u16;
        if !left.is_empty() {
            let left_area = Rect::new(inner.x, y, half_width, 1);
            let line = Line::from(left.clone());
            f.render_widget(Paragraph::new(line), left_area);
        }
        if !right.is_empty() {
            let right_area = Rect::new(inner.x + half_width, y, inner.width - half_width, 1);
            let line = Line::from(right.clone());
            f.render_widget(Paragraph::new(line), right_area);
        }
    }
}

pub(super) fn draw_column_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = (area.height as f32 * 0.8) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let mode_label = if app.vr_column_editor_mode == ColumnEditorMode::Reorder { " [REORDER] " } else { "" };
    let bottom = if app.vr_column_editor_filter.is_empty() {
        " space:toggle /:filter m:move a:apply d:default Esc:close "
    } else {
        " space:toggle ↑↓:navigate Esc:clear filter "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Columns{mode_label}"))
        .title_bottom(Line::from(bottom).fg(Color::DarkGray));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Filter bar at top
    let filter_height = 1u16;
    let filter_area = Rect::new(inner.x, inner.y, inner.width, filter_height);
    let list_area = Rect::new(inner.x, inner.y + filter_height, inner.width, inner.height.saturating_sub(filter_height));

    let filter_active = !app.vr_column_editor_filter.is_empty();
    let filter_text = app.vr_column_editor_filter.trim_matches('\0');
    let filter_display = if !filter_active {
        Line::from(vec![
            Span::styled("  / to filter", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Yellow)),
            Span::raw(filter_text),
            Span::styled("█", Style::default().fg(Color::White)),
        ])
    };
    f.render_widget(Paragraph::new(vec![filter_display]), filter_area);

    // Build filtered view
    let filter_lower = filter_text.to_lowercase();
    let filtered: Vec<usize> = if filter_lower.is_empty() {
        (0..app.vr_column_editor_available.len()).collect()
    } else {
        app.vr_column_editor_available.iter().enumerate()
            .filter(|(_, item)| {
                item.exp.to_lowercase().contains(&filter_lower)
                    || item.friendly_name.to_lowercase().contains(&filter_lower)
            })
            .map(|(i, _)| i)
            .collect()
    };

    let visible_rows = list_area.height as usize;
    let total = filtered.len();

    // Find position of selected in filtered list
    let sel_pos = filtered.iter().position(|&i| i == app.vr_column_editor_selected).unwrap_or(0);

    let scroll_offset = if sel_pos >= visible_rows {
        sel_pos - visible_rows + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for &idx in filtered.iter().skip(scroll_offset).take(visible_rows) {
        let item = &app.vr_column_editor_available[idx];
        let is_selected = idx == app.vr_column_editor_selected;
        let checkbox = if item.enabled { "[x] " } else { "[ ] " };
        let marker = if is_selected && app.vr_column_editor_mode == ColumnEditorMode::Reorder {
            "≡ "
        } else if is_selected {
            "► "
        } else {
            "  "
        };
        let display = if item.friendly_name.is_empty() || item.friendly_name == item.exp {
            item.exp.clone()
        } else {
            format!("{} ({})", item.exp, item.friendly_name)
        };
        let text = format!("{marker}{checkbox}{display}");
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if item.enabled {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(text).style(style));
    }

    // Show scroll indicator
    if total > visible_rows && !lines.is_empty() {
        let pct = if total > 1 { (sel_pos * 100) / (total - 1).max(1) } else { 0 };
        let indicator = format!(" ↕ {}/{} ({}%) ", sel_pos + 1, total, pct);
        let last = lines.len() - 1;
        lines[last] = Line::from(indicator).style(Style::default().fg(Color::DarkGray));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, list_area);
}

pub(super) fn draw_layout_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_width = 44u16.min(area.width.saturating_sub(4));
    let popup_height = (app.vr_saved_layouts.len() as u16 + 9).min(area.height.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    match app.vr_layout_popup_mode {
        LayoutPopupMode::ConfirmDelete => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Confirm Delete ");
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);
            let lines = vec![
                Line::from(""),
                Line::from(format!("  Delete layout '{}'?", app.vr_layout_delete_name))
                    .style(Style::default().fg(Color::Yellow)),
                Line::from(""),
                Line::from("  y: yes  any other key: cancel")
                    .style(Style::default().fg(Color::DarkGray)),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        LayoutPopupMode::List => {
            let filter_active = !app.vr_layout_filter.is_empty();
            let bottom = if filter_active {
                " Enter:select ↑↓:navigate Esc:clear "
            } else {
                " Enter:select /:filter x:delete Esc:close "
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Layouts ")
                .title_bottom(Line::from(bottom).fg(Color::DarkGray));

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let mut lines: Vec<Line> = Vec::new();

            if !filter_active {
                // "Edit Columns" option
                let style = if app.vr_layout_popup_selected == 0 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Magenta)
                };
                lines.push(Line::from("  ⚙ Edit Columns").style(style));

                // "Save Current" option
                let style = if app.vr_layout_popup_selected == 1 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                lines.push(Line::from("  [+] Save Current Layout").style(style));

                // "Default" option
                let style = if app.vr_layout_popup_selected == 2 {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from("  ↺ Default Columns").style(style));

                // Separator
                lines.push(Line::from("  ────────────────────────────────").style(Style::default().fg(Color::DarkGray)));
            } else {
                // Filter bar
                let filter_text = app.vr_layout_filter.trim_matches('\0');
                lines.push(Line::from(vec![
                    Span::styled("  /", Style::default().fg(Color::Yellow)),
                    Span::raw(filter_text),
                    Span::styled("█", Style::default().fg(Color::White)),
                ]));
            }

            // Saved layouts (filtered if filter active)
            let filter_text = app.vr_layout_filter.trim_matches('\0').to_lowercase();
            let mut any_shown = false;
            for (i, layout) in app.vr_saved_layouts.iter().enumerate() {
                if !filter_text.is_empty() && !layout.name.to_lowercase().contains(&filter_text) {
                    continue;
                }
                any_shown = true;
                let is_selected = app.vr_layout_popup_selected == i + 3;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };
                let col_count = layout.columns.len();
                lines.push(Line::from(format!("  {} ({} cols)", layout.name, col_count)).style(style));
            }

            if !any_shown {
                lines.push(Line::from("  (no saved layouts)").style(Style::default().fg(Color::DarkGray)));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
        LayoutPopupMode::SaveInput => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Save Layout ");

            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from("  Layout name:").style(Style::default().fg(Color::Yellow)));

            // Input field with cursor
            let name = &app.vr_layout_save_name;
            let cursor = app.vr_layout_save_cursor;
            let mut spans = vec![Span::raw("  ")];
            if cursor < name.len() {
                spans.push(Span::raw(&name[..cursor]));
                spans.push(Span::styled(&name[cursor..cursor+1], Style::default().bg(Color::White).fg(Color::Black)));
                spans.push(Span::raw(&name[cursor+1..]));
            } else {
                spans.push(Span::raw(name.as_str()));
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            }
            lines.push(Line::from(spans));

            lines.push(Line::from(""));
            lines.push(Line::from("  Enter: save  Esc: cancel").style(Style::default().fg(Color::DarkGray)));

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
    }
}

pub(super) fn draw_view_popup(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::ViewPopupMode;

    let filtered = app.view_filtered_indices();
    let popup_height = (filtered.len() as u16 + 8).min(area.height - 2).max(8);
    let popup_width = 60u16.min(area.width - 4);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let title = match app.vr_view_popup_mode {
        ViewPopupMode::SaveInput => " Save View ",
        ViewPopupMode::ConfirmDelete => " Confirm Delete ",
        _ => " Views ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title);
    f.render_widget(block, popup_area);

    let inner = Rect::new(popup_area.x + 1, popup_area.y + 1, popup_area.width - 2, popup_area.height - 2);

    match app.vr_view_popup_mode {
        ViewPopupMode::SaveInput => {
            let checkbox = if app.vr_view_save_columns { "[x]" } else { "[ ]" };
            let lines = vec![
                Line::from("Enter view name:"),
                Line::from(""),
                Line::from(Span::styled(&app.vr_view_save_name, Style::default().fg(Color::White).add_modifier(Modifier::UNDERLINED))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(checkbox, Style::default().fg(Color::Cyan)),
                    Span::styled(" Save current columns (Tab to toggle)", Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Expression: ", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(app.expression.clone(), Style::default().fg(Color::Gray))),
            ];
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
            let cursor_x = inner.x + app.vr_view_save_cursor as u16;
            let cursor_y = inner.y + 2;
            if cursor_x < inner.right() {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
        ViewPopupMode::ConfirmDelete => {
            let lines = vec![
                Line::from(vec![
                    Span::raw("Delete view "),
                    Span::styled(&app.vr_view_delete_name, Style::default().fg(Color::Yellow)),
                    Span::raw("?"),
                ]),
                Line::from(""),
                Line::from(Span::styled("y/N", Style::default().fg(Color::Red))),
            ];
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
        ViewPopupMode::List => {
            let mut lines: Vec<Line> = Vec::new();
            let active_marker = |id: &str| -> &str {
                if app.vr_active_view.as_deref() == Some(id) { " ●" } else { "" }
            };

            // Option 0: Save current expression as view
            let save_style = if app.vr_view_popup_selected == 0 {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Green)
            };
            lines.push(Line::from(Span::styled("[+] Save Current Expression as View", save_style)));

            // Option 1: Clear view
            let clear_style = if app.vr_view_popup_selected == 1 {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Red)
            };
            let clear_label = if app.vr_active_view.is_some() { "✖ Clear Active View" } else { "✖ No View Active" };
            lines.push(Line::from(Span::styled(clear_label, clear_style)));

            // Separator
            lines.push(Line::from(Span::styled("─".repeat(inner.width as usize), Style::default().fg(Color::DarkGray))));

            // Filter indicator
            if app.vr_view_filter_active {
                lines.push(Line::from(vec![
                    Span::styled("/", Style::default().fg(Color::DarkGray)),
                    Span::styled(&app.vr_view_filter, Style::default().fg(Color::Yellow)),
                ]));
            }

            // Views
            for (fi, &idx) in filtered.iter().enumerate() {
                let view = &app.vr_saved_views[idx];
                let selected = app.vr_view_popup_selected == fi + 2;
                let base_style = if selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                let mut spans = Vec::new();
                if view.shared {
                    spans.push(Span::styled("🔗 ", Style::default().fg(Color::Cyan)));
                } else {
                    spans.push(Span::raw("   "));
                }
                spans.push(Span::styled(&view.name, base_style));
                let marker = active_marker(&view.id);
                if !marker.is_empty() {
                    spans.push(Span::styled(marker, Style::default().fg(Color::Green)));
                }
                // Show expression in gray
                let remaining = inner.width as usize - view.name.len() - 4 - marker.len();
                if remaining > 5 {
                    let expr_display = if view.expression.len() > remaining {
                        format!(" {}..", &view.expression[..remaining - 3])
                    } else {
                        format!(" {}", view.expression)
                    };
                    spans.push(Span::styled(expr_display, Style::default().fg(Color::DarkGray)));
                }
                lines.push(Line::from(spans));
            }

            if filtered.is_empty() && !app.vr_saved_views.is_empty() {
                lines.push(Line::from(Span::styled("  (no matching views)", Style::default().fg(Color::DarkGray))));
            } else if app.vr_saved_views.is_empty() {
                lines.push(Line::from(Span::styled("  (no saved views)", Style::default().fg(Color::DarkGray))));
            }

            // Footer
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Enter=select  x=delete  /=filter  Esc=close",
                Style::default().fg(Color::DarkGray),
            )));

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);
        }
    }
}

pub(super) fn draw_loading(f: &mut Frame, app: &mut App, area: Rect) {
    let owl_right = [
        " ,___,  ",
        " (O,O)  ",
        " /)  )  ",
        "  \" \"   ",
        " _| |_  ",
    ];
    let owl_left = [
        "  ,___,  ",
        "  (O,O)  ",
        "  (  (\\  ",
        "   \" \"   ",
        "  _| |_  ",
    ];
    let popup_width = 30u16;
    let popup_height = (owl_right.len() + 5) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let owl_w = 10u16;
    let max_x = inner.width.saturating_sub(owl_w);

    // Animate owl position
    if app.loading_owl_tick.elapsed() >= std::time::Duration::from_millis(100) {
        app.loading_owl_tick = std::time::Instant::now();
        let new_x = app.loading_owl_x as i16 + app.loading_owl_dx;
        if new_x <= 0 {
            app.loading_owl_x = 0;
            app.loading_owl_dx = 1;
        } else if new_x >= max_x as i16 {
            app.loading_owl_x = max_x;
            app.loading_owl_dx = -1;
        } else {
            app.loading_owl_x = new_x as u16;
        }
    }

    let owl = if app.loading_owl_dx > 0 { &owl_right } else { &owl_left };

    // Draw owl at animated position
    for (i, row) in owl.iter().enumerate() {
        let y = inner.y + 1 + i as u16;
        if y < inner.y + inner.height {
            let x = inner.x + app.loading_owl_x;
            let owl_area = Rect::new(x, y, owl_w.min(inner.width - app.loading_owl_x), 1);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(*row, Style::default().fg(Color::Yellow)))),
                owl_area,
            );
        }
    }

    // Draw "Loading ..." text centered
    let loading_y = inner.y + owl_right.len() as u16 + 2;
    if loading_y < inner.y + inner.height {
        let text = "Loading ...";
        let text_x = inner.x + (inner.width.saturating_sub(text.len() as u16)) / 2;
        let text_area = Rect::new(text_x, loading_y, text.len() as u16, 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)))),
            text_area,
        );
    }
}
