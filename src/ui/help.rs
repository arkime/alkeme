use super::*;

pub(super) fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let key = |k: &str| Span::styled(format!("  {k:19}"), Style::default().fg(Color::Yellow));
    let blank = || Line::from("");

    macro_rules! hdr {
        ($s:expr) => { Line::from(Span::styled($s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))) };
    }

    let (title, help_text) = if app.viewer.packets_view.is_some() {
        ("Packets", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Scroll one page")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Scroll one page")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→"), Span::raw("Jump to bottom")]),
            Line::from(vec![key("Esc / p / q"), Span::raw("Close packets view")]),
            blank(),
            hdr!("Options"),
            blank(),
            Line::from(vec![key("r"), Span::raw("Toggle raw packets")]),
            Line::from(vec![key("l"), Span::raw("Cycle line numbers: hex/dec/off")]),
            blank(),
            hdr!("Colors"),
            blank(),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Cyan)), Span::raw("Source packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::Green)), Span::raw("Destination packets")]),
            Line::from(vec![Span::styled("  ██               ", Style::default().fg(Color::DarkGray)), Span::raw("Hex offset")]),
        ])
    } else if app.viewer.session_view == SessionView::Detail && app.active_tab == Tab::Sessions {
        ("Session Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Add field to expression")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("E"), Span::raw("Edit expression")]),
            Line::from(vec![key("a"), Span::raw("Session actions")]),
            Line::from(vec![key("A"), Span::raw("All sessions actions")]),
        ])
    } else if app.active_tab == Tab::Stats && app.viewer.stats_view == StatsView::Detail {
        ("Stats Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Scroll one line")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / Home"), Span::raw("Jump to top")]),
            Line::from(vec![key("→ / End"), Span::raw("Jump to bottom")]),
            Line::from(vec![key("Esc / q"), Span::raw("Close detail")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("E"), Span::raw("Edit expression")]),
        ])
    } else if app.active_tab == Tab::Stats {
        let mut lines = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("1 / 2 / 3 / 4 / 5"), Span::raw("Switch sub-tab")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open detail")]),
            Line::from(vec![key("/ / E"), Span::raw("Filter / edit expression")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("c"), Span::raw("Columns & layouts")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("Esc"), Span::raw("Close overlay")]),
        ];
        if app.viewer.stats_tab == StatsTab::DBIndices {
            lines.push(blank());
            lines.push(hdr!("Index Operations"));
            lines.push(blank());
            lines.push(Line::from(vec![key("d"), Span::raw("Delete index")]));
            lines.push(Line::from(vec![key("f"), Span::raw("Force merge index")]));
            lines.push(Line::from(vec![key("C"), Span::raw("Close index (if open)")]));
            lines.push(Line::from(vec![key("O"), Span::raw("Open index (if closed)")]));
        }
        if app.viewer.stats_tab == StatsTab::DBStats {
            lines.push(blank());
            lines.push(hdr!("Node Operations"));
            lines.push(blank());
            lines.push(Line::from(vec![key("e"), Span::raw("Exclude/include node")]));
            lines.push(Line::from(vec![key("x"), Span::raw("Exclude/include IP")]));
        }
        if app.viewer.stats_tab == StatsTab::DBTasks {
            lines.push(blank());
            lines.push(hdr!("Task Operations"));
            lines.push(blank());
            lines.push(Line::from(vec![key("d"), Span::raw("Cancel selected task")]));
            lines.push(Line::from(vec![key("X"), Span::raw("Cancel all cancellable tasks")]));
        }
        if app.viewer.stats_tab == StatsTab::DBShards {
            lines.push(blank());
            lines.push(hdr!("Shards Navigation"));
            lines.push(blank());
            lines.push(Line::from(vec![key("← / →"), Span::raw("Scroll nodes left / right")]));
            lines.push(Line::from(vec![key("Shift+← / Shift+→"), Span::raw("Fast scroll nodes")]));
            lines.push(Line::from(vec![key("Home"), Span::raw("Top + reset scroll")]));
            lines.push(Line::from(vec![key("End"), Span::raw("Jump to bottom")]));
            lines.push(Line::from(vec![key("m"), Span::raw("Cycle show mode (All/Not Started/...)")]));
        }
        ("Stats", lines)
    } else if app.active_tab == Tab::Files {
        ("Files", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
            Line::from(vec![key("Home"), Span::raw("First page")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/ / E"), Span::raw("Filter by name")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("c"), Span::raw("Columns & layouts")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
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
            Line::from(vec![key("q"), Span::raw("Quit")]),
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
        ])
    } else if app.viewer.show_column_editor {
        ("Column Editor", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Space / Enter"), Span::raw("Toggle field on/off")]),
            Line::from(vec![key("/"), Span::raw("Filter fields")]),
            Line::from(vec![key("m"), Span::raw("Reorder mode (↑/↓ to move)")]),
            Line::from(vec![key("a"), Span::raw("Apply changes")]),
            Line::from(vec![key("d"), Span::raw("Reset to defaults")]),
        ])
    } else if app.viewer.show_layout_popup {
        ("Layouts", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate layouts")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select / save / load layout")]),
            Line::from(vec![key("/"), Span::raw("Filter layouts")]),
            Line::from(vec![key("x / Delete"), Span::raw("Delete selected layout")]),
        ])
    } else if app.viewer.show_view_popup {
        ("Views", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate views")]),
            Line::from(vec![key("Esc"), Span::raw("Close (or clear filter)")]),
            Line::from(vec![key("q"), Span::raw("Close")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select view / save new view")]),
            Line::from(vec![key("/"), Span::raw("Filter views")]),
            Line::from(vec![key("x"), Span::raw("Delete selected view")]),
            Line::from(vec![key("Tab"), Span::raw("Toggle save columns (in save dialog)")]),
        ])
    } else if app.app_mode == AppMode::Cont3xt && app.active_tab == Tab::C3Stats {
        ("Cont3xt Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("1 / 2"), Span::raw("Switch sub-tab (Integrations / iTypes)")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/"), Span::raw("Filter by name")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh stats")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else if app.app_mode == AppMode::Cont3xt && app.active_tab == Tab::History {
        ("Cont3xt History", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Re-run search")]),
            Line::from(vec![key("d"), Span::raw("Delete history entry")]),
            Line::from(vec![key("/"), Span::raw("Filter by indicator/iType/tags")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh history")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else if app.app_mode == AppMode::Cont3xt && app.active_tab == Tab::Settings {
        use crate::app::C3SettingsTab;
        let mut lines = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("1 / 2 / 3 / 4"), Span::raw("Switch sub-tab")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate items")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
        ];
        if app.parliament.saved_client.is_some() {
            lines.push(Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")]));
        }
        lines.extend_from_slice(&[
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
        ]);
        match app.cont3xt.settings_tab {
            C3SettingsTab::Views => {
                lines.extend_from_slice(&[
                    Line::from(vec![key("n"), Span::raw("New view")]),
                    Line::from(vec![key("Enter / e"), Span::raw("Edit view")]),
                    Line::from(vec![key("d / x"), Span::raw("Delete view")]),
                    Line::from(vec![key("Ctrl+S"), Span::raw("Save view editor")]),
                    Line::from(vec![key("B"), Span::raw("Backup all views to file")]),
                    Line::from(vec![key("s / S"), Span::raw("Sort column / direction")]),
                    Line::from(vec![key("/"), Span::raw("Filter")]),
                    Line::from(vec![key("r"), Span::raw("Refresh")]),
                ]);
            }
            C3SettingsTab::Integrations => {
                lines.extend_from_slice(&[
                    Line::from(vec![key("Enter / e"), Span::raw("Edit integration config")]),
                    Line::from(vec![key("d"), Span::raw("Toggle disabled")]),
                    Line::from(vec![key("p"), Span::raw("Toggle password visibility")]),
                    Line::from(vec![key("Ctrl+S"), Span::raw("Save settings")]),
                    Line::from(vec![key("B"), Span::raw("Backup all integrations to file")]),
                    Line::from(vec![key("s / S"), Span::raw("Sort column / direction")]),
                    Line::from(vec![key("/"), Span::raw("Filter")]),
                    Line::from(vec![key("r"), Span::raw("Refresh")]),
                ]);
            }
            C3SettingsTab::LinkGroups => {
                use crate::app::C3LinkGroupLevel;
                match app.cont3xt.lg_level {
                    C3LinkGroupLevel::GroupList => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("Enter"), Span::raw("Edit links in group")]),
                            Line::from(vec![key("e"), Span::raw("Edit group name/roles")]),
                            Line::from(vec![key("n"), Span::raw("New group")]),
                            Line::from(vec![key("d / x"), Span::raw("Delete group")]),
                            Line::from(vec![key("B"), Span::raw("Backup all groups to file")]),
                            Line::from(vec![key("s / S"), Span::raw("Sort column / direction")]),
                            Line::from(vec![key("/"), Span::raw("Filter")]),
                            Line::from(vec![key("r"), Span::raw("Refresh")]),
                        ]);
                    }
                    C3LinkGroupLevel::GroupEditor => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("↑ / ↓"), Span::raw("Navigate fields")]),
                            Line::from(vec![key("Enter"), Span::raw("Edit roles (on role field)")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Save group")]),
                            Line::from(vec![key("Esc"), Span::raw("Cancel / back to list")]),
                        ]);
                    }
                    C3LinkGroupLevel::LinkList => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("Enter"), Span::raw("Edit link")]),
                            Line::from(vec![key("n / a"), Span::raw("New link / separator (after current)")]),
                            Line::from(vec![key("N / A"), Span::raw("New link / separator (at end)")]),
                            Line::from(vec![key("d / x"), Span::raw("Delete link")]),
                            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Reorder link")]),
                            Line::from(vec![key("/"), Span::raw("Filter links (disables add/reorder)")]),
                            Line::from(vec![key("B"), Span::raw("Backup this group to file")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Save group to server")]),
                            Line::from(vec![key("Esc"), Span::raw("Clear filter / back to group list")]),
                        ]);
                    }
                    C3LinkGroupLevel::LinkEditor => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("↑ / ↓"), Span::raw("Navigate fields")]),
                            Line::from(vec![key("Space"), Span::raw("Toggle indicator type")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Apply changes")]),
                            Line::from(vec![key("Esc"), Span::raw("Cancel / back to link list")]),
                        ]);
                    }
                }
            }
            C3SettingsTab::Overviews => {
                use crate::app::C3OverviewLevel;
                match app.cont3xt.ov_level {
                    C3OverviewLevel::List => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("Enter"), Span::raw("Open field list")]),
                            Line::from(vec![key("e"), Span::raw("Edit overview info")]),
                            Line::from(vec![key("n"), Span::raw("New overview")]),
                            Line::from(vec![key("d / x"), Span::raw("Delete overview")]),
                            Line::from(vec![key("B"), Span::raw("Backup all overviews to file")]),
                            Line::from(vec![key("s / S"), Span::raw("Sort column / direction")]),
                            Line::from(vec![key("/"), Span::raw("Filter")]),
                            Line::from(vec![key("r"), Span::raw("Refresh")]),
                        ]);
                    }
                    C3OverviewLevel::Editor => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("↑ / ↓"), Span::raw("Navigate fields")]),
                            Line::from(vec![key("Enter"), Span::raw("Edit roles / open field list")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Save overview")]),
                            Line::from(vec![key("Esc"), Span::raw("Cancel / back to list")]),
                        ]);
                    }
                    C3OverviewLevel::FieldList => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("Enter / e"), Span::raw("Edit field")]),
                            Line::from(vec![key("n / a"), Span::raw("New field (after current)")]),
                            Line::from(vec![key("N / A"), Span::raw("New field (at end)")]),
                            Line::from(vec![key("d / x"), Span::raw("Delete field")]),
                            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Reorder field")]),
                            Line::from(vec![key("/"), Span::raw("Filter fields")]),
                            Line::from(vec![key("B"), Span::raw("Backup this overview to file")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Save overview to server")]),
                            Line::from(vec![key("Esc"), Span::raw("Clear filter / back to list")]),
                        ]);
                    }
                    C3OverviewLevel::FieldEditor => {
                        lines.extend_from_slice(&[
                            Line::from(vec![key("↑ / ↓"), Span::raw("Navigate fields")]),
                            Line::from(vec![key("Enter"), Span::raw("Open selector (Integration/Field)")]),
                            Line::from(vec![key("Ctrl+S"), Span::raw("Apply changes")]),
                            Line::from(vec![key("Esc"), Span::raw("Cancel / back to field list")]),
                        ]);
                    }
                }
            }
        }
        lines.push(Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]));
        ("Cont3xt Settings", lines)
    } else if app.cont3xt.show_overview_popup {
        ("Select Overview", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate overviews")]),
            Line::from(vec![key("Esc / q / o"), Span::raw("Close popup")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Select overview (session only)")]),
            Line::from(vec![key("d"), Span::raw("Set as default (saves to server)")]),
            Line::from(vec![key("/"), Span::raw("Filter overviews")]),
            Line::from(vec![key("r"), Span::raw("Refresh overviews")]),
        ])
    } else if app.cont3xt.show_link_popup {
        ("Link Groups", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate links")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down (10)")]),
            Line::from(vec![key("Esc / q / l"), Span::raw("Close popup")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open link in browser")]),
            Line::from(vec![key("/"), Span::raw("Filter links by name")]),
            Line::from(vec![key("r"), Span::raw("Refresh link groups")]),
        ])
    } else if app.app_mode == AppMode::Cont3xt && app.active_tab == Tab::Search && app.cont3xt.focus == Cont3xtFocus::Detail {
        ("Cont3xt Detail", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("↑ / ↓"), Span::raw("Scroll detail vertically")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("PgUp / PgDn"), Span::raw("Page up / down")]),
            Line::from(vec![key("← / →"), Span::raw("Scroll detail left / right")]),
            Line::from(vec![key("Shift+← / Shift+→"), Span::raw("Fast scroll left / right")]),
            Line::from(vec![key("Home"), Span::raw("Jump to top, reset scroll")]),
            Line::from(vec![key("End"), Span::raw("Jump to bottom")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter / Esc"), Span::raw("Return to results panel")]),
            Line::from(vec![key("R"), Span::raw("Toggle raw JSON / card view (debug for overview)")]),
            Line::from(vec![key("/"), Span::raw("Filter detail fields")]),
            Line::from(vec![key("E"), Span::raw("Edit search indicator")]),
            Line::from(vec![key("o"), Span::raw("Select overview (on indicator)")]),
            Line::from(vec![key("C"), Span::raw("Show card/overview definition")]),
            Line::from(vec![key("i"), Span::raw("Integrations popup")]),
            Line::from(vec![key("I (Shift+i)"), Span::raw("Views popup")]),
            Line::from(vec![key("l"), Span::raw("Link groups popup")]),
            Line::from(vec![key("J"), Span::raw("Save all results as JSON")]),
            Line::from(vec![key("t"), Span::raw("Edit search tags")]),
            Line::from(vec![key("d"), Span::raw("Edit date range for links")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else if app.active_tab == Tab::Users {
        let mut ht = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate users")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
        ];
        if app.parliament.saved_client.is_some() {
            ht.push(Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")]));
        }
        ht.push(Line::from(vec![key("q"), Span::raw("Quit")]));
        ht.extend(vec![
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Edit user")]),
            Line::from(vec![key("n"), Span::raw("New user")]),
            Line::from(vec![key("N"), Span::raw("New role")]),
            Line::from(vec![key("d / x"), Span::raw("Delete user/role")]),
            Line::from(vec![key("/ / E"), Span::raw("Filter users (live)")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
            blank(),
            hdr!("Editor"),
            blank(),
            Line::from(vec![key("Tab / ↑ / ↓"), Span::raw("Navigate fields")]),
            Line::from(vec![key("Space / Enter"), Span::raw("Toggle bool / open roles")]),
            Line::from(vec![key("Ctrl+S"), Span::raw("Save changes")]),
            Line::from(vec![key("Esc"), Span::raw("Cancel editing")]),
        ]);
        ("Users", ht)
    } else if app.app_mode == AppMode::Cont3xt {
        (if app.active_tab == Tab::Search { "Cont3xt Results" } else { "Cont3xt" }, vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate results list")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Next / prev top-level indicator")]),
            Line::from(vec![key("Ctrl+j / Ctrl+k"), Span::raw("Next / prev same integration")]),
            Line::from(vec![key("← / →"), Span::raw("Jump to top / bottom")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open detail panel")]),
            Line::from(vec![key("/ or E"), Span::raw("Edit search indicator")]),
            Line::from(vec![key("R"), Span::raw("Toggle raw JSON / card view")]),
            Line::from(vec![key("i"), Span::raw("Integrations popup (v:views inside)")]),
            Line::from(vec![key("I (Shift+i)"), Span::raw("Views popup")]),
            Line::from(vec![key("o"), Span::raw("Select overview (on indicator header)")]),
            Line::from(vec![key("l"), Span::raw("Link groups popup")]),
            Line::from(vec![key("J"), Span::raw("Save all results as JSON")]),
            Line::from(vec![key("r"), Span::raw("Re-run search")]),
            Line::from(vec![key("Ctrl+r"), Span::raw("Re-run search (no cache)")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else if app.app_mode == AppMode::Parliament && app.active_tab == Tab::Dashboard {
        ("Parliament Dashboard", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate clusters")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("Enter"), Span::raw("Open cluster in Viewer mode")]),
            Line::from(vec![key("i"), Span::raw("Cluster detail overlay")]),
            Line::from(vec![key("c"), Span::raw("Open Cont3xt (if configured)")]),
            Line::from(vec![key("w"), Span::raw("Open WISE (if configured)")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ])
    } else if app.app_mode == AppMode::Parliament && app.active_tab == Tab::Issues {
        ("Parliament Issues", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate issues")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("/ / E"), Span::raw("Filter issues")]),
            Line::from(vec![key("s"), Span::raw("Next sort column")]),
            Line::from(vec![key("S"), Span::raw("Toggle sort direction")]),
            Line::from(vec![key("r"), Span::raw("Refresh issues")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ])
    } else if app.app_mode == AppMode::Parliament && app.active_tab == Tab::Settings {
        use crate::app::PlSettingsTab;
        let mut lines = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("1 / 2 / 3"), Span::raw("Switch sub-tab (Groups / General / Notifiers)")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate items")]),
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
        ];
        match app.parliament.settings_tab {
            PlSettingsTab::Groups => {
                lines.extend(vec![
                    hdr!("Groups"),
                    blank(),
                    Line::from(vec![key("e / Enter"), Span::raw("Edit selected group/cluster")]),
                    Line::from(vec![key("n"), Span::raw("New group")]),
                    Line::from(vec![key("a"), Span::raw("Add cluster to group")]),
                    Line::from(vec![key("d / x"), Span::raw("Delete selected")]),
                    Line::from(vec![key("B"), Span::raw("Backup groups to JSON file")]),
                    Line::from(vec![key("r"), Span::raw("Refresh")]),
                    Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
                ]);
            }
            PlSettingsTab::General => {
                lines.extend(vec![
                    hdr!("General Settings"),
                    blank(),
                    Line::from(vec![key("Enter"), Span::raw("Edit selected field")]),
                    Line::from(vec![key("Ctrl+S"), Span::raw("Save all settings")]),
                    Line::from(vec![key("r"), Span::raw("Refresh")]),
                    Line::from(vec![key("D"), Span::raw("HTTP debug log")]),
                ]);
            }
            PlSettingsTab::Notifiers => {
                lines.extend(vec![
                    hdr!("Notifiers"),
                    blank(),
                    Line::from(vec![Span::styled("Under construction", Style::default().fg(Color::Yellow))]),
                ]);
            }
        }
        ("Parliament Settings", lines)
    } else if app.app_mode == AppMode::Wise && app.active_tab == Tab::WsStats {
        ("WISE Stats", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate rows")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("1 / 2"), Span::raw("Sources / Types sub-tab")]),
            Line::from(vec![key("/ / E"), Span::raw("Filter stats")]),
            Line::from(vec![key("r"), Span::raw("Refresh")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else if app.app_mode == AppMode::Wise && app.active_tab == Tab::WsQuery {
        ("WISE Query", vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate results")]),
            Line::from(vec![key("Home / End"), Span::raw("Jump to top / bottom")]),
        ].into_iter().chain(
            if app.parliament.saved_client.is_some() {
                vec![Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")])]
            } else { vec![] }
        ).chain(vec![
            Line::from(vec![key("q"), Span::raw("Quit")]),
            blank(),
            hdr!("Actions"),
            blank(),
            Line::from(vec![key("s"), Span::raw("Cycle source")]),
            Line::from(vec![key("t"), Span::raw("Cycle type")]),
            Line::from(vec![key("/ / E"), Span::raw("Edit query value")]),
            Line::from(vec![key("Enter"), Span::raw("Run query")]),
            Line::from(vec![key("D"), Span::raw("HTTP debug log (Enter:expand)")]),
        ]).collect())
    } else {
        let mut ht = vec![
            hdr!("Navigation"),
            blank(),
            Line::from(vec![key("Tab / Shift+Tab"), Span::raw("Switch tabs")]),
            Line::from(vec![key("j / k / ↑ / ↓"), Span::raw("Navigate sessions")]),
            Line::from(vec![key("← / →"), Span::raw("Previous / next page")]),
            Line::from(vec![key("Shift+← / Shift+→"), Span::raw("First / last page")]),
            Line::from(vec![key("Shift+↑ / Shift+↓"), Span::raw("Page up / down")]),
        ];
        if app.parliament.saved_client.is_some() {
            ht.push(Line::from(vec![key("Ctrl+p"), Span::raw("Return to Parliament")]));
        }
        ht.push(Line::from(vec![key("q"), Span::raw("Quit")]));
        ht.extend(vec![
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
        ]);
        ("Sessions", ht)
    };

    let mut lines = help_text;
    lines.push(blank());
    lines.push(Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))));

    let popup_width = 64u16.min(area.width.saturating_sub(4));
    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let help = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {title} Help ")),
        );
    f.render_widget(help, popup_area);
}
