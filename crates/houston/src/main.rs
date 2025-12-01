use houston_api::{GitHubProvider, VcsProvider};
use houston_core::{AppState, AppMsg};
use ratatui::prelude::*;
use std::io;
use tokio::sync::mpsc;

mod ui;
mod events;

use ui::*;
use events::*;

fn get_github_token() -> Option<String> {
    std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|token| token.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to get GitHub token from gh CLI, fall back to None
    let token = get_github_token();
    let provider = GitHubProvider::new(token, None, None);
    
    // Try to fetch repos, fall back to mock data if authentication fails
    let repos = match provider.list_repos().await {
        Ok(repos) => repos.into_iter().map(|r| r.name).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("GitHub API error: {}. Using mock data for testing.", e);
            vec![
                "houston".to_string(),
                "test-repo-1".to_string(),
                "test-repo-2".to_string(),
                "awesome-project".to_string(),
                "backend-api".to_string(),
            ]
        }
    };
    
    let mut app = AppState::new(repos, provider);
    app.load_actions_for_selected_repo().await;

    // Terminal setup
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Enable raw mode for input handling
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    let mut search_mode = false;
    let mut last_repo_name: Option<String> = app.filtered_repos.get(app.selected_repo).cloned();
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMsg>();

    loop {
        app.maybe_clear_notification();

        // Check for background results
        while let Ok(msg) = rx.try_recv() {
            match msg {
                AppMsg::TagsLoaded(tags) => app.finish_loading_tags(tags),
                AppMsg::TagsFailed(e) => app.fail_loading_tags(e),
                AppMsg::ActionsLoaded(actions) => app.finish_loading_actions(actions),
                AppMsg::ActionsFailed(e) => app.fail_loading_actions(e),
                AppMsg::BranchesLoaded(branches) => app.finish_loading_branches(branches),
                AppMsg::BranchesFailed(e) => app.fail_loading_branches(e),
                AppMsg::RunsLoaded(runs) => app.finish_loading_runs(runs),
                AppMsg::RunsFailed(e) => app.fail_loading_runs(e),
                AppMsg::JobsLoaded(jobs) => app.finish_loading_jobs(jobs),
                AppMsg::JobsFailed(e) => app.fail_loading_jobs(e),
                AppMsg::DeploymentsLoaded(deployments) => app.finish_loading_deployments(deployments),
                AppMsg::DeploymentsFailed(e) => app.fail_loading_deployments(e),
            }
        }

        // Always load actions/tags/branches/runs/deployments when repo changes
        let current_repo_name = app.filtered_repos.get(app.selected_repo).cloned();
        if current_repo_name != last_repo_name {
            if let Some(repo_name) = &current_repo_name {
                app.start_loading_tags();
                app.start_loading_actions();
                app.start_loading_branches();
                app.start_loading_runs();
                app.start_loading_deployments();

                let tx_tags = tx.clone();
                let tx_actions = tx.clone();
                let tx_branches = tx.clone();
                let tx_runs = tx.clone();
                let tx_deployments = tx.clone();

                let provider_tags = app.provider.clone();
                let provider_actions = app.provider.clone();
                let provider_branches = app.provider.clone();
                let provider_runs = app.provider.clone();
                let provider_deployments = app.provider.clone();

                let repo_name_tags = repo_name.clone();
                let repo_name_actions = repo_name.clone();
                let repo_name_branches = repo_name.clone();
                let repo_name_runs = repo_name.clone();
                let repo_name_deployments = repo_name.clone();

                tokio::spawn(async move {
                    match provider_tags.list_tags(&repo_name_tags).await {
                        Ok(tags) => tx_tags.send(AppMsg::TagsLoaded(tags)).ok(),
                        Err(e) => tx_tags.send(AppMsg::TagsFailed(format!("Failed to load tags: {}", e))).ok(),
                    };
                });
                tokio::spawn(async move {
                    match provider_actions.list_actions(&repo_name_actions).await {
                        Ok(actions) => tx_actions.send(AppMsg::ActionsLoaded(actions)).ok(),
                        Err(e) => tx_actions.send(AppMsg::ActionsFailed(format!("Failed to load actions: {}", e))).ok(),
                    };
                });
                tokio::spawn(async move {
                    match provider_branches.list_branches(&repo_name_branches).await {
                        Ok(branches) => tx_branches.send(AppMsg::BranchesLoaded(branches)).ok(),
                        Err(e) => tx_branches.send(AppMsg::BranchesFailed(format!("Failed to load branches: {}", e))).ok(),
                    };
                });
                tokio::spawn(async move {
                    match provider_runs.list_action_runs(&repo_name_runs).await {
                        Ok(runs) => tx_runs.send(AppMsg::RunsLoaded(runs)).ok(),
                        Err(e) => tx_runs.send(AppMsg::RunsFailed(format!("Failed to load runs: {}", e))).ok(),
                    };
                });
                tokio::spawn(async move {
                    match provider_deployments.list_deployments(&repo_name_deployments).await {
                        Ok(deployments) => tx_deployments.send(AppMsg::DeploymentsLoaded(deployments)).ok(),
                        Err(e) => tx_deployments.send(AppMsg::DeploymentsFailed(format!("Failed to load deployments: {}", e))).ok(),
                    };
                });
            }
            last_repo_name = current_repo_name;
        }

        // Auto-refresh runs when in Runs view
        if app.should_refresh_runs() {
            if let Some(repo_name) = app.filtered_repos.get(app.selected_repo).cloned() {
                app.start_loading_runs();
                app.mark_runs_refreshed();

                let tx_runs = tx.clone();
                let provider_runs = app.provider.clone();

                tokio::spawn(async move {
                    match provider_runs.list_action_runs(&repo_name).await {
                        Ok(runs) => tx_runs.send(AppMsg::RunsLoaded(runs)).ok(),
                        Err(e) => tx_runs.send(AppMsg::RunsFailed(format!("Failed to load runs: {}", e))).ok(),
                    };
                });
            }
        }

        terminal.draw(|f| render_ui(f, app, search_mode))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                let (should_quit, should_load_logs) = handle_key_event(key, app, &mut search_mode).await?;

                if should_quit {
                    return Ok(());
                }

                // Load jobs if requested (for log viewer)
                if should_load_logs {
                    if let Some(viewer) = &app.log_viewer {
                        let run_id = viewer.run_id.clone();
                        if let Some(repo_name) = app.filtered_repos.get(app.selected_repo).cloned() {
                            let tx_jobs = tx.clone();
                            let provider = app.provider.clone();

                            tokio::spawn(async move {
                                match provider.get_run_jobs(&repo_name, &run_id).await {
                                    Ok(jobs) => { tx_jobs.send(AppMsg::JobsLoaded(jobs)).ok(); }
                                    Err(e) => { tx_jobs.send(AppMsg::JobsFailed(format!("Failed to get jobs: {}", e))).ok(); }
                                }
                            });
                        }
                    }
                }
            }
        }
    }
}
