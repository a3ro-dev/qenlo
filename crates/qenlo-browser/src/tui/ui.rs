use crate::tui::app::{App, Tab};
use crate::tui::theme::QENLO_THEME;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Tabs, Wrap,
    },
};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(8),    // Main Content
            Constraint::Length(3), // Claude Code Command / Status Bar
        ])
        .split(size);

    render_header(frame, app, chunks[0]);
    render_tabs(frame, app, chunks[1]);

    match app.current_tab {
        Tab::DataRows => render_rows_view(frame, app, chunks[2]),
        Tab::VectorSearch => render_search_view(frame, app, chunks[2]),
        Tab::StorageWal => render_storage_view(frame, app, chunks[2]),
        Tab::Diagnostics => render_diagnostics_view(frame, app, chunks[2]),
        Tab::Help => render_help_view(frame, app, chunks[2]),
    }

    render_bottom_bar(frame, app, chunks[3]);

    // Modals
    if let Some(record) = &app.inspect_modal {
        render_inspect_modal(frame, record, size);
    } else if app.add_modal {
        render_add_modal(frame, app, size);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let path_str = app.status.path.as_deref().unwrap_or("In-Memory");
    let status_color = if app.status.open && !app.status.closed {
        QENLO_THEME.ok
    } else {
        QENLO_THEME.bad
    };

    let title_line = Line::from(vec![
        Span::styled(
            " ⬡ QENLO ",
            Style::default()
                .fg(QENLO_THEME.bg)
                .bg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " BROWSER ",
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("📁 {path_str}"),
            Style::default()
                .fg(QENLO_THEME.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Dim: {}D", app.status.dimension),
            Style::default().fg(QENLO_THEME.accent),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Live: {} ({})", app.status.live_rows, app.total_records),
            Style::default().fg(QENLO_THEME.text),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Gen: #{}", app.status.generation),
            Style::default().fg(QENLO_THEME.text_muted),
        ),
        Span::raw(" │ "),
        Span::styled(
            if app.status.open {
                "● READY"
            } else {
                "○ CLOSED"
            },
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(QENLO_THEME.border))
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(QENLO_THEME.surface));

    let p = Paragraph::new(title_line)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(p, area);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec![
        Tab::DataRows.title(),
        Tab::VectorSearch.title(),
        Tab::StorageWal.title(),
        Tab::Diagnostics.title(),
        Tab::Help.title(),
    ];

    let current_idx = match app.current_tab {
        Tab::DataRows => 0,
        Tab::VectorSearch => 1,
        Tab::StorageWal => 2,
        Tab::Diagnostics => 3,
        Tab::Help => 4,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(QENLO_THEME.border))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(QENLO_THEME.bg)),
        )
        .select(current_idx)
        .style(Style::default().fg(QENLO_THEME.text_muted))
        .highlight_style(
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    frame.render_widget(tabs, area);
}

fn render_rows_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // Table
            Constraint::Length(2), // Pagination / info
        ])
        .split(area);

    let header_row = Row::new(vec![
        cell_styled(
            " ID",
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        cell_styled(
            "USER ID",
            Style::default()
                .fg(QENLO_THEME.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
        cell_styled(
            "TIMESTAMP",
            Style::default()
                .fg(QENLO_THEME.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
        cell_styled(
            "STATUS",
            Style::default()
                .fg(QENLO_THEME.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
        cell_styled(
            "VECTOR PREVIEW",
            Style::default()
                .fg(QENLO_THEME.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .style(Style::default().bg(QENLO_THEME.surface_raised))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .records
        .iter()
        .enumerate()
        .map(|(idx, r)| {
            let is_selected = idx == app.selected_idx;
            let status_span = if r.live {
                Span::styled(" ● Active ", Style::default().fg(QENLO_THEME.ok))
            } else {
                Span::styled(" ✕ Deleted ", Style::default().fg(QENLO_THEME.bad))
            };

            let vec_preview: String = r
                .vector
                .iter()
                .take(4)
                .map(|v| format!("{v:.3}"))
                .collect::<Vec<_>>()
                .join(", ");
            let vec_str = format!(
                "[{vec_preview}{}]",
                if r.vector.len() > 4 { ", …" } else { "" }
            );

            let mut row = Row::new(vec![
                Line::from(format!(" #{}", r.id)),
                Line::from(format!(" {}", r.user_id)),
                Line::from(format!(" {}", r.timestamp)),
                Line::from(status_span),
                Line::from(Span::styled(
                    vec_str,
                    Style::default().fg(QENLO_THEME.text_muted),
                )),
            ]);

            if is_selected {
                row = row.style(
                    Style::default()
                        .fg(QENLO_THEME.text)
                        .bg(QENLO_THEME.surface_raised)
                        .add_modifier(Modifier::BOLD),
                );
            }
            row
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Length(16),
        Constraint::Length(14),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths).header(header_row).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .title(Span::styled(
                format!(" Records Data Grid (Total: {}) ", app.total_records),
                Style::default()
                    .fg(QENLO_THEME.text)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(QENLO_THEME.surface)),
    );

    frame.render_widget(table, chunks[0]);

    // Pagination info
    let page_start = if app.total_records == 0 {
        0
    } else {
        app.page_offset + 1
    };
    let page_end = (app.page_offset + app.records.len()).min(app.total_records);
    let page_num = (app.page_offset / app.page_limit.max(1)) + 1;
    let total_pages = ((app.total_records + app.page_limit - 1) / app.page_limit.max(1)).max(1);

    let filter_notice = if !app.filter_input.is_empty() {
        format!(" · Filter: uid='{}'", app.filter_input)
    } else {
        String::new()
    };

    let p_info = Paragraph::new(format!(
        " Showing rows {page_start}..{page_end} of {} (Page {page_num}/{total_pages}){filter_notice}  │  [j/k] Navigate  [Enter] Inspect  [a] Add  [d] Delete  [n/p] Page  [/] Filter",
        app.total_records
    ))
    .style(Style::default().fg(QENLO_THEME.text_muted));

    frame.render_widget(p_info, chunks[1]);
}

fn cell_styled(text: &'static str, style: Style) -> Line<'static> {
    Line::from(Span::styled(text, style))
}

fn render_search_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Left: Query Configuration
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Query vector input
            Constraint::Length(3), // Filter user id input
            Constraint::Length(3), // k slider
            Constraint::Min(4),    // Action buttons / instructions
        ])
        .split(chunks[0]);

    let vec_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.search_active_field == 0 {
            Style::default().fg(QENLO_THEME.accent)
        } else {
            Style::default().fg(QENLO_THEME.border)
        })
        .border_type(BorderType::Rounded)
        .title(" 1. Query Vector Float Array ")
        .style(Style::default().bg(QENLO_THEME.surface));
    let vec_p = Paragraph::new(app.search_vec_input.as_str())
        .block(vec_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(vec_p, left_chunks[0]);

    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.search_active_field == 2 {
            Style::default().fg(QENLO_THEME.accent)
        } else {
            Style::default().fg(QENLO_THEME.border)
        })
        .border_type(BorderType::Rounded)
        .title(" 2. Filter User ID (optional) ")
        .style(Style::default().bg(QENLO_THEME.surface));
    let filter_p = Paragraph::new(if app.search_user_filter.is_empty() {
        "<No filter - all rows eligible>"
    } else {
        &app.search_user_filter
    })
    .block(filter_block)
    .style(if app.search_user_filter.is_empty() {
        Style::default().fg(QENLO_THEME.text_faint)
    } else {
        Style::default().fg(QENLO_THEME.text)
    });
    frame.render_widget(filter_p, left_chunks[1]);

    let k_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(QENLO_THEME.border))
        .border_type(BorderType::Rounded)
        .title(format!(" 3. Target Hits (k = {}) ", app.search_k))
        .style(Style::default().bg(QENLO_THEME.surface));
    let k_gauge = Gauge::default()
        .block(k_block)
        .gauge_style(
            Style::default()
                .fg(QENLO_THEME.accent)
                .bg(QENLO_THEME.surface_raised),
        )
        .ratio((app.search_k as f64) / 64.0)
        .label(format!("{}/64 (+/- to adjust)", app.search_k));
    frame.render_widget(k_gauge, left_chunks[2]);

    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(QENLO_THEME.border))
        .border_type(BorderType::Rounded)
        .title(" Query Shortcuts ")
        .style(Style::default().bg(QENLO_THEME.surface));
    let action_text = vec![
        Line::from(vec![
            Span::styled(
                " [Enter] / [s] ",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Run Cosine Vector Query"),
        ]),
        Line::from(vec![
            Span::styled(
                " [r]           ",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Generate Random Query Vector"),
        ]),
        Line::from(vec![
            Span::styled(
                " [+/-]         ",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Adjust Result Limit k"),
        ]),
        Line::from(vec![
            Span::styled(
                " [Tab]         ",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Switch Input Field"),
        ]),
    ];
    let action_p = Paragraph::new(action_text).block(action_block);
    frame.render_widget(action_p, left_chunks[3]);

    // Right: Ranked Results
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(chunks[1]);

    let metrics_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(QENLO_THEME.border))
        .border_type(BorderType::Rounded)
        .title(" Execution Report ")
        .style(Style::default().bg(QENLO_THEME.surface));
    let metrics_text = if app.search_metrics.is_empty() {
        "No query executed yet. Press [Enter] to run."
    } else {
        &app.search_metrics
    };
    let metrics_p = Paragraph::new(metrics_text)
        .block(metrics_block)
        .style(Style::default().fg(QENLO_THEME.accent));
    frame.render_widget(metrics_p, right_chunks[0]);

    let results_items: Vec<ListItem> = if app.search_results.is_empty() {
        vec![ListItem::new("  (No hits to display)")]
    } else {
        app.search_results
            .iter()
            .enumerate()
            .map(|(idx, hit)| {
                let sim_bars = (hit.similarity * 10.0).clamp(0.0, 10.0) as usize;
                let bar_str = format!("{}{}", "█".repeat(sim_bars), "░".repeat(10 - sim_bars));
                let user_str = hit
                    .record
                    .as_ref()
                    .map_or("".to_string(), |r| format!(" · User: {}", r.user_id));
                let ts_str = hit
                    .record
                    .as_ref()
                    .map_or("".to_string(), |r| format!(" · Ts: {}", r.timestamp));

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" #{:<2} ", idx + 1),
                        Style::default()
                            .fg(QENLO_THEME.bg)
                            .bg(QENLO_THEME.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("ID #{:<6}", hit.id),
                        Style::default()
                            .fg(QENLO_THEME.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" Dist: {:.4} ", hit.distance),
                        Style::default().fg(QENLO_THEME.text_muted),
                    ),
                    Span::styled(
                        format!("[{bar_str}] {:.3} sim", hit.similarity),
                        Style::default().fg(QENLO_THEME.ok),
                    ),
                    Span::styled(
                        format!("{user_str}{ts_str}"),
                        Style::default().fg(QENLO_THEME.text_faint),
                    ),
                ]))
            })
            .collect()
    };

    let results_list = List::new(results_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .title(format!(
                " Top-{} Nearest Neighbors ",
                app.search_results.len()
            ))
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(results_list, right_chunks[1]);
}

fn render_storage_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Summary cards
            Constraint::Min(6),    // Files table
        ])
        .split(area);

    let summary = if let Some(s) = &app.storage_details {
        format!(
            " Total Disk: {}  │  Canonical Generation: #{}  │  Durable Generation: #{}  │  Max Budget: {}",
            format_bytes(s.total_bytes),
            s.generation,
            s.durable_generation
                .map_or("None".to_string(), |g| g.to_string()),
            format_bytes(s.max_load_bytes)
        )
    } else {
        " Storage details unavailable".to_string()
    };

    let sum_p = Paragraph::new(vec![
        Line::from(Span::styled(
            " Qenlo Storage & Compaction Manager",
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(summary),
        Line::from(Span::styled(
            " Actions: [f] Flush WAL & Compact to Snapshot  │  [e] Export Snapshot  │  [r] Refresh",
            Style::default().fg(QENLO_THEME.text_muted),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(sum_p, chunks[0]);

    let file_rows: Vec<Row> = if let Some(s) = &app.storage_details {
        s.files
            .iter()
            .map(|f| {
                Row::new(vec![
                    Line::from(Span::styled(
                        &f.name,
                        Style::default()
                            .fg(QENLO_THEME.text)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(&f.kind, Style::default().fg(QENLO_THEME.ok))),
                    Line::from(format_bytes(f.size_bytes)),
                    Line::from(Span::styled(
                        &f.path,
                        Style::default().fg(QENLO_THEME.text_muted),
                    )),
                ])
            })
            .collect()
    } else {
        Vec::new()
    };

    let file_table = Table::new(
        file_rows,
        [
            Constraint::Length(24),
            Constraint::Length(20),
            Constraint::Length(16),
            Constraint::Min(30),
        ],
    )
    .header(
        Row::new(vec!["FILE NAME", "TYPE", "SIZE", "PATH"])
            .style(
                Style::default()
                    .bg(QENLO_THEME.surface_raised)
                    .fg(QENLO_THEME.text_muted),
            )
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .title(" On-Disk Collection Snapshot & WAL Files ")
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(file_table, chunks[1]);
}

fn render_diagnostics_view(frame: &mut Frame, app: &App, area: Rect) {
    let diag = app.diagnostics.as_ref();
    let os = diag.map_or("-", |d| d.os.as_str());
    let arch = diag.map_or("-", |d| d.arch.as_str());
    let cpu_path = diag.map_or("-", |d| d.cpu_distance_path.as_str());

    let text = vec![
        Line::from(Span::styled(
            " Qenlo Native Engine & Diagnostics",
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(format!(" • Host OS:                  {os}")),
        Line::from(format!(" • Target Architecture:      {arch}")),
        Line::from(vec![
            Span::raw(" • CPU SIMD Acceleration:    "),
            Span::styled(
                cpu_path,
                Style::default()
                    .fg(QENLO_THEME.ok)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!(
            " • Dimension:                {}D",
            app.status.dimension
        )),
        Line::from(" • Max Supported k:          64 (exact bounded heap)"),
        Line::from(" • Exact CPU Order:          AVX2+FMA > AVX2 > NEON > Scalar Fallback"),
        Line::from(" • Durability Guarantee:     Checksummed snapshots + sync WAL manifests"),
    ];

    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .title(" Hardware Environment & Engine Capabilities ")
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(p, area);
}

fn render_help_view(frame: &mut Frame, _app: &App, area: Rect) {
    let text = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts & Navigation Reference",
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(" [Tab] / [Shift+Tab]   Switch active navigation tab"),
        Line::from(
            " [1] - [4]             Jump directly to tab (1: Rows, 2: Search, 3: Storage, 4: Diag)",
        ),
        Line::from(" [j] / [k] / [↑] / [↓] Navigate rows in the table"),
        Line::from(" [n] / [p]             Next / Previous page of records"),
        Line::from(" [Enter]               Inspect full vector components for selected row"),
        Line::from(" [a]                   Open 'Add Record' modal dialog"),
        Line::from(" [d] / [Delete]        Delete selected row (durable tombstone)"),
        Line::from(" [/]                   Filter data grid by User ID"),
        Line::from(" [s]                   Execute vector cosine query"),
        Line::from(" [r]                   Generate random query vector"),
        Line::from(" [f]                   Flush and compact WAL to snapshot"),
        Line::from(
            " [:]                   Claude Code-style command prompt (:open, :create, :flush, :quit)",
        ),
        Line::from(" [q] / [Ctrl+C]        Quit QenloDB Browser"),
        Line::raw(""),
        Line::from(Span::styled(
            " Interactive Commands (press ':' to activate):",
            Style::default().fg(QENLO_THEME.accent),
        )),
        Line::from("  :open <path> [dim]   Open a durable collection directory or .qn archive"),
        Line::from("  :create <path> <dim> Create a brand new durable vector collection"),
        Line::from("  :flush               Flush pending WAL entries to canonical snapshot"),
        Line::from("  :export <file.qn>    Export collection into portable archive"),
        Line::from("  :search              Switch to search view"),
        Line::from("  :quit                Exit application"),
    ];

    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .title(" Help & Command Cheat Sheet ")
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(p, area);
}

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.command_mode {
        let cmd_line = Line::from(vec![
            Span::styled(
                " :",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.command_input, Style::default().fg(QENLO_THEME.text)),
            Span::styled("█", Style::default().fg(QENLO_THEME.accent)),
        ]);
        let p = Paragraph::new(cmd_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(QENLO_THEME.accent))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(QENLO_THEME.surface_raised)),
        );
        frame.render_widget(p, area);
        return;
    }

    if app.filter_mode {
        let filter_line = Line::from(vec![
            Span::styled(
                " Filter (user_id): ",
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.filter_input, Style::default().fg(QENLO_THEME.text)),
            Span::styled("█", Style::default().fg(QENLO_THEME.accent)),
        ]);
        let p = Paragraph::new(filter_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(QENLO_THEME.accent))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(QENLO_THEME.surface_raised)),
        );
        frame.render_widget(p, area);
        return;
    }

    // Normal mode: Show status message or shortcut hints
    let status_span = if let Some((msg, _, is_err)) = &app.status_message {
        if *is_err {
            Span::styled(
                format!(" ✕ {msg}"),
                Style::default()
                    .fg(QENLO_THEME.bad)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" ℹ {msg}"), Style::default().fg(QENLO_THEME.ok))
        }
    } else {
        Span::styled(
            " [Tab] Switch  [j/k] Navigate  [Enter] Inspect  [s] Search  [/] Filter  [:] Command  [?] Help  [q] Quit",
            Style::default().fg(QENLO_THEME.text_muted),
        )
    };

    let p = Paragraph::new(Line::from(status_span)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(QENLO_THEME.border))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(QENLO_THEME.surface)),
    );
    frame.render_widget(p, area);
}

fn render_inspect_modal(frame: &mut Frame, record: &crate::state::RecordDto, area: Rect) {
    let modal_area = centered_rect(65, 70, area);
    frame.render_widget(Clear, modal_area);

    let status_str = if record.live {
        "Active"
    } else {
        "Tombstone (Deleted)"
    };
    let vec_formatted: String = record
        .vector
        .iter()
        .map(|v| format!("{v:.6}"))
        .collect::<Vec<_>>()
        .join(", ");

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("Record ID: #{}", record.id),
                Style::default()
                    .fg(QENLO_THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   │   "),
            Span::styled(
                format!("User ID: {}", record.user_id),
                Style::default().fg(QENLO_THEME.text),
            ),
            Span::raw("   │   "),
            Span::styled(
                format!("Timestamp: {}", record.timestamp),
                Style::default().fg(QENLO_THEME.text),
            ),
            Span::raw("   │   "),
            Span::styled(
                format!("Status: {status_str}"),
                Style::default().fg(if record.live {
                    QENLO_THEME.ok
                } else {
                    QENLO_THEME.bad
                }),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            format!(
                "Normalized Vector Components ({} dims):",
                record.vector.len()
            ),
            Style::default().fg(QENLO_THEME.text_muted),
        )),
        Line::from(Span::styled(
            format!("[{vec_formatted}]"),
            Style::default().fg(QENLO_THEME.text),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "Press [Esc] or [Enter] to close",
            Style::default().fg(QENLO_THEME.accent),
        )),
    ];

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(QENLO_THEME.accent))
                .border_type(BorderType::Rounded)
                .title(format!(" Record Inspector #{} ", record.id))
                .style(Style::default().bg(QENLO_THEME.surface_raised)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(p, modal_area);
}

fn render_add_modal(frame: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(60, 55, area);
    frame.render_widget(Clear, modal_area);

    let fields = [
        ("Record ID (u64):", &app.add_id, 0),
        ("User ID (u64):", &app.add_user_id, 1),
        ("Timestamp (i64):", &app.add_ts, 2),
        ("Vector Components [f32, ...]:", &app.add_vec, 3),
    ];

    let mut lines = Vec::new();
    for (label, val, idx) in fields {
        let is_active = idx == app.add_field_idx;
        let prefix = if is_active { " ▶ " } else { "   " };
        let style = if is_active {
            Style::default()
                .fg(QENLO_THEME.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(QENLO_THEME.text)
        };

        lines.push(Line::from(Span::styled(format!("{prefix}{label}"), style)));
        lines.push(Line::from(Span::styled(
            format!("    {val}{}", if is_active { "█" } else { "" }),
            Style::default().fg(QENLO_THEME.text),
        )));
        lines.push(Line::raw(""));
    }

    lines.push(Line::from(Span::styled(
        " [Tab] Next Field  │  [Enter] Insert Record  │  [Ctrl+R] Random Vector  │  [Esc] Cancel",
        Style::default().fg(QENLO_THEME.accent),
    )));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(QENLO_THEME.accent))
                .border_type(BorderType::Rounded)
                .title(" Add New Record ")
                .style(Style::default().bg(QENLO_THEME.surface_raised)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(p, modal_area);
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

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let k = 1024.0;
    let sizes = ["B", "KiB", "MiB", "GiB"];
    let i = (bytes as f64).log(k).floor() as usize;
    let i = i.min(sizes.len() - 1);
    format!("{:.2} {}", (bytes as f64) / k.powi(i as i32), sizes[i])
}
