//! Pull requests panel.

use houston_api::PullRequest;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;

pub fn render_pull_requests_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    pull_requests: &[PullRequest],
    selected: usize,
    loading: bool,
) {
    let title = if ctx.zoomed {
        " Pull Requests [ZOOMED - z to exit] "
    } else {
        " Pull Requests "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    if loading && pull_requests.is_empty() {
        let p = Paragraph::new("Loading pull requests...")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(p, ctx.area);
        return;
    }

    if pull_requests.is_empty() {
        let p = Paragraph::new("No open pull requests")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(p, ctx.area);
        return;
    }

    let rows: Vec<Row> = pull_requests
        .iter()
        .enumerate()
        .map(|(i, pr)| {
            let is_selected = i == selected;
            let (icon, color) = pr_icon(pr);

            let selector = if is_selected { "▶" } else { " " };

            let number_cell = Cell::from(format!("{} #{}", selector, pr.number))
                .style(Style::default().fg(Color::DarkGray));

            let title_text = truncate(&pr.title, 45);
            let title_style = if pr.draft {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC)
            } else if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let state_cell = Cell::from(icon).style(Style::default().fg(color));

            let branch_text = truncate(&pr.branch, 20);

            let updated = format_date(&pr.updated_at);

            let row_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                number_cell,
                state_cell,
                Cell::from(title_text).style(title_style),
                Cell::from(truncate(&pr.author, 15)).style(Style::default().fg(Color::Cyan)),
                Cell::from(branch_text).style(Style::default().fg(Color::DarkGray)),
                Cell::from(updated).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("#").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("TITLE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("AUTHOR").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("BRANCH").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("UPDATED").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::Gray));

    let widths = [
        Constraint::Length(8),
        Constraint::Length(2),
        Constraint::Min(20),
        Constraint::Length(15),
        Constraint::Length(20),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths).header(header).block(block);

    f.render_widget(table, ctx.area);
}

fn pr_icon(pr: &PullRequest) -> (&'static str, Color) {
    if pr.draft {
        ("○", Color::DarkGray)
    } else {
        match pr.state.as_str() {
            "open" => ("●", Color::Green),
            "closed" => ("✗", Color::Red),
            _ => ("?", Color::Gray),
        }
    }
}

fn format_date(s: &str) -> String {
    s.split('T').next().unwrap_or("-").to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
