//! Runs panel - displays action runs in a table format.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;
use houston_api::ActionRun;

/// Renders the runs panel with the given action runs.
pub fn render_runs_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    runs: &[ActionRun],
    selected: usize,
    loading: bool,
) {
    let title = if ctx.zoomed {
        " Runs [ZOOMED - z to exit] "
    } else {
        " Runs "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    if ctx.is_compact() {
        render_compact(f, ctx.area, block, runs, selected, loading);
    } else {
        render_full(f, ctx.area, block, runs, selected, loading);
    }
}

fn render_full(
    f: &mut Frame,
    area: Rect,
    block: Block,
    runs: &[ActionRun],
    selected: usize,
    loading: bool,
) {
    // Header row
    let header = Row::new(vec![
        Cell::from("STATUS").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("WORKFLOW").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("REF").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("STARTED").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = if loading && runs.is_empty() {
        vec![Row::new(vec![Cell::from("Loading...")]).style(Style::default().fg(Color::DarkGray))]
    } else if runs.is_empty() {
        vec![
            Row::new(vec![Cell::from("No runs found")]).style(Style::default().fg(Color::DarkGray)),
        ]
    } else {
        runs.iter()
            .enumerate()
            .map(|(i, run)| {
                let (icon, status_color) = status_style(run);
                let status_text = run.conclusion.as_deref().unwrap_or(&run.status);
                let status_cell = Cell::from(format!("{} {}", icon, status_text))
                    .style(Style::default().fg(status_color));

                let branch_display = run.branch.as_deref().unwrap_or("-");
                let started = format_time(&run.started_at);

                let style = if i == selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    status_cell,
                    Cell::from(truncate(&run.workflow_name, 25)),
                    Cell::from(truncate(branch_display, 15)),
                    Cell::from(started),
                ])
                .style(style)
            })
            .collect()
    };

    let widths = [
        Constraint::Length(14),
        Constraint::Percentage(40),
        Constraint::Percentage(25),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    f.render_widget(table, area);
}

fn render_compact(
    f: &mut Frame,
    area: Rect,
    block: Block,
    runs: &[ActionRun],
    selected: usize,
    loading: bool,
) {
    // Compact mode: just status icon and workflow name
    let items: Vec<ListItem> = if loading && runs.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else if runs.is_empty() {
        vec![ListItem::new("No runs").style(Style::default().fg(Color::DarkGray))]
    } else {
        runs.iter()
            .map(|run| {
                let (icon, color) = status_style(run);
                let text = format!("{} {}", icon, truncate(&run.workflow_name, 20));
                ListItem::new(text).style(Style::default().fg(color))
            })
            .collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if !runs.is_empty() {
        state.select(Some(selected));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn status_style(run: &ActionRun) -> (&'static str, Color) {
    match run.status.as_str() {
        "completed" | "success" => ("✓", Color::Green),
        "failure" | "failed" => ("✗", Color::Red),
        "in_progress" | "queued" | "pending" | "waiting" => ("●", Color::Yellow),
        "cancelled" => ("○", Color::DarkGray),
        _ => match run.conclusion.as_deref() {
            Some("success") => ("✓", Color::Green),
            Some("failure") => ("✗", Color::Red),
            Some("cancelled") => ("○", Color::DarkGray),
            _ => ("?", Color::Gray),
        },
    }
}

fn format_time(started_at: &Option<String>) -> String {
    started_at
        .as_deref()
        .and_then(|s| s.split('T').nth(1))
        .and_then(|t| t.split('.').next())
        .map(|t| t[..5].to_string()) // HH:MM
        .unwrap_or_else(|| "-".to_string())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
