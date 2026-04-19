use houston_api::{GitHubProvider, VcsProvider};
use houston_core::{AppMsg, AppState};
use ratatui::prelude::*;
use std::io;
use tokio::sync::mpsc;

mod events;
mod ui;

use events::handle_key_event;
use ui::render_ui;

fn get_github_token() -> Option<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Some(token.trim().to_string());
        }
    }
    std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|t| t.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_github_token();
    let provider = GitHubProvider::new(token, None, None);

    let repos = provider
        .list_repos()
        .await
        .map(|repos| repos.into_iter().map(|r| r.name).collect())
        .unwrap_or_else(|e| {
            eprintln!("GitHub API error: {}. Using mock data.", e);
            vec!["houston".into(), "test-repo".into()]
        });

    let mut app = AppState::new(repos, provider);

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run_app(&mut terminal, &mut app).await;

    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    result.map_err(Into::into)
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    let mut search_mode = false;
    let mut last_repo: Option<String> = None;
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMsg>();

    loop {
        app.maybe_clear_notification();

        while let Ok(msg) = rx.try_recv() {
            app.handle_message(msg);
        }

        // Load all data when repo changes
        let current_repo = app.filtered_repos.get(app.selected_repo).cloned();
        if current_repo != last_repo {
            if let Some(repo) = current_repo.clone() {
                app.start_all_loading();
                spawn_data_loaders(&tx, &app.provider, &repo);
            }
            last_repo = current_repo;
        }

        // Auto-refresh runs when viewing them
        if app.should_refresh_runs() {
            if let Some(repo) = app.filtered_repos.get(app.selected_repo).cloned() {
                app.start_loading_runs();
                app.mark_runs_refreshed();
                let (tx, p) = (tx.clone(), app.provider.clone());
                tokio::spawn(async move {
                    let _ = tx.send(match p.list_action_runs(&repo).await {
                        Ok(d) => AppMsg::RunsLoaded(d),
                        Err(e) => AppMsg::RunsFailed(e.to_string()),
                    });
                });
            }
        }

        terminal.draw(|f| render_ui(f, app, search_mode))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                let (quit, load_jobs) = handle_key_event(key, app, &mut search_mode).await?;
                if quit {
                    return Ok(());
                }

                if load_jobs {
                    if let (Some(v), Some(repo)) = (
                        &app.log_viewer,
                        app.filtered_repos.get(app.selected_repo).cloned(),
                    ) {
                        let (tx, p, run_id) = (tx.clone(), app.provider.clone(), v.run_id.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(match p.get_run_jobs(&repo, &run_id).await {
                                Ok(d) => AppMsg::JobsLoaded(d),
                                Err(e) => AppMsg::JobsFailed(e.to_string()),
                            });
                        });
                    }
                }
            }
        }
    }
}

fn spawn_data_loaders(tx: &mpsc::UnboundedSender<AppMsg>, provider: &GitHubProvider, repo: &str) {
    let repo = repo.to_string();

    macro_rules! spawn {
        ($p:expr, $r:expr, $method:ident, $ok:ident, $err:ident) => {{
            let (tx, p, r) = (tx.clone(), $p.clone(), $r.clone());
            tokio::spawn(async move {
                let _ = tx.send(match p.$method(&r).await {
                    Ok(d) => AppMsg::$ok(d),
                    Err(e) => AppMsg::$err(e.to_string()),
                });
            });
        }};
    }

    spawn!(provider, repo, list_tags, TagsLoaded, TagsFailed);
    spawn!(provider, repo, list_actions, ActionsLoaded, ActionsFailed);
    spawn!(
        provider,
        repo,
        list_branches,
        BranchesLoaded,
        BranchesFailed
    );
    spawn!(provider, repo, list_action_runs, RunsLoaded, RunsFailed);
    spawn!(
        provider,
        repo,
        list_deployments,
        DeploymentsLoaded,
        DeploymentsFailed
    );
}
