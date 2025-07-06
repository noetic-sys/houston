use houston_ui::{render_actions_panel, render_tags_panel, render_repo_list_panel_with_title_and_style, render_dialog};
use houston_ui::{RepoListPanel, ActionsPanel, TagsPanel, FocusedPanel};
use houston_core::AppState;
use ratatui::{prelude::*, widgets::{Block, Borders}};

pub fn render_ui(f: &mut Frame, app: &mut AppState, search_mode: bool) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ].as_ref())
        .split(f.area());

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2), // Notification bar
        ].as_ref())
        .split(f.area());

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
        main_chunks[0],
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
        main_chunks[1],
        &actions_panel,
        &mut app.actions_list_state,
        app.actions_loading,
        if app.focused_panel == FocusedPanel::Actions { highlight } else { normal },
    );

    // Render tags panel (right)
    let tags_panel = TagsPanel::new(&app.tags, app.selected_tag);
    render_tags_panel(
        f,
        main_chunks[2],
        &tags_panel,
        &mut app.tags_list_state,
        app.tags_loading,
        if app.focused_panel == FocusedPanel::Tags { highlight } else { normal },
    );

    // Render notification bar at the bottom
    if let Some(msg) = &app.notification {
        let block = Block::default()
            .title(msg.as_str())
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Red));
        f.render_widget(block, outer_chunks[1]);
    }

    // Render dialog modal if open
    if let Some(dialog) = &app.dialog {
        render_dialog(f, dialog);
    }
} 