use houston_ui::{
    render_actions_panel, render_tags_panel, render_repo_list_panel_with_title_and_style,
    render_dialog, render_header, render_footer, render_runs_view, render_log_viewer,
    RepoListPanel, ActionsPanel, TagsPanel, FocusedPanel, View,
};
use houston_core::AppState;
use ratatui::prelude::*;

pub fn render_ui(f: &mut Frame, app: &mut AppState, search_mode: bool) {
    // If log viewer is open, render it fullscreen
    if let Some(log_viewer) = &app.log_viewer {
        render_log_viewer(f, log_viewer);
        return;
    }
    // Main layout: header, content, footer
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(1),     // Content
            Constraint::Length(1),  // Footer
        ])
        .split(f.area());

    let header_area = main_layout[0];
    let content_area = main_layout[1];
    let footer_area = main_layout[2];

    // Get current repo name for header
    let current_repo = app.filtered_repos.get(app.selected_repo).map(|s| s.as_str());

    // Check if anything is loading
    let is_loading = app.tags_loading || app.actions_loading || app.branches_loading || app.runs_loading;

    // Render header
    render_header(f, header_area, current_repo, app.current_view, is_loading);

    // Render footer (with notification if present)
    render_footer(f, footer_area, app.current_view, app.notification.as_deref());

    // Render content based on current view
    match app.current_view {
        View::Repos => render_repos_view(f, content_area, app, search_mode),
        View::Workflows => render_workflows_view(f, content_area, app),
        View::Runs => render_runs_view(f, content_area, &app.runs, app.selected_run, app.runs_loading),
        View::Envs => render_envs_view(f, content_area, app),
        View::Tags => render_tags_view(f, content_area, app),
    }

    // Render dialog modal if open (on top of everything)
    if let Some(dialog) = &app.dialog {
        render_dialog(f, dialog);
    }
}

/// Original 3-panel repos view
fn render_repos_view(f: &mut Frame, area: Rect, app: &mut AppState, search_mode: bool) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Repo list title with search info
    let repo_title = if search_mode {
        format!("Repositories (search: {})", app.search_query)
    } else if !app.search_query.is_empty() {
        format!("Repositories (filtered: {})", app.search_query)
    } else {
        "Repositories".to_string()
    };

    // Highlight style for focused panel
    let highlight = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let normal = Style::default();

    // Render repo list panel (left)
    let panel = RepoListPanel::new(&app.filtered_repos, app.selected_repo);
    let repo_loading = app.filtered_repos.is_empty() && (app.tags_loading || app.actions_loading);
    render_repo_list_panel_with_title_and_style(
        f,
        chunks[0],
        &panel,
        &mut app.repo_list_state,
        &repo_title,
        if app.focused_panel == FocusedPanel::Repos { highlight } else { normal },
        repo_loading,
    );

    // Render actions panel (middle)
    let actions_panel = ActionsPanel::new(&app.actions, app.selected_action);
    render_actions_panel(
        f,
        chunks[1],
        &actions_panel,
        &mut app.actions_list_state,
        app.actions_loading,
        if app.focused_panel == FocusedPanel::Actions { highlight } else { normal },
    );

    // Render tags panel (right)
    let tags_panel = TagsPanel::new(&app.tags, app.selected_tag);
    render_tags_panel(
        f,
        chunks[2],
        &tags_panel,
        &mut app.tags_list_state,
        app.tags_loading,
        if app.focused_panel == FocusedPanel::Tags { highlight } else { normal },
    );
}

/// Workflows view - full width list of workflows
fn render_workflows_view(f: &mut Frame, area: Rect, app: &mut AppState) {
    let highlight = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let actions_panel = ActionsPanel::new(&app.actions, app.selected_action);
    render_actions_panel(
        f,
        area,
        &actions_panel,
        &mut app.actions_list_state,
        app.actions_loading,
        highlight,
    );
}

/// Environments/Deployments view - grouped by environment
fn render_envs_view(f: &mut Frame, area: Rect, app: &mut AppState) {
    use ratatui::widgets::{Block, Borders, Paragraph};
    use std::collections::BTreeMap;

    // Group deployments by environment (BTreeMap for sorted keys)
    let mut env_map: BTreeMap<String, Vec<&houston_api::DeploymentInfo>> = BTreeMap::new();
    for deployment in &app.deployments {
        env_map.entry(deployment.environment.clone())
            .or_default()
            .push(deployment);
    }

    // Build lines grouped by environment
    let mut lines: Vec<Line> = Vec::new();

    if app.deployments_loading && app.deployments.is_empty() {
        lines.push(Line::from(Span::styled("Loading deployments...", Style::default().fg(Color::DarkGray))));
    } else if app.deployments.is_empty() {
        lines.push(Line::from(Span::styled("No deployments found", Style::default().fg(Color::DarkGray))));
    } else {
        // Sort environments: production first, then staging, then others alphabetically
        let mut env_order: Vec<_> = env_map.keys().cloned().collect();
        env_order.sort_by(|a, b| {
            let priority = |s: &str| -> i32 {
                let lower = s.to_lowercase();
                if lower.contains("prod") { 0 }
                else if lower.contains("stag") { 1 }
                else if lower.contains("dev") { 2 }
                else { 3 }
            };
            priority(a).cmp(&priority(b)).then(a.cmp(b))
        });

        let mut global_idx = 0;
        for env_name in env_order {
            if let Some(deployments) = env_map.get(&env_name) {
                // Environment header
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("━━━ {} ", env_name.to_uppercase()),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        format!("({} deployments) ", deployments.len()),
                        Style::default().fg(Color::DarkGray)
                    ),
                    Span::styled(
                        "━".repeat(40),
                        Style::default().fg(Color::DarkGray)
                    ),
                ]));

                // Show latest deployment prominently
                if let Some(latest) = deployments.first() {
                    let (icon, color) = match latest.status.as_str() {
                        "success" => ("✓", Color::Green),
                        "failure" | "error" => ("✗", Color::Red),
                        "pending" | "queued" | "in_progress" => ("●", Color::Yellow),
                        "inactive" => ("○", Color::DarkGray),
                        _ => ("?", Color::Gray),
                    };

                    // Format ref - show tag-like refs nicely
                    let ref_display = if latest.ref_name.starts_with("refs/tags/") {
                        format!("🏷 {}", latest.ref_name.trim_start_matches("refs/tags/"))
                    } else if latest.ref_name.contains('/') {
                        latest.ref_name.split('/').last().unwrap_or(&latest.ref_name).to_string()
                    } else {
                        latest.ref_name.clone()
                    };

                    let is_selected = global_idx == app.selected_deployment;
                    let row_style = if is_selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    let prefix = if is_selected { "▶ " } else { "  " };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, row_style),
                        Span::styled(format!("{} ", icon), Style::default().fg(color)),
                        Span::styled(format!("{:<12}", latest.status), Style::default().fg(color)),
                        Span::styled(format!("{:<20}", ref_display), Style::default().fg(Color::Yellow)),
                        Span::styled(format!("{} ", latest.sha), Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{} by {}",
                                latest.created_at.split('T').next().unwrap_or(&latest.created_at),
                                latest.creator
                            ),
                            Style::default().fg(Color::White)
                        ),
                    ]).style(row_style));
                    global_idx += 1;
                }

                // Show older deployments (compact)
                for deployment in deployments.iter().skip(1).take(2) {
                    let (icon, color) = match deployment.status.as_str() {
                        "success" => ("✓", Color::Green),
                        "failure" | "error" => ("✗", Color::Red),
                        "inactive" => ("○", Color::DarkGray),
                        _ => ("●", Color::Yellow),
                    };

                    let ref_display = if deployment.ref_name.starts_with("refs/tags/") {
                        format!("🏷 {}", deployment.ref_name.trim_start_matches("refs/tags/"))
                    } else {
                        deployment.ref_name.split('/').last().unwrap_or(&deployment.ref_name).to_string()
                    };

                    let is_selected = global_idx == app.selected_deployment;
                    let row_style = if is_selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    let prefix = if is_selected { "▶ " } else { "  " };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, row_style),
                        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                        Span::styled(format!("{:<10}", deployment.status), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:<18}", ref_display), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{} ", deployment.sha), Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            deployment.created_at.split('T').next().unwrap_or(&deployment.created_at),
                            Style::default().fg(Color::DarkGray)
                        ),
                    ]).style(row_style));
                    global_idx += 1;
                }

                if deployments.len() > 3 {
                    lines.push(Line::from(Span::styled(
                        format!("    ... and {} more", deployments.len() - 3),
                        Style::default().fg(Color::DarkGray)
                    )));
                }

                lines.push(Line::from("")); // Spacing between environments
            }
        }
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Deployments by Environment"));
    f.render_widget(content, area);
}

/// Tags view - full width list of tags
fn render_tags_view(f: &mut Frame, area: Rect, app: &mut AppState) {
    let highlight = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let tags_panel = TagsPanel::new(&app.tags, app.selected_tag);
    render_tags_panel(
        f,
        area,
        &tags_panel,
        &mut app.tags_list_state,
        app.tags_loading,
        highlight,
    );
}
