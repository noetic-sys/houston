//! Deployments panel - displays deployments grouped by environment.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;
use houston_api::DeploymentInfo;

/// Renders the deployments panel with deployments grouped by environment.
pub fn render_deployments_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    deployments: &[DeploymentInfo],
    selected: usize,
    loading: bool,
) {
    let title = if ctx.zoomed {
        " Deployments [ZOOMED - z to exit] "
    } else {
        " Deployments "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    // Group deployments by environment
    let grouped = group_by_environment(deployments);

    let mut lines: Vec<Line> = Vec::new();

    if loading && deployments.is_empty() {
        lines.push(Line::from(Span::styled(
            "Loading deployments...",
            Style::default().fg(Color::DarkGray),
        )));
    } else if deployments.is_empty() {
        lines.push(Line::from(Span::styled(
            "No deployments found",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let mut flat_index = 0;

        for (env_name, env_deployments) in &grouped {
            // Environment header
            lines.push(Line::from(vec![
                Span::styled("━━━ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    env_name.to_uppercase(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ({}) ", env_deployments.len()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "━".repeat(
                        20.min(ctx.area.width.saturating_sub(env_name.len() as u16 + 15) as usize),
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));

            // Deployments in this environment
            for (i, dep) in env_deployments.iter().enumerate() {
                let is_selected = flat_index == selected;
                let is_latest = i == 0;

                let (icon, color) = deployment_status_style(&dep.status);

                let mut spans = vec![
                    if is_selected {
                        Span::styled("▶ ", Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("  ")
                    },
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                ];

                // Show ref/version
                let ref_display = if dep.ref_name.len() > 15 {
                    format!("{}...", &dep.ref_name[..12])
                } else {
                    dep.ref_name.clone()
                };
                spans.push(Span::styled(
                    ref_display,
                    if is_latest {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ));

                // Show SHA (abbreviated)
                spans.push(Span::styled(
                    format!(" {}", &dep.sha[..7.min(dep.sha.len())]),
                    Style::default().fg(Color::DarkGray),
                ));

                // Show creator if space permits and not compact
                if !ctx.is_compact() && ctx.area.width > 50 {
                    spans.push(Span::styled(
                        format!(" by {}", dep.creator),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                let style = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                lines.push(Line::from(spans).style(style));
                flat_index += 1;
            }

            lines.push(Line::from("")); // Blank line between environments
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, ctx.area);
}

/// Groups deployments by environment, sorted by priority.
fn group_by_environment(deployments: &[DeploymentInfo]) -> Vec<(String, Vec<&DeploymentInfo>)> {
    use std::collections::HashMap;

    let mut grouped: HashMap<String, Vec<&DeploymentInfo>> = HashMap::new();

    for dep in deployments {
        grouped
            .entry(dep.environment.clone())
            .or_default()
            .push(dep);
    }

    // Sort each group by created_at (most recent first)
    for deps in grouped.values_mut() {
        deps.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    }

    // Sort environments by priority
    let mut result: Vec<_> = grouped.into_iter().collect();
    result.sort_by(|a, b| env_priority(&a.0).cmp(&env_priority(&b.0)));

    result
}

fn env_priority(env: &str) -> u8 {
    let env_lower = env.to_lowercase();
    if env_lower.contains("prod") {
        0
    } else if env_lower.contains("stag") {
        1
    } else if env_lower.contains("dev") {
        2
    } else {
        3
    }
}

fn deployment_status_style(status: &str) -> (&'static str, Color) {
    match status.to_lowercase().as_str() {
        "success" | "active" => ("✓", Color::Green),
        "failure" | "failed" | "error" => ("✗", Color::Red),
        "pending" | "in_progress" | "queued" | "waiting" => ("●", Color::Yellow),
        "inactive" => ("○", Color::DarkGray),
        _ => ("?", Color::Gray),
    }
}
