use houston_ui::{
    render_dialog, render_header, render_footer, render_log_viewer,
    HeaderStats, PanelId,
    panels::{
        render_runs_panel, render_logs_panel, render_repos_panel,
        render_workflows_panel, render_deployments_panel, render_tags_panel,
    },
};
use houston_core::AppState;
use ratatui::prelude::*;

pub fn render_ui(f: &mut Frame, app: &mut AppState, search_mode: bool) {
    // Check if we're in zoomed mode for the log viewer
    // If zoomed on Logs panel AND log_viewer exists, render full-screen
    if app.window_manager.zoomed() == Some(PanelId::Logs) {
        if let Some(log_viewer) = &app.log_viewer {
            render_log_viewer(f, log_viewer);
            return;
        }
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
    let is_loading = app.tags_loading || app.actions_loading || app.branches_loading
        || app.runs_loading || app.deployments_loading;

    // Calculate stats for header
    let stats = HeaderStats {
        repo_count: app.filtered_repos.len(),
        workflow_count: app.actions.len(),
        runs_success: app.runs.iter().filter(|r| r.status == "completed" || r.conclusion.as_deref() == Some("success")).count(),
        runs_failed: app.runs.iter().filter(|r| r.status == "failure" || r.conclusion.as_deref() == Some("failure")).count(),
        runs_pending: app.runs.iter().filter(|r| matches!(r.status.as_str(), "in_progress" | "queued" | "pending" | "waiting")).count(),
        deployments_count: app.deployments.len(),
    };

    // Render header
    render_header(f, header_area, current_repo, app.window_manager.current_preset(), is_loading, &stats);

    // Render footer
    render_footer(f, footer_area, app.window_manager.current_preset(), app.notification.as_deref(), false);

    // Render content using window manager
    render_panels(f, content_area, app, search_mode);

    // Render dialog modal if open (on top of everything)
    if let Some(dialog) = &app.dialog {
        render_dialog(f, dialog);
    }
}

/// Render all visible panels using the window manager
fn render_panels(f: &mut Frame, area: Rect, app: &mut AppState, search_mode: bool) {
    // Get panel rectangles from window manager
    let panel_rects = app.window_manager.compute_panel_rects(area);

    for (panel_id, rect) in panel_rects {
        let ctx = app.window_manager.panel_context(panel_id, rect);

        match panel_id {
            PanelId::Repos => {
                render_repos_panel(
                    f,
                    &ctx,
                    &app.filtered_repos,
                    app.selected_repo,
                    &mut app.repo_list_state,
                    if search_mode { &app.search_query } else { "" },
                );
            }
            PanelId::Workflows => {
                render_workflows_panel(
                    f,
                    &ctx,
                    &app.actions,
                    app.selected_action,
                    &mut app.actions_list_state,
                    app.actions_loading,
                );
            }
            PanelId::Runs => {
                render_runs_panel(
                    f,
                    &ctx,
                    &app.runs,
                    app.selected_run,
                    app.runs_loading,
                );
            }
            PanelId::Logs => {
                render_logs_panel(
                    f,
                    &ctx,
                    app.log_viewer.as_ref(),
                );
            }
            PanelId::Deployments => {
                render_deployments_panel(
                    f,
                    &ctx,
                    &app.deployments,
                    app.selected_deployment,
                    app.deployments_loading,
                );
            }
            PanelId::Tags => {
                render_tags_panel(
                    f,
                    &ctx,
                    &app.tags,
                    app.selected_tag,
                    &mut app.tags_list_state,
                    app.tags_loading,
                );
            }
        }
    }
}
