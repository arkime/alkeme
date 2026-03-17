use super::*;
use super::cont3xt_settings::{c3_draw_settings, c3_draw_view_editor, c3_draw_role_popup, c3_draw_int_editor, c3_draw_group_role_popup, c3_draw_ov_fe_selector_popup};
use crate::api::{CardField, Cont3xtCard, Cont3xtOverview};
use crate::app::{C3TreeItem, C3SettingsTab, C3GroupEditorField};

pub(super) fn draw_cont3xt(f: &mut Frame, app: &mut App) {
    let any_popup = app.has_popup_open();
    let area = f.area();

    // Double-buffer: if a popup is open, restore cached background instead of re-rendering
    if any_popup {
        if let Some(ref cache) = app.popup_bg_cache {
            if cache.area == area {
                // Restore cached background into the frame buffer
                let buf = f.buffer_mut();
                let src = cache.content();
                let dst = buf.content.as_mut_slice();
                dst[..src.len()].clone_from_slice(src);
            } else {
                // Terminal resized — invalidate cache and re-render
                app.popup_bg_cache = None;
                draw_cont3xt_background(f, app);
                app.popup_bg_cache = Some(f.buffer_mut().clone());
            }
        } else {
            // First frame with popup — render background and cache it
            draw_cont3xt_background(f, app);
            app.popup_bg_cache = Some(f.buffer_mut().clone());
        }
    } else {
        // No popup — render normally, clear cache
        app.popup_bg_cache = None;
        draw_cont3xt_background(f, app);
    }

    // Render popup overlays on top
    if app.cont3xt.show_card_popup {
        draw_card_popup(f, app, area);
    }
    if app.cont3xt.show_overview_popup {
        draw_overview_popup(f, app, area);
    }
    if app.cont3xt.show_link_popup {
        draw_link_popup(f, app, area);
    }
    if app.cont3xt.show_integration_popup {
        draw_integration_popup(f, app, area);
    }
    if app.cont3xt.save_json_prompt.is_some() {
        draw_save_json_prompt(f, app, area);
    }
    if app.cont3xt.show_tags_popup {
        draw_tags_popup(f, app, area);
    }
    if app.cont3xt.show_date_popup {
        draw_date_popup(f, app, area);
    }
    if app.cont3xt.view_editor_open {
        c3_draw_view_editor(f, app, area);
    }
    if app.cont3xt.role_popup_open {
        if app.active_tab == Tab::Settings && app.cont3xt.settings_tab == C3SettingsTab::LinkGroups {
            let active = app.cont3xt.lg_group_editor_field;
            let roles = if active == C3GroupEditorField::ViewRoles {
                &app.cont3xt.lg_group_editor_view_roles
            } else {
                &app.cont3xt.lg_group_editor_edit_roles
            };
            c3_draw_group_role_popup(f, app, area, roles);
        } else if app.active_tab == Tab::Settings && app.cont3xt.settings_tab == C3SettingsTab::Overviews {
            let roles = if app.cont3xt.role_popup_for_edit {
                &app.cont3xt.ov_editor_edit_roles
            } else {
                &app.cont3xt.ov_editor_view_roles
            };
            c3_draw_group_role_popup(f, app, area, roles);
        } else {
            c3_draw_role_popup(f, app, area);
        }
    }
    if app.cont3xt.int_editor_open {
        c3_draw_int_editor(f, app, area);
    }
    if app.cont3xt.ov_fe_popup_open {
        c3_draw_ov_fe_selector_popup(f, app, area);
    }
}

fn draw_cont3xt_background(f: &mut Frame, app: &mut App) {
    let status_h = status_bar_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(3), // search bar
            Constraint::Min(0),   // content
            Constraint::Length(status_h), // status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Search => {
            draw_cont3xt_search_bar(f, app, chunks[1]);
            if app.cont3xt.searching {
                let gauge_area = Rect::new(chunks[2].x, chunks[2].y, chunks[2].width, 1);
                let results_area = Rect::new(chunks[2].x, chunks[2].y + 1, chunks[2].width, chunks[2].height.saturating_sub(1));
                let sent = app.cont3xt.search_sent;
                let total = app.cont3xt.search_total;
                let ratio = if total > 0 { (sent as f64 / total as f64).min(1.0) } else { 0.0 };
                let label = if total > 0 {
                    format!(" {}/{} ", sent, total)
                } else {
                    " Searching... ".to_string()
                };
                let gauge = LineGauge::default()
                    .filled_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                    .unfilled_style(Style::default().fg(Color::DarkGray))
                    .label(Span::styled(label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)))
                    .line_set(symbols::line::THICK)
                    .ratio(ratio);
                f.render_widget(gauge, gauge_area);
                draw_cont3xt_results(f, app, results_area);
            } else {
                draw_cont3xt_results(f, app, chunks[2]);
            }
        }
        Tab::C3Stats => {
            let stats_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, chunks[1].height + chunks[2].height);
            c3_draw_stats(f, app, stats_area);
        }
        Tab::History => {
            let history_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, chunks[1].height + chunks[2].height);
            c3_draw_history(f, app, history_area);
        }
        Tab::Settings => {
            let settings_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, chunks[1].height + chunks[2].height);
            c3_draw_settings(f, app, settings_area);
        }
        Tab::Users => {
            let users_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, chunks[1].height + chunks[2].height);
            super::users::draw_users_tab(f, app, users_area);
        }
        _ => {}
    }

    draw_status_bar(f, app, chunks[3]);
}

fn draw_cont3xt_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let expr_display = if app.input_mode == InputMode::Expression {
        &app.expression_edit
    } else {
        &app.expression
    };

    let integrations_label = if let Some(ref name) = app.cont3xt.active_view_name {
        format!("[view: {name}] ")
    } else if app.cont3xt.disabled_integrations.is_empty() {
        "[all] ".to_string()
    } else {
        "[custom] ".to_string()
    };

    let tags_label = if app.cont3xt.tags.is_empty() {
        String::new()
    } else {
        format!("[tags: {}] ", app.cont3xt.tags.join(","))
    };

    let days = (app.cont3xt.stop_date - app.cont3xt.start_date).num_days();
    let date_label = format!("[{}: {}d] ", app.cont3xt.date_start_edit, days);

    let is_editing = app.input_mode == InputMode::Expression;
    let file_label = if let Some(ref path) = app.cont3xt.loaded_file {
        format!("[file: {path}] ")
    } else {
        String::new()
    };
    let title = format!(" Search (/) {integrations_label}{tags_label}{date_label}{file_label}");
    render_text_input(f, expr_display, app.expression_cursor, is_editing, &title, area);
}

fn draw_cont3xt_results(f: &mut Frame, app: &mut App, area: Rect) {
    if app.cont3xt.results.is_empty() && app.expression.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Results ");
        let placeholder = Paragraph::new("  Enter an indicator to search (IP, domain, hash, email, ...)")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(placeholder, area);
        return;
    }
    if app.cont3xt.results.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Results ");
        let text = if app.cont3xt.searching {
            "  Searching...".to_string()
        } else {
            format!("  No results for: {}", app.expression)
        };
        let content = Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(content, area);
        return;
    }

    // Split into left (integration list) and right (detail)
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),  // integration list
            Constraint::Min(0),     // detail pane
        ])
        .split(area);

    // Left pane: results tree grouped by indicator
    let results_focused = app.cont3xt.focus == Cont3xtFocus::Results;
    let results_border_style = if results_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(results_border_style)
        .title(format!(" Results ({}) ", app.cont3xt.results.len()));

    let inner = results_block.inner(horiz[0]);
    f.render_widget(results_block, horiz[0]);

    let visible_height = inner.height as usize;
    app.visible_rows = visible_height;

    // Build tree using parent-child indicator relationships
    // Group results by (itype, indicator)
    let mut indicator_results: std::collections::HashMap<(String, String), Vec<usize>> = std::collections::HashMap::new();
    let mut indicator_order: Vec<(String, String)> = Vec::new();

    // Start with init-ordered indicators as the canonical order
    for (itype, query) in &app.cont3xt.init_indicators {
        let key = (itype.clone(), query.clone());
        if !indicator_order.contains(&key) {
            indicator_order.push(key);
        }
    }

    for (idx, result) in app.cont3xt.results.iter().enumerate() {
        let key = (result.itype.clone(), result.indicator.clone());
        if !indicator_order.contains(&key) {
            indicator_order.push(key.clone());
        }
        indicator_results.entry(key).or_default().push(idx);
    }

    // Also ensure parent indicators exist in indicator_order even without results
    // This handles chains like URL -> DOMAIN -> IP where URL may have no direct results
    for ((_child_ind, _child_itype), parents) in &app.cont3xt.indicator_parents {
        for (parent_query, parent_itype) in parents {
            let parent_key = (parent_itype.clone(), parent_query.clone());
            if !indicator_order.contains(&parent_key) {
                indicator_order.push(parent_key);
            }
        }
    }

    // Build a tree: find root indicators (those with no parent or whose parent is not in our set)
    let mut children_of: std::collections::HashMap<(String, String), Vec<(String, String)>> = std::collections::HashMap::new();
    let mut has_parent: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for key in &indicator_order {
        // c3_indicator_parents is keyed as (indicator, itype), convert to match our (itype, indicator)
        let lookup_key = (key.1.clone(), key.0.clone());
        if let Some(parents) = app.cont3xt.indicator_parents.get(&lookup_key) {
            for parent in parents {
                // parent is (parent_query, parent_itype), convert to (itype, indicator)
                let parent_key = (parent.1.clone(), parent.0.clone());
                if indicator_results.contains_key(&parent_key) || indicator_order.contains(&parent_key) {
                    children_of.entry(parent_key).or_default().push(key.clone());
                    has_parent.insert(key.clone());
                }
            }
        }
    }

    // Recursive tree builder
    fn build_tree(
        key: &(String, String),
        depth: u16,
        children_of: &std::collections::HashMap<(String, String), Vec<(String, String)>>,
        indicator_results: &std::collections::HashMap<(String, String), Vec<usize>>,
        results: &[crate::api::Cont3xtResult],
        rows: &mut Vec<(u16, String, Option<usize>)>,
    ) {
        rows.push((depth, format!("{} {}", key.0.to_uppercase(), key.1), None));
        if let Some(indices) = indicator_results.get(key) {
            for &idx in indices {
                rows.push((depth + 2, results[idx].name.clone(), Some(idx)));
            }
        }
        if let Some(kids) = children_of.get(key) {
            for child in kids {
                build_tree(child, depth + 2, children_of, indicator_results, results, rows);
            }
        }
    }

    let mut display_rows: Vec<(u16, String, Option<usize>)> = Vec::new();
    // Only start from root indicators (not children)
    for key in &indicator_order {
        if !has_parent.contains(key) {
            build_tree(key, 0, &children_of, &indicator_results, &app.cont3xt.results, &mut display_rows);
        }
    }

    // Build tree_order and tree_roots from display_rows
    let mut tree_order: Vec<C3TreeItem> = Vec::new();
    let mut tree_roots: Vec<usize> = Vec::new();
    for (depth, label, idx) in &display_rows {
        if *depth == 0 && idx.is_none() {
            // Root indicator header — parse "ITYPE QUERY" back to (itype, query)
            tree_roots.push(tree_order.len());
            if let Some(space_pos) = label.find(' ') {
                let itype = label[..space_pos].to_lowercase();
                let query = label[space_pos + 1..].to_string();
                tree_order.push(C3TreeItem::Indicator(itype, query));
            } else {
                tree_order.push(C3TreeItem::Indicator(label.to_lowercase(), String::new()));
            }
        } else if idx.is_none() {
            // Child indicator header
            if let Some(space_pos) = label.find(' ') {
                let itype = label[..space_pos].to_lowercase();
                let query = label[space_pos + 1..].to_string();
                tree_order.push(C3TreeItem::Indicator(itype, query));
            } else {
                tree_order.push(C3TreeItem::Indicator(label.to_lowercase(), String::new()));
            }
        } else if let Some(result_idx) = idx {
            tree_order.push(C3TreeItem::Result(*result_idx));
        }
    }
    app.cont3xt.tree_order = tree_order;
    app.cont3xt.tree_roots = tree_roots;

    // tree_order now has 1:1 mapping with display_rows
    let selected_display_row = app.cont3xt.selected.min(display_rows.len().saturating_sub(1));

    // Scroll to keep selected visible
    let scroll_offset = if selected_display_row >= visible_height {
        selected_display_row - visible_height + 1
    } else {
        0
    };

    for (row_i, (indent, label, result_idx)) in display_rows.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let y = inner.y + (row_i - scroll_offset) as u16;
        if y >= inner.y + inner.height { break; }

        let is_header = result_idx.is_none();
        let is_selected = row_i == selected_display_row;

        let style = if is_selected && results_focused {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if is_header {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = " ".repeat(*indent as usize);
        let w = inner.width as usize;

        // For result rows, extract _cont3xt.count and severity
        if let Some(idx) = result_idx
            && let Some(result) = app.cont3xt.results.get(*idx) {
                let count_val = result.data.get("_cont3xt")
                    .and_then(|c| c.get("count"))
                    .and_then(|v| v.as_u64());
                let severity = result.data.get("_cont3xt")
                    .and_then(|c| c.get("severity"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if let Some(count) = count_val {
                    let count_str = format!(" {count}");
                    let name_part = format!("{prefix}{label}");
                    let pad_len = w.saturating_sub(name_part.len() + count_str.len());
                    let padded_name = format!("{name_part}{}", " ".repeat(pad_len));
                    let count_style = if severity == "high" {
                        if is_selected {
                            Style::default().fg(Color::Red).bg(if results_focused { Color::Cyan } else { Color::Yellow })
                        } else {
                            Style::default().fg(Color::Red)
                        }
                    } else {
                        style
                    };
                    let line = Line::from(vec![
                        Span::styled(padded_name, style),
                        Span::styled(count_str, count_style),
                    ]);
                    f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                    continue;
                }
            }

        let full_label = format!("{prefix}{label}");
        let truncated = if full_label.len() > w {
            format!("{}…", &full_label[..w - 1])
        } else {
            format!("{:<width$}", full_label, width = w)
        };

        let span = Span::styled(truncated, style);
        f.render_widget(Paragraph::new(span), Rect::new(inner.x, y, inner.width, 1));
    }

    // Right pane: detail for selected integration
    let detail_focused = app.cont3xt.focus == Cont3xtFocus::Detail;
    let detail_border_style = if detail_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_tree_item = app.cont3xt.tree_order.get(app.cont3xt.selected).cloned();

    // Determine detail content based on selected tree item type
    let (detail_title, detail_lines) = match &selected_tree_item {
        Some(C3TreeItem::Result(idx)) => {
            if let Some(result) = app.cont3xt.results.get(*idx) {
                let card = if !app.cont3xt.raw_view {
                    app.cont3xt.integrations.iter()
                        .find(|i| i.name == result.name)
                        .and_then(|i| i.card.as_ref())
                } else {
                    None
                };
                let view_label = if app.cont3xt.raw_view { " [RAW] " } else { "" };
                let title = format!(" {} — {} {view_label}", result.name, result.indicator);
                let lines = if let Some(card) = card {
                    render_card_lines(card, &result.data, &result.indicator)
                } else {
                    flatten_json_to_lines(&result.data, "", 0)
                };
                (title, lines)
            } else {
                (" Detail ".to_string(), Vec::new())
            }
        }
        Some(C3TreeItem::Indicator(itype, query)) => {
            let itype_lower = itype.to_lowercase();
            // Use user-selected overview if set, otherwise prefer default, then any
            let overview = if let Some(selected_id) = app.cont3xt.selected_overviews.get(&itype_lower) {
                app.cont3xt.overviews.iter().find(|o| o.id == *selected_id)
            } else {
                None
            }.or_else(|| app.cont3xt.overviews.iter().find(|o| o.itype.to_lowercase() == itype_lower && o.is_default))
             .or_else(|| app.cont3xt.overviews.iter().find(|o| o.itype.to_lowercase() == itype_lower));
            if let Some(overview) = overview {
                let title_str = overview.title.replace("%{query}", query);
                let raw_label = if app.cont3xt.raw_view { " [DEBUG] " } else { "" };
                let title = format!(" {}{raw_label}", title_str);
                let lines = render_overview_lines(overview, itype, query, &app.cont3xt.results, &app.cont3xt.integrations, app.cont3xt.raw_view);
                (title, lines)
            } else {
                let title = format!(" {} — {} ", itype, query);
                (title, vec![JsonLine::KeyValue("No overview available".to_string(), format!("for type '{}'", itype))])
            }
        }
        None => (String::new(), Vec::new()),
    };

    if selected_tree_item.is_some() {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(detail_border_style)
            .title(detail_title.clone());

        let detail_inner = detail_block.inner(horiz[1]);
        f.render_widget(detail_block, horiz[1]);

        let mut lines = detail_lines;
        align_table_columns(&mut lines);

        // Apply detail filter
        if !app.cont3xt.detail_filter.is_empty() {
            let filter_lower = app.cont3xt.detail_filter.to_lowercase();
            let len = lines.len();

            // Mark data lines that match the filter
            let mut keep: Vec<bool> = lines.iter().map(|line| {
                match line {
                    JsonLine::KeyValue(k, v) => format!("{k}: {v}").to_lowercase().contains(&filter_lower),
                    JsonLine::ArrayValue(v) => v.to_lowercase().contains(&filter_lower),
                    JsonLine::TableRow(cells, _) => cells.join(" ").to_lowercase().contains(&filter_lower),
                    _ => false,
                }
            }).collect();

            // Bottom-up: keep TableHeader only if a following TableRow (before non-TableRow) is kept
            for i in (0..len).rev() {
                if matches!(&lines[i], JsonLine::TableHeader(_, _)) {
                    keep[i] = lines[i+1..].iter().zip(keep[i+1..].iter()).any(|(next, &k)| {
                        if !matches!(next, JsonLine::TableRow(_, _)) { return false; }
                        k
                    });
                }
            }

            // Bottom-up: keep Header only if any kept line follows before the next Header
            for i in (0..len).rev() {
                if matches!(&lines[i], JsonLine::Header(_, _)) {
                    keep[i] = lines[i+1..].iter().zip(keep[i+1..].iter()).any(|(next, &k)| {
                        if matches!(next, JsonLine::Header(_, _)) { return false; }
                        k
                    });
                }
            }

            let mut filtered = Vec::new();
            for (i, line) in lines.into_iter().enumerate() {
                if keep[i] { filtered.push(line); }
            }
            lines = filtered;
        }

        // Show filter bar if filtering
        let filter_height = if app.input_mode == InputMode::DetailFilter || !app.cont3xt.detail_filter.is_empty() { 1u16 } else { 0 };
        let content_height = detail_inner.height.saturating_sub(filter_height);

        let total_lines = lines.len();

        // Clamp scroll
        let max_scroll = total_lines.saturating_sub(content_height as usize);
        let scroll = (app.cont3xt.detail_scroll as usize).min(max_scroll);
        app.cont3xt.detail_scroll = scroll as u16;

        let max_width = detail_inner.width as usize;
        let hscroll = app.cont3xt.detail_hscroll as usize;
        let c3_filter_lower = app.cont3xt.detail_filter.to_lowercase();
        let rendered_lines: Vec<Line> = lines.iter().map(|line| {
            let spans = match line {
                JsonLine::KeyValue(key, value) => {
                    let prefix = format!(" {}: ", key);
                    let remaining = max_width.saturating_sub(prefix.len());
                    let truncated: String = value.chars().take(remaining).collect();
                    let key_style = Style::default().fg(Color::Yellow);
                    let val_style = Style::default().fg(Color::White);
                    let mut s = highlight_filter_spans(&prefix, &c3_filter_lower, key_style);
                    s.extend(highlight_filter_spans(&truncated, &c3_filter_lower, val_style));
                    s
                }
                JsonLine::Header(key, is_array) => {
                    let suffix = if *is_array { " [" } else { " {" };
                    vec![Span::styled(format!(" {}{}", key, suffix), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]
                }
                JsonLine::Close(bracket) => {
                    vec![Span::styled(format!(" {bracket}"), Style::default().fg(Color::DarkGray))]
                }
                JsonLine::ArrayValue(value) => {
                    let remaining = max_width.saturating_sub(5);
                    let truncated: String = value.chars().take(remaining).collect();
                    let mut s = vec![Span::styled("   • ", Style::default().fg(Color::DarkGray))];
                    s.extend(highlight_filter_spans(&truncated, &c3_filter_lower, Style::default().fg(Color::White)));
                    s
                }
                JsonLine::TableRow(cells, widths) => {
                    let row_str = format_table_cells(cells, widths, " │ ");
                    let visible: String = row_str.chars().skip(hscroll).take(max_width.saturating_sub(2)).collect();
                    let mut s = vec![Span::raw("  ")];
                    s.extend(highlight_filter_spans(&visible, &c3_filter_lower, Style::default().fg(Color::White)));
                    s
                }
                JsonLine::TableHeader(cells, widths) => {
                    let row_str = format_table_cells(cells, widths, " │ ");
                    let visible: String = row_str.chars().skip(hscroll).take(max_width.saturating_sub(2)).collect();
                    vec![
                        Span::raw("  "),
                        Span::styled(visible, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    ]
                }
            };
            Line::from(spans)
        }).collect();

        let detail_paragraph = Paragraph::new(rendered_lines)
            .scroll((scroll as u16, 0));
        f.render_widget(detail_paragraph, Rect::new(detail_inner.x, detail_inner.y, detail_inner.width, content_height));

        // Filter bar at bottom of detail pane
        if filter_height > 0 {
            let filter_y = detail_inner.y + content_height;
            let filter_text = format!(" /{}", app.cont3xt.detail_filter);
            let filter_style = if app.input_mode == InputMode::DetailFilter {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            f.render_widget(
                Paragraph::new(Span::styled(&filter_text, filter_style)),
                Rect::new(detail_inner.x, filter_y, detail_inner.width, 1),
            );
            if app.input_mode == InputMode::DetailFilter {
                f.set_cursor_position((detail_inner.x + filter_text.len() as u16, filter_y));
            }
        }

        // Scrollbar indicator + raw toggle hint
        let hint = if detail_focused { " R:toggle raw " } else { "" };
        if total_lines > content_height as usize {
            let pct = if max_scroll > 0 { scroll * 100 / max_scroll } else { 0 };
            let indicator = format!(" {}/{} ({}%){hint}", scroll + 1, total_lines, pct);
            let x = horiz[1].x + horiz[1].width.saturating_sub(indicator.len() as u16 + 1);
            f.render_widget(
                Paragraph::new(Span::styled(&indicator, Style::default().fg(Color::DarkGray))),
                Rect::new(x, horiz[1].y, indicator.len() as u16, 1),
            );
        } else if !hint.is_empty() {
            let x = horiz[1].x + horiz[1].width.saturating_sub(hint.len() as u16 + 1);
            f.render_widget(
                Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
                Rect::new(x, horiz[1].y, hint.len() as u16, 1),
            );
        }
    } else {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(detail_border_style)
            .title(" Detail ");
        f.render_widget(detail_block, horiz[1]);
    }
}

enum JsonLine {
    KeyValue(String, String),
    Header(String, bool),    // name, is_array
    Close(String),
    ArrayValue(String),
    TableHeader(Vec<String>, Vec<usize>),  // cells, column widths
    TableRow(Vec<String>, Vec<usize>),     // cells, column widths
}

/// Render overview lines by pulling data from multiple integration results
fn render_overview_lines(
    overview: &Cont3xtOverview,
    _itype: &str,
    query: &str,
    results: &[crate::api::Cont3xtResult],
    integrations: &[crate::api::Cont3xtIntegration],
    debug: bool,
) -> Vec<JsonLine> {
    let mut lines = Vec::new();

    for field in &overview.fields {
        if field.field_type == "linked" {
            // Find the integration result for this indicator with matching integration name
            let result = results.iter().find(|r| r.name == field.from && r.indicator == query);
            let display_name = field.alias.as_deref().unwrap_or(&field.field);

            if let Some(result) = result {
                // Find the card definition for this integration
                let card = integrations.iter()
                    .find(|i| i.name == field.from)
                    .and_then(|i| i.card.as_ref());

                if let Some(card) = card {
                    // Find the card field with matching label
                    let card_field = card.fields.iter().find(|cf| cf.label == field.field);
                    if let Some(cf) = card_field {
                        let val = get_by_path(&result.data, &cf.field);
                        if val.is_some() {
                            render_overview_card_field(&mut lines, display_name, cf, val, &result.data);
                        } else if debug {
                            lines.push(JsonLine::KeyValue(
                                display_name.to_string(),
                                format!("(no data at path '{}' in {})", cf.field, field.from),
                            ));
                        }
                    } else if debug {
                        let card_labels: Vec<&str> = card.fields.iter().map(|f| f.label.as_str()).collect();
                        lines.push(JsonLine::KeyValue(
                            display_name.to_string(),
                            format!("(card field '{}' not found in {}, have: [{}])", field.field, field.from, card_labels.join(", ")),
                        ));
                    }
                } else {
                    // No card — try direct data access
                    let val = get_by_path(&result.data, &field.field);
                    if let Some(v) = val {
                        lines.push(JsonLine::KeyValue(display_name.to_string(), format_json_value(v)));
                    } else if debug {
                        lines.push(JsonLine::KeyValue(
                            display_name.to_string(),
                            format!("(no card for integration '{}')", field.from),
                        ));
                    }
                }
            } else if debug {
                lines.push(JsonLine::KeyValue(
                    display_name.to_string(),
                    format!("(no result from '{}')", field.from),
                ));
            }
        }
    }

    lines
}

/// Render a single overview field using its card field definition
fn render_overview_card_field(lines: &mut Vec<JsonLine>, display_name: &str, cf: &CardField, val: Option<&serde_json::Value>, _data: &serde_json::Value) {
    match cf.field_type.as_str() {
        "table" => {
            lines.push(JsonLine::Header(display_name.to_string(), true));
            if let Some(arr) = val.and_then(|v| v.as_array()) {
                if !cf.fields.is_empty() {
                    let headers: Vec<String> = cf.fields.iter().map(|f| f.label.clone()).collect();
                    lines.push(JsonLine::TableHeader(headers, vec![]));
                }
                for item in arr {
                    let row_data = if let Some(ref fr) = cf.field_root {
                        get_by_path(item, fr).unwrap_or(item)
                    } else {
                        item
                    };
                    if cf.fields.is_empty() {
                        lines.push(JsonLine::ArrayValue(format_json_value(row_data)));
                    } else {
                        let cells: Vec<String> = cf.fields.iter().map(|sub| {
                            get_by_path(row_data, &sub.field)
                                .map(|v| format_card_value(v, sub))
                                .unwrap_or_default()
                        }).collect();
                        lines.push(JsonLine::TableRow(cells, vec![]));
                    }
                }
            }
            lines.push(JsonLine::Close("  ]".to_string()));
        }
        "array" => {
            if let Some(arr) = val.and_then(|v| v.as_array()) {
                let items: Vec<serde_json::Value> = if let Some(ref fr) = cf.field_root {
                    arr.iter().filter_map(|item| get_by_path(item, fr).cloned()).collect()
                } else {
                    arr.clone()
                };
                let items: Vec<&serde_json::Value> = items.iter()
                    .filter(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(true))
                    .collect();
                if let Some(ref join) = cf.join {
                    let joined: Vec<String> = items.iter().map(|v| format_json_value(v)).collect();
                    lines.push(JsonLine::KeyValue(display_name.to_string(), joined.join(join)));
                } else {
                    lines.push(JsonLine::Header(display_name.to_string(), true));
                    for item in items {
                        lines.push(JsonLine::ArrayValue(format_json_value(item)));
                    }
                    lines.push(JsonLine::Close("  ]".to_string()));
                }
            } else if let Some(v) = val {
                lines.push(JsonLine::KeyValue(display_name.to_string(), format_card_value(v, cf)));
            }
        }
        _ => {
            // string, url, date, ms, seconds, etc — single value
            if let Some(v) = val {
                lines.push(JsonLine::KeyValue(display_name.to_string(), format_card_value(v, cf)));
            }
        }
    }
}

/// Post-process lines to compute aligned column widths for table blocks
fn align_table_columns(lines: &mut Vec<JsonLine>) {
    // Find contiguous blocks of TableHeader + TableRows and compute max col widths
    let mut i = 0;
    while i < lines.len() {
        if matches!(&lines[i], JsonLine::TableHeader(_, _)) {
            let start = i;
            // Collect all column widths in this table block
            let mut col_widths: Vec<usize> = Vec::new();
            let mut j = i;
            while j < lines.len() {
                let cells = match &lines[j] {
                    JsonLine::TableHeader(c, _) => Some(c),
                    JsonLine::TableRow(c, _) => Some(c),
                    _ => None,
                };
                if let Some(cells) = cells {
                    for (col, cell) in cells.iter().enumerate() {
                        let w = cell.chars().count();
                        if col >= col_widths.len() {
                            col_widths.push(w);
                        } else if w > col_widths[col] {
                            col_widths[col] = w;
                        }
                    }
                    j += 1;
                } else {
                    break;
                }
            }
            // Apply widths back to the lines
            for k in start..j {
                match &mut lines[k] {
                    JsonLine::TableHeader(_, widths) |
                    JsonLine::TableRow(_, widths) => {
                        *widths = col_widths.clone();
                    }
                    _ => {}
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn flatten_json_to_lines(value: &serde_json::Value, prefix: &str, depth: usize) -> Vec<JsonLine> {
    let mut lines = Vec::new();
    let indent = "  ".repeat(depth);

    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                if key == "_cont3xt" { continue; }
                let display_key = if prefix.is_empty() {
                    format!("{indent}{key}")
                } else {
                    format!("{indent}{prefix}.{key}")
                };
                match val {
                    serde_json::Value::Object(_) => {
                        lines.push(JsonLine::Header(display_key, false));
                        lines.extend(flatten_json_to_lines(val, "", depth + 1));
                        lines.push(JsonLine::Close(format!("{}}}",  "  ".repeat(depth + 1))));
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.iter().all(|v| !v.is_object() && !v.is_array()) {
                            // Simple array: show inline
                            let vals: Vec<String> = arr.iter().map(format_json_value).collect();
                            lines.push(JsonLine::KeyValue(display_key, vals.join(", ")));
                        } else {
                            lines.push(JsonLine::Header(display_key, true));
                            for item in arr {
                                if item.is_object() {
                                    lines.extend(flatten_json_to_lines(item, "", depth + 1));
                                    lines.push(JsonLine::Close(format!("{}---", "  ".repeat(depth + 1))));
                                } else {
                                    lines.push(JsonLine::ArrayValue(format_json_value(item)));
                                }
                            }
                            lines.push(JsonLine::Close(format!("{}]", "  ".repeat(depth + 1))));
                        }
                    }
                    _ => {
                        lines.push(JsonLine::KeyValue(display_key, format_json_value(val)));
                    }
                }
            }
        }
        _ => {
            lines.push(JsonLine::KeyValue(format!("{indent}{prefix}"), format_json_value(value)));
        }
    }
    lines
}

fn format_json_value(val: &serde_json::Value) -> String {
    let s = match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    };
    sanitize_control_chars(&s)
}

/// Strip control characters (newlines, carriage returns, tabs, etc.) that break rendering
fn sanitize_control_chars(s: &str) -> String {
    s.chars().map(|c| if c.is_control() && c != ' ' { ' ' } else { c }).collect()
}

/// Format table cells with aligned column widths
fn format_table_cells(cells: &[String], widths: &[usize], sep: &str) -> String {
    let mut out = String::new();
    for (j, cell) in cells.iter().enumerate() {
        if j > 0 { out.push_str(sep); }
        let w = widths.get(j).copied().unwrap_or(0);
        let char_count = cell.chars().count();
        out.push_str(cell);
        if char_count < w {
            for _ in 0..(w - char_count) {
                out.push(' ');
            }
        }
    }
    out
}

/// Navigate a dotted path like "foo.bar.baz" into a JSON value
fn get_by_path<'a>(data: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    if path.is_empty() {
        return Some(data);
    }
    // Try full key first (handles flattened keys like "source.ip")
    if let Some(v) = data.get(path) {
        return Some(v);
    }
    // Fall back to nested traversal
    let mut current = data;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn defang_string(s: &str) -> String {
    s.replace("http", "hXXp").replace('.', "[.]")
}

fn format_card_value(val: &serde_json::Value, field: &CardField) -> String {
    let raw = match &field.field_type[..] {
        "date" => {
            // Try parsing as ISO date string or epoch ms
            if let Some(s) = val.as_str() {
                sanitize_control_chars(s)
            } else if let Some(n) = val.as_f64() {
                let secs = if n > 1e12 { (n / 1000.0) as i64 } else { n as i64 };
                chrono::DateTime::from_timestamp(secs, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        "ms" => {
            if let Some(n) = val.as_f64() {
                let secs = (n / 1000.0) as i64;
                chrono::DateTime::from_timestamp(secs, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        "seconds" => {
            if let Some(n) = val.as_f64() {
                chrono::DateTime::from_timestamp(n as i64, 0)
                    .map(|dt| dt.format("%Y/%m/%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format_json_value(val))
            } else {
                format_json_value(val)
            }
        }
        _ => format_json_value(val),
    };
    let raw = sanitize_control_chars(&raw);
    if field.defang { defang_string(&raw) } else { raw }
}

fn render_card_lines(card: &Cont3xtCard, data: &serde_json::Value, _indicator: &str) -> Vec<JsonLine> {
    let mut lines = Vec::new();

    for field_def in &card.fields {
        let val = get_by_path(data, &field_def.field);

        match &field_def.field_type[..] {
            "table" => {
                lines.push(JsonLine::Header(field_def.label.clone(), true));
                if let Some(arr) = val.and_then(|v| v.as_array()) {
                    // Table header
                    if !field_def.fields.is_empty() {
                        let headers: Vec<String> = field_def.fields.iter()
                            .map(|f| f.label.clone())
                            .collect();
                        lines.push(JsonLine::TableHeader(headers, vec![]));
                    }
                    // Table rows
                    for item in arr {
                        let row_data = if let Some(ref fr) = field_def.field_root {
                            get_by_path(item, fr).unwrap_or(item)
                        } else {
                            item
                        };
                        if field_def.fields.is_empty() {
                            lines.push(JsonLine::ArrayValue(format_json_value(row_data)));
                        } else {
                            let cells: Vec<String> = field_def.fields.iter().map(|sub| {
                                get_by_path(row_data, &sub.field)
                                    .map(|v| format_card_value(v, sub))
                                    .unwrap_or_default()
                            }).collect();
                            lines.push(JsonLine::TableRow(cells, vec![]));
                        }
                    }
                }
                lines.push(JsonLine::Close("  ]".to_string()));
            }
            "array" => {
                if let Some(arr) = val.and_then(|v| v.as_array()) {
                    let items: Vec<serde_json::Value> = if let Some(ref fr) = field_def.field_root {
                        arr.iter().filter_map(|item| get_by_path(item, fr).cloned()).collect()
                    } else {
                        arr.clone()
                    };
                    // Filter empty if applicable
                    let items: Vec<&serde_json::Value> = items.iter()
                        .filter(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(true))
                        .collect();
                    if let Some(ref join) = field_def.join {
                        let joined: Vec<String> = items.iter().map(|v| format_json_value(v)).collect();
                        lines.push(JsonLine::KeyValue(field_def.label.clone(), joined.join(join)));
                    } else {
                        lines.push(JsonLine::Header(field_def.label.clone(), true));
                        for item in items {
                            lines.push(JsonLine::ArrayValue(format_json_value(item)));
                        }
                        lines.push(JsonLine::Close("  ]".to_string()));
                    }
                } else if let Some(v) = val {
                    lines.push(JsonLine::KeyValue(field_def.label.clone(), format_card_value(v, field_def)));
                }
            }
            "json" => {
                lines.push(JsonLine::Header(field_def.label.clone(), false));
                if let Some(v) = val {
                    let pretty = serde_json::to_string_pretty(v).unwrap_or_else(|_| format_json_value(v));
                    for json_line in pretty.lines() {
                        lines.push(JsonLine::ArrayValue(json_line.to_string()));
                    }
                }
                lines.push(JsonLine::Close("  }".to_string()));
            }
            "dnsRecords" => {
                // DNS records: data is an object with record types as keys
                if let Some(obj) = val.or(Some(data)).and_then(|v| v.as_object()) {
                    for (rtype, rdata) in obj {
                        if rtype == "_cont3xt" { continue; }
                        lines.push(JsonLine::Header(rtype.clone(), false));
                        if let Some(answers) = rdata.get("Answer").and_then(|a| a.as_array()) {
                            for ans in answers {
                                let name = ans.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                let data_str = ans.get("data").and_then(|v| v.as_str()).unwrap_or("");
                                let ttl = ans.get("TTL").and_then(|v| v.as_u64()).unwrap_or(0);
                                lines.push(JsonLine::ArrayValue(format!("{name} → {data_str} (TTL: {ttl})")));
                            }
                        } else {
                            let status = rdata.get("Status").and_then(|v| v.as_u64()).unwrap_or(0);
                            lines.push(JsonLine::ArrayValue(format!("Status: {status}")));
                        }
                    }
                }
            }
            _ => {
                // string, url, externalLink, date, ms, seconds
                if let Some(v) = val {
                    if v.is_null() { continue; }
                    if let Some(s) = v.as_str()
                        && s.is_empty() { continue; }
                    lines.push(JsonLine::KeyValue(field_def.label.clone(), format_card_value(v, field_def)));
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(JsonLine::KeyValue("(no card fields matched)".to_string(), String::new()));
    }

    lines
}

fn c3_draw_stats(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // sub-tab bar
            Constraint::Min(0),   // table
        ])
        .split(area);

    // Sub-tab bar
    let titles: Vec<Line> = C3StatsTab::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.name())))
        .collect();
    let tabs_widget = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .select(C3StatsTab::ALL.iter().position(|&t| t == app.cont3xt.stats_tab).unwrap_or(0));
    f.render_widget(tabs_widget, chunks[0]);

    let columns = app.cont3xt.stats_tab.columns();
    let all_data = app.c3_stats_current_data();

    // Filter
    let mut filtered: Vec<&serde_json::Value> = all_data.iter()
        .filter(|item| {
            app.cont3xt.stats_filter.is_empty()
            || item.get("name").and_then(|v| v.as_str()).unwrap_or("")
                .to_lowercase().contains(&app.cont3xt.stats_filter.to_lowercase())
        })
        .collect();

    // Sort
    let sort_field = columns.get(app.cont3xt.stats_sort_col).map(|c| c.0).unwrap_or("name");
    filtered.sort_by(|a, b| {
        let cmp = if sort_field == "name" {
            let va = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let vb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            va.to_lowercase().cmp(&vb.to_lowercase())
        } else {
            let va = a.get(sort_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let vb = b.get(sort_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        };
        if app.cont3xt.stats_sort_desc { cmp.reverse() } else { cmp }
    });

    // Build header
    let header_cells: Vec<Cell> = columns.iter().enumerate().map(|(i, &(_, label, _))| {
        let is_sorted = i == app.cont3xt.stats_sort_col;
        Cell::from(sort_header_label(label, is_sorted, app.cont3xt.stats_sort_desc))
            .style(sort_header_style(is_sorted))
    }).collect();
    let header = Row::new(header_cells).height(1);

    // Build rows
    let rows: Vec<Row> = filtered.iter().map(|item| {
        let cells: Vec<Cell> = columns.iter().map(|&(field, _, _)| {
            let val = item.get(field);
            let text = match field {
                "name" => val.and_then(|v| v.as_str()).unwrap_or("").to_string(),
                "cacheRecentAvgMS" | "directRecentAvgMS" => {
                    val.and_then(|v| v.as_f64())
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_else(|| "0".to_string())
                }
                _ => {
                    val.and_then(|v| v.as_u64())
                        .map(format_number)
                        .unwrap_or_else(|| "0".to_string())
                }
            };
            if field == "name" {
                Cell::from(text)
            } else {
                Cell::from(text).style(Style::default().fg(Color::White))
            }
        }).collect();

        Row::new(cells)
    }).collect();

    let widths: Vec<Constraint> = columns.iter().map(|&(_, _, w)| Constraint::Length(w)).collect();

    let filter_info = if app.cont3xt.stats_filtering {
        format!(" /{}█ ", app.cont3xt.stats_filter)
    } else if !app.cont3xt.stats_filter.is_empty() {
        format!(" /{} ", app.cont3xt.stats_filter)
    } else {
        String::new()
    };

    let title = format!(" {} ({}) {}", app.cont3xt.stats_tab.name(), filtered.len(), filter_info);
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    app.cont3xt.stats_table_state.select(Some(app.cont3xt.stats_selected));
    f.render_stateful_widget(table, chunks[1], &mut app.cont3xt.stats_table_state);
}

fn c3_draw_history(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = C3_HISTORY_COLUMNS;

    // Client-side filter
    let filter_lower = app.cont3xt.history_filter.to_lowercase();
    let filtered: Vec<&serde_json::Value> = app.cont3xt.history_data.iter()
        .filter(|item| {
            if app.cont3xt.history_filter.is_empty() { return true; }
            item.get("indicator").and_then(|v| v.as_str()).unwrap_or("")
                .to_lowercase().contains(&filter_lower)
            || item.get("iType").and_then(|v| v.as_str()).unwrap_or("")
                .to_lowercase().contains(&filter_lower)
            || item.get("tags").and_then(|v| v.as_array())
                .map(|a| a.iter().any(|t| t.as_str().unwrap_or("").to_lowercase().contains(&filter_lower)))
                .unwrap_or(false)
        })
        .collect();

    // Build header
    let header_cells: Vec<Cell> = columns.iter().enumerate().map(|(i, &(_, label, _, sortable))| {
        let is_sorted = i == app.cont3xt.history_sort_col;
        let text = sort_header_label(label, is_sorted, app.cont3xt.history_sort_desc);
        let style = if is_sorted {
            sort_header_style(true)
        } else if sortable {
            sort_header_style(false)
        } else {
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)
        };
        Cell::from(text).style(style)
    }).collect();
    let header = Row::new(header_cells).height(1);

    // Build rows
    let rows: Vec<Row> = filtered.iter().map(|item| {
        let cells: Vec<Cell> = columns.iter().map(|&(field, _, _, _)| {
            let text = match field {
                "issuedAt" => {
                    if let Some(ms) = item.get("issuedAt").and_then(|v| v.as_u64()) {
                        format_epoch_short(ms as f64)
                    } else { "-".into() }
                }
                "iType" => item.get("iType").and_then(|v| v.as_str()).unwrap_or("-").to_string(),
                "indicator" => item.get("indicator").and_then(|v| v.as_str()).unwrap_or("-").to_string(),
                "tags" => {
                    item.get("tags").and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_else(|| "-".into())
                }
                "resultCount" => {
                    item.get("resultCount").and_then(|v| v.as_u64())
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".into())
                }
                "took" => {
                    item.get("took").and_then(|v| v.as_u64())
                        .map(|v| format!("{}ms", v))
                        .unwrap_or_else(|| "?".into())
                }
                _ => "-".into(),
            };
            let style = match field {
                "resultCount" | "took" => Style::default().fg(Color::White),
                "iType" => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            };
            Cell::from(text).style(style)
        }).collect();
        Row::new(cells)
    }).collect();

    // Indicator column gets remaining width
    let widths: Vec<Constraint> = columns.iter().map(|&(field, _, w, _)| {
        if field == "indicator" { Constraint::Min(w) } else { Constraint::Length(w) }
    }).collect();

    let filter_info = if app.cont3xt.history_filtering {
        format!(" /{}█ ", app.cont3xt.history_filter)
    } else if !app.cont3xt.history_filter.is_empty() {
        format!(" /{} ", app.cont3xt.history_filter)
    } else {
        String::new()
    };

    let total_pages = app.cont3xt.history_total.div_ceil(100);
    let page_start = (app.cont3xt.history_page - 1) * 100 + 1;
    let page_end = (page_start + app.cont3xt.history_data.len()).saturating_sub(1);
    let page_info = if app.cont3xt.history_total > 0 {
        format!(" [{}-{} of {}] ", page_start, page_end, app.cont3xt.history_total)
    } else {
        " [0] ".to_string()
    };
    let nav = if total_pages > 1 {
        format!("◄ {}/{} ► ", app.cont3xt.history_page, total_pages)
    } else { String::new() };

    let title = format!(" History{}{}{}", page_info, nav, filter_info);
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    app.cont3xt.history_table_state.select(Some(app.cont3xt.history_selected));
    f.render_stateful_widget(table, area, &mut app.cont3xt.history_table_state);
}

fn draw_link_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 30u16.min(area.height.saturating_sub(4));
    let popup_area = center_popup(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let (indicator, itype) = match app.cont3xt.tree_order.get(app.cont3xt.selected) {
        Some(C3TreeItem::Result(idx)) => app.cont3xt.results.get(*idx)
            .map(|r| (r.indicator.as_str(), r.itype.as_str()))
            .unwrap_or((app.expression.as_str(), app.cont3xt.search_itype.as_str())),
        Some(C3TreeItem::Indicator(it, q)) => (q.as_str(), it.as_str()),
        None => (app.expression.as_str(), app.cont3xt.search_itype.as_str()),
    };
    let title = format!(
        " Links for {} ({}) — {} links ",
        indicator, itype, app.cont3xt.link_flat.len()
    );
    let filter_line = if app.cont3xt.link_popup_filtering {
        format!("Filter: {}█", app.cont3xt.link_popup_filter)
    } else if !app.cont3xt.link_popup_filter.is_empty() {
        format!("Filter: {}", app.cont3xt.link_popup_filter)
    } else {
        String::new()
    };

    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(" / filter  r refresh  Enter open  q close ").centered())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content_area = if !filter_line.is_empty() {
        let filter_style = if app.cont3xt.link_popup_filtering {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new(filter_line).style(filter_style),
            Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
        );
        Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height.saturating_sub(1) }
    } else {
        inner
    };

    if app.cont3xt.link_flat.is_empty() {
        f.render_widget(
            Paragraph::new("No links available for this indicator type")
                .style(Style::default().fg(Color::DarkGray)),
            content_area,
        );
        return;
    }

    // Reserve bottom for selected link description
    let selected_info = app.cont3xt.link_flat.get(app.cont3xt.link_popup_selected)
        .map(|(_, _, url, info, _)| (url.clone(), info.clone()))
        .unwrap_or_default();
    let has_desc = !selected_info.0.is_empty();
    let desc_height = if has_desc { 3u16 } else { 0 };
    let list_area = Rect {
        x: content_area.x, y: content_area.y,
        width: content_area.width,
        height: content_area.height.saturating_sub(desc_height),
    };
    let desc_area = Rect {
        x: content_area.x, y: list_area.y + list_area.height,
        width: content_area.width,
        height: desc_height.min(content_area.height),
    };

    // Count display lines up to and including selected item to determine scroll
    let visible = list_area.height as usize;
    let selected = app.cont3xt.link_popup_selected;

    // Count total lines up to selected to find its display position
    let mut last_group = String::new();
    let mut selected_line_pos = 0usize;
    for (i, (group, _, _, _, _)) in app.cont3xt.link_flat.iter().enumerate() {
        if *group != last_group {
            if !last_group.is_empty() {
                selected_line_pos += 1; // spacer
            }
            selected_line_pos += 1; // header
            last_group = group.clone();
        }
        if i == selected { break; }
        selected_line_pos += 1; // item line
    }

    let scroll_offset = if selected_line_pos >= visible {
        selected_line_pos - visible + 1
    } else {
        0
    };

    // Render only visible lines starting from scroll_offset
    let mut lines: Vec<Line> = Vec::new();
    let mut line_idx = 0usize;
    last_group = String::new();
    for (i, (group, name, url, _info, color)) in app.cont3xt.link_flat.iter().enumerate() {
        if *group != last_group {
            if !last_group.is_empty() {
                if line_idx >= scroll_offset && lines.len() < visible {
                    lines.push(Line::from(""));
                }
                line_idx += 1;
            }
            if line_idx >= scroll_offset && lines.len() < visible {
                lines.push(Line::from(Span::styled(
                    format!("── {} ──", group),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
            }
            line_idx += 1;
            last_group = group.clone();
        }
        if lines.len() >= visible { break; }
        if line_idx >= scroll_offset {
            let name_color = if i == selected {
                Color::Black
            } else {
                parse_hex_color(color).unwrap_or(Color::White)
            };
            let style = if i == selected {
                Style::default().fg(name_color).bg(Color::Yellow)
            } else {
                Style::default().fg(name_color)
            };
            let max_url_len = (popup_width as usize).saturating_sub(name.len() + 6);
            let url_display = if max_url_len == 0 {
                String::new()
            } else if url.len() > max_url_len {
                format!("{}…", &url[..max_url_len.saturating_sub(1)])
            } else {
                url.clone()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {name}"), style),
                Span::styled(format!("  {url_display}"), if i == selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
            ]));
        }
        line_idx += 1;
    }

    f.render_widget(Paragraph::new(lines), list_area);

    // Render description of selected link
    if has_desc {
        let mut desc_lines: Vec<Line> = Vec::new();
        desc_lines.push(Line::from(Span::styled(
            "─".repeat(desc_area.width as usize),
            Style::default().fg(Color::DarkGray),
        )));
        let url_line = format!("URL: {}", selected_info.0);
        let w = desc_area.width as usize;
        desc_lines.push(Line::from(Span::styled(
            if url_line.len() > w { format!("{}…", &url_line[..w.saturating_sub(1)]) } else { url_line },
            Style::default().fg(Color::Gray),
        )));
        if !selected_info.1.is_empty() {
            desc_lines.push(Line::from(Span::styled(
                selected_info.1.clone(),
                Style::default().fg(Color::Green),
            )));
        }
        f.render_widget(Paragraph::new(desc_lines), desc_area);
    }
}

fn draw_integration_popup(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::IntegrationPopupMode;

    let popup_width = 50u16.min(area.width.saturating_sub(4));

    match app.cont3xt.integration_popup_mode {
        IntegrationPopupMode::Views | IntegrationPopupMode::SaveInput | IntegrationPopupMode::ConfirmDelete => {
            // Views list: "Save Current" + saved views
            let list_len = app.cont3xt.views.len() + 1; // +1 for "Save Current"
            let popup_height = (list_len as u16 + 4).min(area.height.saturating_sub(4)).max(6);
            let popup_area = center_popup(popup_width, popup_height, area);

            f.render_widget(Clear, popup_area);

            let bottom_line = match app.cont3xt.integration_popup_mode {
                IntegrationPopupMode::SaveInput => {
                    let cursor = format!(" Name: {}█ ", app.cont3xt.view_save_name);
                    Line::from(Span::styled(cursor, Style::default().fg(Color::Yellow))).centered()
                }
                IntegrationPopupMode::ConfirmDelete => {
                    let name = app.cont3xt.views.get(app.cont3xt.view_selected.saturating_sub(1))
                        .map(|v| v.name.as_str()).unwrap_or("?");
                    Line::from(Span::styled(
                        format!(" Delete '{name}'? (y/n) "),
                        Style::default().fg(Color::Red)
                    )).centered()
                }
                _ => Line::from(" Enter:load  x:delete  Esc:back ").centered(),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title(" Views ")
                .title_bottom(bottom_line);
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let visible_height = inner.height as usize;
            let scroll_offset = if list_len > visible_height {
                let sel = app.cont3xt.view_selected;
                if sel >= visible_height { sel - visible_height + 1 } else { 0 }
            } else { 0 };

            for i in scroll_offset..(scroll_offset + visible_height).min(list_len) {
                let y = inner.y + (i - scroll_offset) as u16;
                let is_selected = i == app.cont3xt.view_selected;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Magenta)
                } else {
                    Style::default().fg(Color::White)
                };

                if i == 0 {
                    // "Save Current" option
                    let enabled = app.cont3xt.integrations.len() - app.cont3xt.disabled_integrations.len();
                    let label = format!(" 💾 Save Current ({enabled} integrations)");
                    f.render_widget(Paragraph::new(Span::styled(label, style)), Rect::new(inner.x, y, inner.width, 1));
                } else {
                    let view = &app.cont3xt.views[i - 1];
                    let count = view.integrations.len();
                    let shared = if !view.editable { " 🔗" } else { "" };
                    let label = format!(" {} ({count}){shared}", view.name);
                    let id_str = view.id.as_str();
                    let w = inner.width as usize;
                    let label_len = label.chars().count() + if !view.editable { 1 } else { 0 }; // 🔗 is 2 cells wide
                    let id_len = id_str.len() + 1; // +1 for trailing space
                    let gap = w.saturating_sub(label_len + id_len);
                    let line = Line::from(vec![
                        Span::styled(label, style),
                        Span::styled(" ".repeat(gap), style),
                        Span::styled(format!("{id_str} "), style.patch(Style::default().fg(Color::DarkGray))),
                    ]);
                    f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                }
            }
        }
        IntegrationPopupMode::Integrations => {
            let filtered: Vec<(usize, &crate::api::Cont3xtIntegration)> = app.cont3xt.integrations.iter().enumerate()
                .filter(|(_, int)| {
                    app.cont3xt.integration_popup_filter.is_empty()
                    || int.name.to_lowercase().contains(&app.cont3xt.integration_popup_filter.to_lowercase())
                })
                .collect();

            let disabled_count = app.cont3xt.disabled_integrations.len();
            let total = app.cont3xt.integrations.len();
            let enabled = total - disabled_count;

            let popup_height = (filtered.len() as u16 + 5).min(area.height.saturating_sub(4));
            let popup_area = center_popup(popup_width, popup_height, area);

            f.render_widget(Clear, popup_area);

            let bottom_line = if app.cont3xt.integration_popup_filtering {
                let cursor = format!(" /{}█ ", app.cont3xt.integration_popup_filter);
                Line::from(Span::styled(cursor, Style::default().fg(Color::Yellow))).centered()
            } else if !app.cont3xt.integration_popup_filter.is_empty() {
                Line::from(format!(" /{} │ Spc:toggle a:all n:none !:inv v:views ", app.cont3xt.integration_popup_filter)).centered()
            } else {
                Line::from(" Spc:toggle a:all n:none !:inv /:filter v:views ").centered()
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" Integrations ({enabled}/{total}) "))
                .title_bottom(bottom_line);
            let inner = block.inner(popup_area);
            f.render_widget(block, popup_area);

            let visible_height = inner.height as usize;
            let scroll_offset = if filtered.len() > visible_height {
                let sel = app.cont3xt.integration_popup_selected;
                if sel >= visible_height { sel - visible_height + 1 } else { 0 }
            } else { 0 };

            for (i, (_, integ)) in filtered.iter().enumerate().skip(scroll_offset).take(visible_height) {
                let y = inner.y + (i - scroll_offset) as u16;
                let is_selected = i == app.cont3xt.integration_popup_selected;
                let is_disabled = app.cont3xt.disabled_integrations.contains(&integ.name);

                let check = if is_disabled { "✗" } else { "✓" };
                let check_color = if is_disabled { Color::Red } else { Color::Green };

                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                if is_selected {
                    let label = format!(" {check} {}", integ.name);
                    f.render_widget(Paragraph::new(Span::styled(label, style)), Rect::new(inner.x, y, inner.width, 1));
                } else {
                    let line = Line::from(vec![
                        Span::styled(format!(" {check} "), Style::default().fg(check_color)),
                        Span::styled(integ.name.clone(), style),
                    ]);
                    f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
                }
            }
        }
    }
}

fn draw_card_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_w = (area.width - 4).min(80);
    let popup_h = (area.height - 4).min(40);
    let popup_area = center_popup(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let (title, text) = match app.cont3xt.tree_order.get(app.cont3xt.selected) {
        Some(C3TreeItem::Result(idx)) => {
            let result = match app.cont3xt.results.get(*idx) {
                Some(r) => r,
                None => return,
            };
            let card = app.cont3xt.integrations.iter()
                .find(|i| i.name == result.name)
                .and_then(|i| i.card.as_ref());
            let title = format!(" Card: {} ", result.name);
            let text = if let Some(card) = card {
                let mut lines = Vec::new();
                lines.push(format!("Title: {}", card.title));
                lines.push(String::new());
                for (fi, field) in card.fields.iter().enumerate() {
                    lines.push(format!("Field[{}]:", fi));
                    lines.push(format!("  label: {}", field.label));
                    lines.push(format!("  field: {}", field.field));
                    lines.push(format!("  type: {}", field.field_type));
                    if let Some(ref join) = field.join {
                        lines.push(format!("  join: {:?}", join));
                    }
                    if field.defang {
                        lines.push("  defang: true".to_string());
                    }
                    if let Some(ref root) = field.field_root {
                        lines.push(format!("  fieldRoot: {}", root));
                    }
                    if field.filter_empty {
                        lines.push("  filterEmpty: true".to_string());
                    }
                    if !field.fields.is_empty() {
                        lines.push("  sub-fields:".to_string());
                        for sf in &field.fields {
                            lines.push(format!("    - {} ({}): {}", sf.label, sf.field_type, sf.field));
                        }
                    }
                    lines.push(String::new());
                }
                lines.join("\n")
            } else {
                "No card definition found for this integration.".to_string()
            };
            (title, text)
        }
        Some(C3TreeItem::Indicator(itype, query)) => {
            let itype_lower = itype.to_lowercase();
            let overview = app.cont3xt.overviews.iter()
                .find(|o| o.itype.to_lowercase() == itype_lower && o.is_default)
                .or_else(|| app.cont3xt.overviews.iter().find(|o| o.itype.to_lowercase() == itype_lower));
            let title = format!(" Overview: {} {} ", itype, query);
            let text = if let Some(ov) = overview {
                let mut lines = Vec::new();
                lines.push(format!("Name: {}", ov.name));
                lines.push(format!("Title: {}", ov.title));
                lines.push(format!("iType: {}", ov.itype));
                lines.push(format!("Default: {}", ov.is_default));
                lines.push(String::new());
                for (fi, field) in ov.fields.iter().enumerate() {
                    lines.push(format!("Field[{}]:", fi));
                    lines.push(format!("  type: {}", field.field_type));
                    lines.push(format!("  from: {}", field.from));
                    lines.push(format!("  field: {}", field.field));
                    if let Some(ref alias) = field.alias {
                        lines.push(format!("  alias: {}", alias));
                    }
                    lines.push(String::new());
                }
                lines.join("\n")
            } else {
                format!("No overview found for iType '{}'", itype)
            };
            (title, text)
        }
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let para = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((app.cont3xt.card_popup_scroll, 0));
    f.render_widget(para, inner);
}

fn draw_overview_popup(f: &mut Frame, app: &App, area: Rect) {
    let (itype, _query) = match app.cont3xt.tree_order.get(app.cont3xt.selected) {
        Some(C3TreeItem::Indicator(itype, query)) => (itype.clone(), query.clone()),
        _ => return,
    };
    let itype_lower = itype.to_lowercase();
    let filter_lower = app.cont3xt.overview_popup_filter.to_lowercase();
    let mut matching: Vec<&crate::api::Cont3xtOverview> = app.cont3xt.overviews.iter()
        .filter(|o| o.itype.to_lowercase() == itype_lower)
        .filter(|o| filter_lower.is_empty() || o.name.to_lowercase().contains(&filter_lower))
        .collect();
    matching.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let current_id = app.cont3xt.selected_overviews.get(&itype_lower);

    let filter_row = if app.cont3xt.overview_popup_filtering || !app.cont3xt.overview_popup_filter.is_empty() { 1u16 } else { 0 };
    let popup_w = (area.width - 4).min(60);
    let popup_h = (matching.len() as u16 + 4 + filter_row).min(area.height - 4);
    let popup_area = center_popup(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Select Overview ({}) ", itype));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut y_offset = 0u16;

    if app.cont3xt.overview_popup_filtering || !app.cont3xt.overview_popup_filter.is_empty() {
        let filter_style = if app.cont3xt.overview_popup_filtering {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let filter_text = format!(" /{}█", app.cont3xt.overview_popup_filter);
        f.render_widget(
            Paragraph::new(Span::styled(filter_text, filter_style)),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        y_offset = 1;
    }

    for (i, ov) in matching.iter().enumerate() {
        if y_offset + i as u16 >= inner.height { break; }
        let is_selected = i == app.cont3xt.overview_popup_selected;
        let is_active = Some(&ov.id) == current_id
            || (current_id.is_none() && ov.is_default)
            || (current_id.is_none() && i == 0 && !matching.iter().any(|o| o.is_default));

        let marker = if is_active { "● " } else { "  " };
        let default_tag = if ov.is_default { " (default)" } else { "" };
        let label = format!("{marker}{}{default_tag}", ov.name);

        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        f.render_widget(
            Paragraph::new(Span::styled(format!(" {label:<width$}", width = inner.width as usize - 1), style)),
            Rect::new(inner.x, inner.y + y_offset + i as u16, inner.width, 1),
        );
    }
}

fn draw_save_json_prompt(f: &mut Frame, app: &App, area: Rect) {
    let filename = match &app.cont3xt.save_json_prompt {
        Some(f) => f,
        None => return,
    };

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 3u16;
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

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
                .title(" Save JSON "),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_tags_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 5u16;
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(vec![
            Span::styled("Tags: ", Style::default().fg(Color::Yellow)),
            Span::styled(&app.cont3xt.tags_edit, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ]),
        Line::from(Span::styled(
            "  comma separated, Enter to set, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Edit Tags "),
        );
    f.render_widget(paragraph, popup_area);
}

fn draw_date_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 8u16;
    let popup_area = center_popup(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let start_style = if app.cont3xt.date_field == 0 { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) };
    let stop_style = if app.cont3xt.date_field == 1 { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) };
    let cursor = Span::styled("█", Style::default().fg(Color::Gray));

    let mut start_spans = vec![
        Span::styled(" Start: ", Style::default().fg(Color::Yellow)),
        Span::styled(&app.cont3xt.date_start_edit, start_style),
    ];
    if app.cont3xt.date_field == 0 { start_spans.push(cursor.clone()); }

    let mut stop_spans = vec![
        Span::styled("  Stop: ", Style::default().fg(Color::Yellow)),
        Span::styled(&app.cont3xt.date_stop_edit, stop_style),
    ];
    if app.cont3xt.date_field == 1 { stop_spans.push(cursor); }

    let lines = vec![
        Line::from(start_spans),
        Line::from(stop_spans),
        Line::from(""),
        Line::from(Span::styled(
            "  Tab/↑↓ switch, Enter set, Esc cancel",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  now, -7d, -1h, -30m, YYYY-MM-DD",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Date Range "),
        );
    f.render_widget(paragraph, popup_area);
}
