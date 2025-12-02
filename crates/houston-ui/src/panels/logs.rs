//! Logs panel - displays job and step details for a run.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;
use crate::LogViewerState;

/// Renders the logs panel with job and step information.
pub fn render_logs_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    log_viewer: Option<&LogViewerState>,
) {
    let title = if ctx.zoomed {
        " Logs [ZOOMED - z to exit] "
    } else {
        " Logs "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    match log_viewer {
        Some(state) => render_log_content(f, ctx.area, block, state),
        None => render_placeholder(f, ctx.area, block),
    }
}

fn render_placeholder(f: &mut Frame, area: Rect, block: Block) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Select a run to view logs",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter on a run in the Runs panel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_log_content(f: &mut Frame, area: Rect, block: Block, state: &LogViewerState) {
    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Header with run info
    lines.push(Line::from(vec![
        Span::styled("Workflow: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &state.workflow_name,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Run ID: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&state.run_id),
    ]));
    lines.push(Line::from(""));

    if state.loading && state.jobs.is_empty() {
        lines.push(Line::from(Span::styled(
            "Loading jobs...",
            Style::default().fg(Color::Yellow),
        )));
    } else if state.jobs.is_empty() {
        lines.push(Line::from(Span::styled(
            "No jobs found",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for job in &state.jobs {
            // Job header
            let (job_icon, job_color) = job_status_style(&job.status, job.conclusion.as_deref());

            lines.push(Line::from(vec![
                Span::styled(format!("{} ", job_icon), Style::default().fg(job_color)),
                Span::styled(
                    &job.name,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ({})", job.conclusion.as_deref().unwrap_or(&job.status)),
                    Style::default().fg(job_color),
                ),
            ]));

            // Steps
            for step in &job.steps {
                let (icon, color) = step_status_style(&step.status, step.conclusion.as_deref());

                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                    Span::styled(&step.name, Style::default().fg(Color::White)),
                ]));
            }

            lines.push(Line::from("")); // Blank line between jobs
        }
    }

    // Apply scroll offset
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(state.scroll_offset)
        .collect();

    let paragraph = Paragraph::new(visible_lines).block(block);

    f.render_widget(paragraph, area);
}

fn job_status_style(status: &str, conclusion: Option<&str>) -> (&'static str, Color) {
    match conclusion {
        Some("success") => ("✓", Color::Green),
        Some("failure") => ("✗", Color::Red),
        Some("cancelled") => ("○", Color::DarkGray),
        Some("skipped") => ("⊘", Color::DarkGray),
        _ => match status {
            "completed" => ("✓", Color::Green),
            "in_progress" | "queued" => ("●", Color::Yellow),
            _ => ("?", Color::Gray),
        },
    }
}

fn step_status_style(status: &str, conclusion: Option<&str>) -> (&'static str, Color) {
    match conclusion {
        Some("success") => ("✓", Color::Green),
        Some("failure") => ("✗", Color::Red),
        Some("cancelled") => ("○", Color::DarkGray),
        Some("skipped") => ("⊘", Color::DarkGray),
        _ => match status {
            "completed" => ("✓", Color::Green),
            "in_progress" | "queued" => ("●", Color::Yellow),
            _ => ("○", Color::DarkGray),
        },
    }
}
