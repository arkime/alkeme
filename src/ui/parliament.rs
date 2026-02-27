use super::*;
use crate::api::{PlClusterStats, PlIssue};

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
        _ => {
            let block = Block::default().borders(Borders::ALL).title(app.active_tab.name());
            f.render_widget(block, chunks[1]);
        }
    }

    draw_status_bar(f, app, chunks[2]);

    // Detail overlay
    if app.pl_show_detail && app.active_tab == Tab::Dashboard {
        draw_cluster_detail(f, app, f.area());
    }
}

fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    if app.pl_groups.is_empty() {
        let msg = Paragraph::new("No parliament data. Press 'r' to refresh.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Dashboard "));
        f.render_widget(msg, area);
        return;
    }

    // Build display lines for groups and clusters
    let mut lines: Vec<Line> = Vec::new();
    let nav_idx = app.pl_dashboard_nav_index();

    for (gi, group) in app.pl_groups.iter().enumerate() {
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
            let is_selected = app.pl_cluster_list.iter().position(|&(g, c)| g == gi && c == ci)
                .map(|idx| idx == nav_idx)
                .unwrap_or(false);

            let cluster_id = cluster.id.as_deref().unwrap_or("");
            let stats = app.pl_stats.get(cluster_id);
            let issues = app.pl_issues_map.get(cluster_id);

            let line = build_cluster_line(cluster, stats, issues, is_selected);
            lines.push(line);

            // Show issues inline (up to 3)
            if let Some(issues) = issues {
                for (i, issue) in issues.iter().take(3).enumerate() {
                    let severity_color = if issue.severity == "red" { Color::Red } else { Color::Yellow };
                    let prefix = if i == 0 { "  └─ " } else { "     " };
                    lines.push(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(
                            format!("⚠ {}", issue.title),
                            Style::default().fg(severity_color),
                        ),
                        Span::raw(": "),
                        Span::styled(&issue.message, Style::default().fg(Color::White)),
                        if !issue.node.is_empty() {
                            Span::styled(format!(" ({})", issue.node), Style::default().fg(Color::DarkGray))
                        } else {
                            Span::raw("")
                        },
                    ]));
                }
                if issues.len() > 3 {
                    lines.push(Line::from(Span::styled(
                        format!("     ... and {} more", issues.len() - 3),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }
        lines.push(Line::from("")); // spacer between groups
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Dashboard (↑/↓ navigate, Enter=open cluster, i=detail, r=refresh) ");

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((0, 0));

    f.render_widget(paragraph, area);
}

fn build_cluster_line<'a>(
    cluster: &crate::api::PlCluster,
    stats: Option<&PlClusterStats>,
    issues: Option<&Vec<PlIssue>>,
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
                format!(" {:>6}d/s", stats.delta_tdps as u64),
                Style::default().fg(drops_color),
            ));

            // Monitoring sessions
            let mon_color = if stats.monitoring == 0 { Color::Yellow } else { Color::Cyan };
            spans.push(Span::styled(
                format!(" {:>6}sess", stats.monitoring),
                Style::default().fg(mon_color),
            ));

            // Arkime nodes
            spans.push(Span::styled(
                format!(" {:>3}nodes", stats.arkime_nodes),
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

    // Issue count
    if let Some(issues) = issues {
        if !issues.is_empty() {
            let has_red = issues.iter().any(|i| i.severity == "red");
            let color = if has_red { Color::Red } else { Color::Yellow };
            spans.push(Span::styled(
                format!(" [{}⚠]", issues.len()),
                Style::default().fg(color),
            ));
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

    // Filter bar
    let filter_display = if app.input_mode == InputMode::Expression {
        &app.pl_issues_filter_edit
    } else {
        &app.pl_issues_filter
    };
    let filter_style = if app.input_mode == InputMode::Expression {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let sort_indicator = format!("{} {}", app.pl_issues_sort.label(),
        if app.pl_issues_sort_desc { "▼" } else { "▲" });

    let filter_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(sort_indicator.len() as u16 + 4),
        ])
        .split(chunks[0]);

    let filter_widget = Paragraph::new(Span::styled(filter_display.as_str(), filter_style))
        .block(Block::default().borders(Borders::ALL).title(" Filter (/) "));
    f.render_widget(filter_widget, filter_chunks[0]);

    if app.input_mode == InputMode::Expression {
        f.set_cursor_position((
            filter_chunks[0].x + app.expression_cursor as u16 + 1,
            filter_chunks[0].y + 1,
        ));
    }

    let sort_widget = Paragraph::new(Span::styled(
        &sort_indicator,
        Style::default().fg(Color::Cyan),
    ))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" s/S "));
    f.render_widget(sort_widget, filter_chunks[1]);

    // Issues table
    let filtered = app.pl_filtered_issues();
    let total = app.pl_issues.len();

    let sort_arrow = if app.pl_issues_sort_desc { "▼" } else { "▲" };
    let pl_sort_hdr = |sort: PlIssueSort, label: &str| -> Cell {
        let (text, style) = if app.pl_issues_sort == sort {
            (format!("{label} {sort_arrow}"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            (label.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        };
        Cell::from(text).style(style)
    };
    let header = Row::new(vec![
        pl_sort_hdr(PlIssueSort::Cluster, "Cluster"),
        pl_sort_hdr(PlIssueSort::Severity, "Severity"),
        pl_sort_hdr(PlIssueSort::Title, "Title"),
        Cell::from("Node").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Message").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        pl_sort_hdr(PlIssueSort::FirstNoticed, "First Noticed"),
        pl_sort_hdr(PlIssueSort::LastNoticed, "Last Noticed"),
    ]);

    let rows: Vec<Row> = filtered.iter().enumerate().map(|(i, issue)| {
        let severity_color = if issue.severity == "red" { Color::Red } else { Color::Yellow };
        let style = if i == app.pl_issues_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

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
        ]).style(style)
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
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Issues [{}/{}] ", filtered.len(), total)));

    f.render_widget(table, chunks[1]);
}

fn draw_cluster_detail(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    let nav_idx = app.pl_dashboard_nav_index();
    let (gi, ci) = if nav_idx < app.pl_cluster_list.len() {
        app.pl_cluster_list[nav_idx]
    } else {
        return;
    };

    let cluster = match app.pl_groups.get(gi).and_then(|g| g.clusters.get(ci)) {
        Some(c) => c,
        None => return,
    };

    let group = &app.pl_groups[gi];
    let cluster_id = cluster.id.as_deref().unwrap_or("");
    let stats = app.pl_stats.get(cluster_id);
    let issues = app.pl_issues_map.get(cluster_id);

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
        .scroll((app.pl_detail_scroll, 0));

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn format_human_bps(bps: f64) -> String {
    if bps >= 1_000_000_000.0 {
        format!("{:.1} Gbps", bps * 8.0 / 1_000_000_000.0)
    } else if bps >= 1_000_000.0 {
        format!("{:.1} Mbps", bps * 8.0 / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.1} Kbps", bps * 8.0 / 1_000.0)
    } else {
        format!("{:.0} bps", bps * 8.0)
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
