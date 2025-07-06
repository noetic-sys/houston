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
            }
        }

        // Always load actions/tags when repo changes
        let current_repo_name = app.filtered_repos.get(app.selected_repo).cloned();
        if current_repo_name != last_repo_name {
            if let Some(repo_name) = &current_repo_name {
                app.start_loading_tags();
                app.start_loading_actions();
                let tx_tags = tx.clone();
                let tx_actions = tx.clone();
                let provider_tags = app.provider.clone();
                let provider_actions = app.provider.clone();
                let repo_name_tags = repo_name.clone();
                let repo_name_actions = repo_name.clone();
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
            }
            last_repo_name = current_repo_name;
        }

        terminal.draw(|f| render_ui(f, app, search_mode))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if handle_key_event(key, app, &mut search_mode).await? {
                    return Ok(());
                }
            }
        }
    }
}
