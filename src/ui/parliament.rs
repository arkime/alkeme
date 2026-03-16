use super::*;
use crate::api::PlClusterStats;

pub fn draw_parliament(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),   // content
            Constraint::Length(status_h), // status
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Dashboard => draw_dashboard(f, app, chunks[1]),
        Tab::Issues => draw_issues(f, app, chunks[1]),
        Tab::Settings => draw_pl_settings(f, app, chunks[1]),
        Tab::Users => super::users::draw_users_tab(f, app, chunks[1]),
        _ => {
            let block = Block::default().borders(Borders::ALL).title(app.active_tab.name());
            f.render_widget(block, chunks[1]);
            arkime::draw_under_construction(f, app, chunks[1]);
            arkime::draw_owl(f, app, chunks[1]);
        }
    }

    draw_status_bar(f, app, chunks[2]);

    // Detail overlay
    if app.parliament.show_detail && app.active_tab == Tab::Dashboard {
        draw_cluster_detail(f, app, f.area());
    }
}

fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    if app.parliament.groups.is_empty() {
        let msg = Paragraph::new("No parliament data. Press 'r' to refresh.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Dashboard "));
        f.render_widget(msg, area);
        return;
    }

    // Build display lines for groups and clusters
    let mut lines: Vec<Line> = Vec::new();
    let nav_idx = app.pl_dashboard_nav_index();

    for (gi, group) in app.parliament.groups.iter().enumerate() {
        // Group header
        lines.push(Line::from(vec![
            Span::styled(
                format!("━━━ {} ━━━", group.title),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));

        if !group.description.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    {}", group.description),
                Style::default().fg(Color::DarkGray),
            )));
        }

        for (ci, cluster) in group.clusters.iter().enumerate() {
            let is_selected = app.parliament.cluster_list.iter().position(|&(g, c)| g == gi && c == ci)
                .map(|idx| idx == nav_idx)
                .unwrap_or(false);

            let cluster_id = cluster.id.as_deref().unwrap_or("");
            let stats = app.parliament.stats.get(cluster_id);
            let issues = app.parliament.issues_map.get(cluster_id);

            let line = build_cluster_line(cluster, stats, is_selected);
            lines.push(line);

            // Show issues as grouped counts (e.g., "2 Out of Date, 1 Low Packets")
            if let Some(issues) = issues {
                if !issues.is_empty() {
                    // Count issues by title
                    let mut counts: Vec<(String, u32, bool)> = Vec::new();
                    for issue in issues {
                        let is_red = issue.severity == "red";
                        if let Some(entry) = counts.iter_mut().find(|(t, _, _)| *t == issue.title) {
                            entry.1 += 1;
                            if is_red { entry.2 = true; }
                        } else {
                            counts.push((issue.title.clone(), 1, is_red));
                        }
                    }
                    let mut issue_spans: Vec<Span> = vec![Span::raw("     └─ ")];
                    for (i, (title, count, is_red)) in counts.iter().enumerate() {
                        if i > 0 {
                            issue_spans.push(Span::raw(", "));
                        }
                        let color = if *is_red { Color::LightRed } else { Color::Yellow };
                        issue_spans.push(Span::styled(
                            format!("{} {}", count, title),
                            Style::default().fg(color),
                        ));
                    }
                    lines.push(Line::from(issue_spans));
                }
            }
        }
        lines.push(Line::from("")); // spacer between groups
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Dashboard (↑/↓ navigate, Enter=open cluster, i=detail, r=refresh) ");

    // Auto-scroll to keep selected cluster visible
    let content_height = area.height.saturating_sub(2) as u16; // borders
    app.visible_rows = content_height as usize;
    // Calculate line index of selected cluster
    let mut selected_line: u16 = 0;
    let mut found = false;
    let mut line_count: u16 = 0;
    for (gi, group) in app.parliament.groups.iter().enumerate() {
        line_count += 1; // group header
        if !group.description.is_empty() {
            line_count += 1;
        }
        for (ci, _cluster) in group.clusters.iter().enumerate() {
            if app.parliament.cluster_list.iter().position(|&(g, c)| g == gi && c == ci)
                .map(|idx| idx == nav_idx)
                .unwrap_or(false) && !found
            {
                selected_line = line_count;
                found = true;
            }
            line_count += 1; // cluster line
            let cluster_id = _cluster.id.as_deref().unwrap_or("");
            if let Some(issues) = app.parliament.issues_map.get(cluster_id) {
                if !issues.is_empty() {
                    line_count += 1; // single grouped-counts line
                }
            }
        }
        line_count += 1; // spacer
    }
    if found && content_height > 0 {
        if selected_line < app.parliament.dashboard_scroll {
            app.parliament.dashboard_scroll = selected_line;
        } else if selected_line >= app.parliament.dashboard_scroll + content_height {
            app.parliament.dashboard_scroll = selected_line - content_height + 1;
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.parliament.dashboard_scroll, 0));

    f.render_widget(paragraph, area);
}

fn build_cluster_line<'a>(
    cluster: &crate::api::PlCluster,
    stats: Option<&PlClusterStats>,
    is_selected: bool,
) -> Line<'a> {
    let mut spans: Vec<Span> = Vec::new();

    // Selection indicator
    if is_selected {
        spans.push(Span::styled(" ► ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    } else {
        spans.push(Span::raw("   "));
    }

    // Type icon
    let type_icon = match cluster.cluster_type.as_str() {
        "disabled" => "⊘ ",
        "multiviewer" => "⌂ ",
        "noAlerts" => "🔕",
        _ => "  ",
    };
    spans.push(Span::styled(type_icon, Style::default().fg(Color::DarkGray)));

    // Health status indicator
    if let Some(stats) = stats {
        let (status_char, status_color) = match stats.status.as_str() {
            "green" => ("●", Color::Green),
            "yellow" => ("●", Color::Yellow),
            "red" => ("●", Color::Red),
            _ => ("○", Color::DarkGray),
        };
        spans.push(Span::styled(format!("{} ", status_char), Style::default().fg(status_color)));
    } else if cluster.cluster_type == "disabled" {
        spans.push(Span::styled("○ ", Style::default().fg(Color::DarkGray)));
    } else {
        spans.push(Span::styled("? ", Style::default().fg(Color::DarkGray)));
    }

    // Cluster title
    let title_style = if is_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if cluster.cluster_type == "disabled" {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    spans.push(Span::styled(
        format!("{:<24}", cluster.title),
        title_style,
    ));

    // Stats (only for non-disabled, non-multiviewer clusters)
    if cluster.cluster_type != "disabled" && cluster.cluster_type != "multiviewer" {
        if let Some(stats) = stats {
            // BPS
            spans.push(Span::styled(
                format!(" {:>10}", format_human_bps(stats.delta_bps)),
                Style::default().fg(Color::Blue),
            ));

            // Drops/sec
            let drops_color = if stats.delta_tdps > 0.0 { Color::Red } else { Color::Green };
            spans.push(Span::styled(
                format!(" {:>6} d/s", stats.delta_tdps as u64),
                Style::default().fg(drops_color),
            ));

            // Monitoring sessions
            let mon_color = if stats.monitoring == 0 { Color::Yellow } else { Color::Cyan };
            spans.push(Span::styled(
                format!(" {:>11} sess", format_number(stats.monitoring as u64)),
                Style::default().fg(mon_color),
            ));

            // Arkime nodes
            spans.push(Span::styled(
                format!(" {:>3} nodes", stats.arkime_nodes),
                Style::default().fg(Color::White),
            ));

            // ES nodes
            spans.push(Span::styled(
                format!(" {}/{}", stats.data_nodes, stats.total_nodes),
                Style::default().fg(Color::DarkGray),
            ));

            // ES version
            if !stats.es_version.is_empty() {
                spans.push(Span::styled(
                    format!(" ES:{}", stats.es_version),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Error indicators
            if !stats.health_error.is_empty() {
                spans.push(Span::styled(" ⚠health", Style::default().fg(Color::Red)));
            }
            if !stats.stats_error.is_empty() {
                spans.push(Span::styled(" ⚠stats", Style::default().fg(Color::Red)));
            }
        }
    }

    Line::from(spans)
}

fn draw_issues(f: &mut Frame, app: &mut App, area: Rect) {
    // Toolbar for filter
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // filter bar
            Constraint::Min(0),   // table
        ])
        .split(area);

    app.visible_rows = chunks[1].height.saturating_sub(3) as usize;

    // Filter bar
    let filter_display = if app.input_mode == InputMode::Expression {
        &app.parliament.issues_filter_edit
    } else {
        &app.parliament.issues_filter
    };

    let sort_indicator = format!("{} {}", app.parliament.issues_sort.label(),
        if app.parliament.issues_sort_desc { "▼" } else { "▲" });

    let filter_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(sort_indicator.len() as u16 + 4),
        ])
        .split(chunks[0]);

    let is_editing = app.input_mode == InputMode::Expression;
    render_text_input(f, filter_display, app.expression_cursor, is_editing, " Filter (/) ", filter_chunks[0]);

    let sort_widget = Paragraph::new(Span::styled(
        &sort_indicator,
        Style::default().fg(Color::Cyan),
    ))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" s/S "));
    f.render_widget(sort_widget, filter_chunks[1]);

    // Issues table
    let filtered = app.pl_filtered_issues();
    let total = app.parliament.issues.len();

    let pl_sort_hdr = |sort: PlIssueSort, label: &str| -> Cell {
        let is_sorted = app.parliament.issues_sort == sort;
        Cell::from(sort_header_label(label, is_sorted, app.parliament.issues_sort_desc))
            .style(sort_header_style(is_sorted))
    };
    let header = Row::new(vec![
        pl_sort_hdr(PlIssueSort::Cluster, "Cluster"),
        pl_sort_hdr(PlIssueSort::Severity, "Severity"),
        pl_sort_hdr(PlIssueSort::Title, "Title"),
        Cell::from("Node").style(sort_header_style(false)),
        Cell::from("Message").style(sort_header_style(false)),
        pl_sort_hdr(PlIssueSort::FirstNoticed, "First Noticed"),
        pl_sort_hdr(PlIssueSort::LastNoticed, "Last Noticed"),
    ]);

    let rows: Vec<Row> = filtered.iter().map(|issue| {
        let severity_color = if issue.severity == "red" { Color::Red } else { Color::Yellow };

        let mut ack_prefix = String::new();
        if issue.acknowledged.is_some() {
            ack_prefix.push_str("✓ ");
        }
        if issue.is_ignored() {
            ack_prefix.push_str("⊘ ");
        }

        Row::new(vec![
            Cell::from(issue.cluster.clone()),
            Cell::from(format!("{}{}", ack_prefix, issue.severity.clone()))
                .style(Style::default().fg(severity_color)),
            Cell::from(issue.title.clone()),
            Cell::from(issue.node.clone()),
            Cell::from(issue.message.clone()),
            Cell::from(format_epoch_ms_full(issue.first_noticed)),
            Cell::from(format_epoch_ms_full(issue.last_noticed)),
        ])
    }).collect();

    let widths = [
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Min(20),
        Constraint::Length(20),
        Constraint::Length(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Issues [{}/{}] ", filtered.len(), total)));

    f.render_stateful_widget(table, chunks[1], &mut app.parliament.issues_table_state);
}

fn draw_cluster_detail(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    let nav_idx = app.pl_dashboard_nav_index();
    let (gi, ci) = if nav_idx < app.parliament.cluster_list.len() {
        app.parliament.cluster_list[nav_idx]
    } else {
        return;
    };

    let cluster = match app.parliament.groups.get(gi).and_then(|g| g.clusters.get(ci)) {
        Some(c) => c,
        None => return,
    };

    let group = &app.parliament.groups[gi];
    let cluster_id = cluster.id.as_deref().unwrap_or("");
    let stats = app.parliament.stats.get(cluster_id);
    let issues = app.parliament.issues_map.get(cluster_id);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Group: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&group.title, Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Cluster: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&cluster.title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));
    if !cluster.description.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&cluster.description, Style::default().fg(Color::White)),
        ]));
    }
    if !cluster.url.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("URL: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&cluster.url, Style::default().fg(Color::Blue)),
        ]));
    }
    if !cluster.cluster_type.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&cluster.cluster_type, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    if let Some(stats) = stats {
        lines.push(Line::from(Span::styled(
            "── Stats ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        let status_color = match stats.status.as_str() {
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "red" => Color::Red,
            _ => Color::DarkGray,
        };
        lines.push(Line::from(vec![
            Span::styled("  Health: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&stats.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]));

        if !stats.es_version.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ES Version: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&stats.es_version, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("  Bytes/sec: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_human_bps(stats.delta_bps), Style::default().fg(Color::Blue)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Drops/sec: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", stats.delta_tdps as u64),
                Style::default().fg(if stats.delta_tdps > 0.0 { Color::Red } else { Color::Green }),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Monitoring: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} sessions", stats.monitoring), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Arkime Nodes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", stats.arkime_nodes), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ES Nodes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} data / {} total", stats.data_nodes, stats.total_nodes), Style::default().fg(Color::White)),
        ]));

        if !stats.health_error.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Health Error: ", Style::default().fg(Color::Red)),
                Span::styled(&stats.health_error, Style::default().fg(Color::Red)),
            ]));
        }
        if !stats.stats_error.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Stats Error: ", Style::default().fg(Color::Red)),
                Span::styled(&stats.stats_error, Style::default().fg(Color::Red)),
            ]));
        }
    }

    lines.push(Line::from(""));

    if let Some(issues) = issues {
        lines.push(Line::from(Span::styled(
            format!("── Issues ({}) ──", issues.len()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for issue in issues {
            let severity_color = if issue.severity == "red" { Color::Red } else { Color::Yellow };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", issue.severity), Style::default().fg(severity_color)),
                Span::styled(&issue.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
                Span::styled(&issue.message, Style::default().fg(Color::White)),
            ]));
            if !issue.node.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("    Node: "),
                    Span::styled(&issue.node, Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::raw("    First: "),
                Span::styled(format_epoch_ms_full(issue.first_noticed), Style::default().fg(Color::DarkGray)),
                Span::raw("  Last: "),
                Span::styled(format_epoch_ms_full(issue.last_noticed), Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "── No Issues ──",
            Style::default().fg(Color::Green),
        )));
    }

    let title = format!(" {} - {} (Esc to close) ", group.title, cluster.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.parliament.detail_scroll, 0));

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn format_human_bps(bps: f64) -> String {
    let bits = bps * 8.0;
    if bits >= 1_000_000_000.0 {
        format!("{:.1} Gbps", bits / 1_000_000_000.0)
    } else if bits >= 1_000_000.0 {
        format!("{:.1} Mbps", bits / 1_000_000.0)
    } else if bits >= 1_000.0 {
        format!("{:.1} Kbps", bits / 1_000.0)
    } else {
        format!("{:.0} bps", bits)
    }
}

fn format_epoch_ms_full(ms: u64) -> String {
    if ms == 0 {
        return "-".into();
    }
    let secs = (ms / 1000) as i64;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
        let local: chrono::DateTime<chrono::Local> = dt.into();
        return local.format("%Y/%m/%d %H:%M:%S").to_string();
    }
    "-".into()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ── Parliament Settings ──────────────────────────────────────────────────────

fn draw_pl_settings(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::{PlSettingsTab, PlSettingsLevel};

    let sub_tabs: Vec<Span> = PlSettingsTab::ALL.iter().enumerate().map(|(i, tab)| {
        let label = format!(" {}:{} ", i + 1, tab.label());
        if *tab == app.parliament.settings_tab {
            Span::styled(label, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(label, Style::default().fg(Color::DarkGray))
        }
    }).collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    f.render_widget(Paragraph::new(Line::from(sub_tabs)), chunks[0]);

    match app.parliament.settings_tab {
        PlSettingsTab::Groups => draw_pl_groups(f, app, chunks[1]),
        PlSettingsTab::General => draw_pl_general(f, app, chunks[1]),
        PlSettingsTab::Notifiers => super::arkime::draw_under_construction(f, app, chunks[1]),
    }

    // Draw group/cluster editor popup on top if active
    match app.parliament.settings_level {
        PlSettingsLevel::GroupEditor => draw_pl_group_editor(f, app, area),
        PlSettingsLevel::ClusterEditor => draw_pl_cluster_editor(f, app, area),
        _ => {}
    }

    // Draw backup filename prompt on top
    if let Some(ref filename) = app.parliament.backup_prompt {
        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 3u16;
        let popup_area = center_popup(popup_width, popup_height, area);
        f.render_widget(Clear, popup_area);

        let cursor = app.parliament.backup_cursor;
        let (before, after) = filename.split_at(cursor.min(filename.len()));
        let line = Line::from(vec![
            Span::styled("Filename: ", Style::default().fg(Color::Yellow)),
            Span::styled(before, Style::default().fg(Color::White)),
            Span::styled(if after.is_empty() { " ".to_string() } else { after[..1].to_string() }, Style::default().fg(Color::Black).bg(Color::White)),
            Span::styled(if after.len() > 1 { &after[1..] } else { "" }, Style::default().fg(Color::White)),
        ]);
        let paragraph = Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Backup Groups "),
            );
        f.render_widget(paragraph, popup_area);
    }
}

fn draw_pl_groups(f: &mut Frame, app: &mut App, area: Rect) {
    let items = &app.parliament.settings_items;

    let rows: Vec<Row> = items.iter().map(|&(gi, ci_opt)| {
        match ci_opt {
            None => {
                let group = &app.parliament.groups[gi];
                let cluster_count = group.clusters.len();
                Row::new(vec![
                    Cell::from(format!("▸ {}", group.title))
                        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Cell::from(group.description.clone()),
                    Cell::from(format!("{} clusters", cluster_count)),
                    Cell::from(group.id.chars().take(8).collect::<String>())
                        .style(Style::default().fg(Color::DarkGray)),
                ])
            }
            Some(ci) => {
                let cluster = &app.parliament.groups[gi].clusters[ci];
                let type_str = if cluster.cluster_type.is_empty() {
                    "".to_string()
                } else {
                    format!("[{}]", cluster.cluster_type)
                };
                Row::new(vec![
                    Cell::from(format!("  └ {}", cluster.title)),
                    Cell::from(cluster.url.clone())
                        .style(Style::default().fg(Color::Blue)),
                    Cell::from(type_str)
                        .style(Style::default().fg(Color::DarkGray)),
                    Cell::from(cluster.id.as_deref().unwrap_or("").chars().take(8).collect::<String>())
                        .style(Style::default().fg(Color::DarkGray)),
                ])
            }
        }
    }).collect();

    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("URL / Description").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Info").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("ID").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]).bottom_margin(0);

    let table = Table::new(
        rows,
        [
            Constraint::Min(25),
            Constraint::Percentage(40),
            Constraint::Length(15),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(
        " Groups  n:new group  a:add cluster  e/Enter:edit  d:delete  Ctrl+S:save "
    ))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.parliament.settings_table_state);
}

fn draw_pl_group_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = center_popup(60, 9, area);
    f.render_widget(Clear, popup_area);

    let title = if app.parliament.group_editor_is_new {
        " New Group "
    } else {
        " Edit Group "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Min(4),   // Description (bordered paragraph)
            Constraint::Length(1), // Footer
        ])
        .split(inner);

    let is_title = app.parliament.group_editor_field == crate::app::PlGroupEditorField::Title;
    let label_w = 14u16;

    // Title field
    let title_style = if is_title { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::White) };
    let title_line = Line::from(vec![
        Span::styled("Title:        ", Style::default().fg(Color::Cyan)),
        Span::styled(app.parliament.group_editor_title.clone(), title_style),
    ]);
    f.render_widget(Paragraph::new(title_line), chunks[0]);
    if is_title {
        let cursor = app.parliament.group_editor_title_cursor.min(app.parliament.group_editor_title.len());
        f.set_cursor_position((chunks[0].x + label_w + cursor as u16, chunks[0].y));
    }

    // Description field (bordered paragraph)
    let desc_style = if !is_title { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::White) };
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if !is_title { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) })
        .title("Description");
    let desc_inner = desc_block.inner(chunks[1]);
    let desc_text = &app.parliament.group_editor_desc;
    let inner_width = desc_inner.width as usize;
    let cursor = app.parliament.group_editor_desc_cursor.min(desc_text.len());

    // Calculate which row/col the cursor is on with wrapping
    let mut row = 0u16;
    let mut col = 0u16;
    for (i, ch) in desc_text.chars().enumerate() {
        if i == cursor { break; }
        if ch == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
            if inner_width > 0 && col as usize >= inner_width {
                row += 1;
                col = 0;
            }
        }
    }
    let scroll = if row >= desc_inner.height { row - desc_inner.height + 1 } else { 0 };

    let desc_lines: Vec<Line> = desc_text.split('\n')
        .map(|l| Line::from(Span::styled(l, desc_style)))
        .collect();

    f.render_widget(
        Paragraph::new(desc_lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(desc_block),
        chunks[1],
    );
    if !is_title {
        f.set_cursor_position((desc_inner.x + col, desc_inner.y + row - scroll));
    }

    f.render_widget(
        Paragraph::new(" Tab:switch field  Ctrl+S:save  Esc:cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

fn draw_pl_cluster_editor(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::PlClusterEditorField;

    let popup_area = center_popup(70, 18, area);
    f.render_widget(Clear, popup_area);

    let title = if app.parliament.cluster_editor_is_new {
        " New Cluster "
    } else {
        " Edit Cluster "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let p = &app.parliament;
    let field = p.cluster_editor_field;
    let label_w = 20u16; // Widest label "Hide Arkime Nodes: "

    // Build constraints: Description gets extra space, others get 1 line
    let constraints: Vec<Constraint> = PlClusterEditorField::ALL.iter()
        .map(|fd| {
            if *fd == PlClusterEditorField::Description {
                Constraint::Min(4) // bordered paragraph
            } else {
                Constraint::Length(1)
            }
        })
        .chain(std::iter::once(Constraint::Length(1))) // footer
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, f_def) in PlClusterEditorField::ALL.iter().enumerate() {
        let is_active = *f_def == field;

        if f_def.is_bool() {
            let val = match f_def {
                PlClusterEditorField::HideDeltaBPS => p.cluster_editor_hide_delta_bps,
                PlClusterEditorField::HideDeltaTDPS => p.cluster_editor_hide_delta_tdps,
                PlClusterEditorField::HideMonitoring => p.cluster_editor_hide_monitoring,
                PlClusterEditorField::HideArkimeNodes => p.cluster_editor_hide_arkime_nodes,
                PlClusterEditorField::HideDataNodes => p.cluster_editor_hide_data_nodes,
                PlClusterEditorField::HideTotalNodes => p.cluster_editor_hide_total_nodes,
                _ => false,
            };
            let checkbox = if val { "☑" } else { "☐" };
            let style = if is_active {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let indicator = if is_active { "▸ " } else { "  " };
            f.render_widget(
                Paragraph::new(format!("{}{}: {} {}", indicator, f_def.label(), checkbox, if val { "Yes" } else { "No" })).style(style),
                chunks[i],
            );
        } else if *f_def == PlClusterEditorField::Type {
            let val = if p.cluster_editor_type.is_empty() { "normal" } else { &p.cluster_editor_type };
            let style = if is_active {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let indicator = if is_active { "▸ " } else { "  " };
            f.render_widget(
                Paragraph::new(format!("{}{}: {} (Enter to cycle)", indicator, f_def.label(), val)).style(style),
                chunks[i],
            );
        } else if *f_def == PlClusterEditorField::Description {
            // Multiline bordered paragraph
            let desc_style = if is_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::White) };
            let desc_block = Block::default()
                .borders(Borders::ALL)
                .border_style(if is_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) })
                .title("Description");
            let desc_inner = desc_block.inner(chunks[i]);
            let desc_text = &p.cluster_editor_desc;
            let cursor = p.cluster_editor_desc_cursor.min(desc_text.len());
            let inner_width = desc_inner.width as usize;

            let mut row = 0u16;
            let mut col = 0u16;
            for (ci, ch) in desc_text.chars().enumerate() {
                if ci == cursor { break; }
                if ch == '\n' {
                    row += 1;
                    col = 0;
                } else {
                    col += 1;
                    if inner_width > 0 && col as usize >= inner_width {
                        row += 1;
                        col = 0;
                    }
                }
            }
            let scroll = if row >= desc_inner.height { row - desc_inner.height + 1 } else { 0 };

            let desc_lines: Vec<Line> = desc_text.split('\n')
                .map(|l| Line::from(Span::styled(l, desc_style)))
                .collect();

            f.render_widget(
                Paragraph::new(desc_lines)
                    .wrap(Wrap { trim: false })
                    .scroll((scroll, 0))
                    .block(desc_block),
                chunks[i],
            );
            if is_active {
                f.set_cursor_position((desc_inner.x + col, desc_inner.y + row - scroll));
            }
        } else {
            // Inline text fields: Title, URL, Local URL
            let (text, text_cursor) = match f_def {
                PlClusterEditorField::Title => (&p.cluster_editor_title, p.cluster_editor_title_cursor),
                PlClusterEditorField::Url => (&p.cluster_editor_url, p.cluster_editor_url_cursor),
                PlClusterEditorField::LocalUrl => (&p.cluster_editor_local_url, p.cluster_editor_local_url_cursor),
                _ => continue,
            };
            let val_style = if is_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::White) };
            let indicator = if is_active { "▸ " } else { "  " };
            let padded_label = format!("{}{:<18}", indicator, format!("{}:", f_def.label()));
            let line = Line::from(vec![
                Span::styled(padded_label, Style::default().fg(Color::Cyan)),
                Span::styled(text.clone(), val_style),
            ]);
            f.render_widget(Paragraph::new(line), chunks[i]);
            if is_active {
                let cursor = text_cursor.min(text.len());
                f.set_cursor_position((chunks[i].x + label_w + cursor as u16, chunks[i].y));
            }
        }
    }

    let footer_idx = PlClusterEditorField::ALL.len();
    if footer_idx < chunks.len() {
        f.render_widget(
            Paragraph::new(" Tab:next  Space:toggle  Ctrl+S:save  Esc:cancel")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[footer_idx],
        );
    }
}

fn draw_pl_general(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::PlGeneralField;

    let rows: Vec<Row> = PlGeneralField::ALL.iter().enumerate().map(|(i, field)| {
        let is_selected = i == app.parliament.general_selected;
        let label = field.label();

        if app.parliament.general_editing && is_selected {
            let text = &app.parliament.general_edit_value;
            Row::new(vec![
                Cell::from(label.to_string()),
                Cell::from(Span::styled(text.clone(), Style::default().fg(Color::Yellow))),
            ])
        } else {
            let value = if field.is_select() {
                let val = app.pl_general_field_value(field);
                format!("{} (Enter to toggle)", val)
            } else {
                app.pl_general_field_value(field)
            };
            Row::new(vec![
                Cell::from(label.to_string()),
                Cell::from(value),
            ])
        }
    }).collect();

    let header = Row::new(vec![
        Cell::from("Setting").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Value").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);

    let table = Table::new(
        rows,
        [
            Constraint::Length(30),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(
        " General Settings  Enter:edit  Ctrl+S:save "
    ))
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol("▸ ");

    let mut table_state = TableState::default().with_selected(app.parliament.general_selected);
    f.render_stateful_widget(table, area, &mut table_state);

    // Show real blinking cursor when editing a field
    if app.parliament.general_editing {
        let cursor = app.parliament.general_edit_cursor.min(app.parliament.general_edit_value.len());
        let offset = table_state.offset();
        let row_in_view = app.parliament.general_selected.saturating_sub(offset);
        // x: border(1) + highlight_symbol(2) + first_col(30) + col_gap(1) + cursor_pos
        let cursor_x = area.x + 1 + 2 + 30 + 1 + cursor as u16;
        // y: border(1) + header(1) + row_index
        let cursor_y = area.y + 1 + 1 + row_in_view as u16;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
