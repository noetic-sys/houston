use houston_api::{GitHubProvider, VcsProvider, Action};
use houston_ui::{RepoListPanel, render_repo_list_panel, ActionsPanel, render_actions_panel};
use ratatui::{prelude::*, widgets::{ListState, Paragraph, Block, Borders}};
use std::io;

struct AppState {
    repos: Vec<String>,
    selected_repo: usize,
    repo_list_state: ListState,
    actions: Vec<Action>,
    selected_action: usize,
    actions_list_state: ListState,
    provider: GitHubProvider,
    needs_action_reload: bool,
}

impl AppState {
    fn new(repos: Vec<String>, provider: GitHubProvider) -> Self {
        let mut repo_list_state = ListState::default();
        let selected_repo = 0;
        if !repos.is_empty() {
            repo_list_state.select(Some(selected_repo));
        }
        Self { 
            repos, 
            selected_repo, 
            repo_list_state,
            actions: Vec::new(),
            selected_action: 0,
            actions_list_state: ListState::default(),
            provider,
            needs_action_reload: true,
        }
    }

    fn next_repo(&mut self) {
        if !self.repos.is_empty() {
            self.selected_repo = (self.selected_repo + 1) % self.repos.len();
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    fn previous_repo(&mut self) {
        if !self.repos.is_empty() {
            self.selected_repo = if self.selected_repo == 0 {
                self.repos.len() - 1
            } else {
                self.selected_repo - 1
            };
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    fn next_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = (self.selected_action + 1) % self.actions.len();
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    fn previous_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = if self.selected_action == 0 {
                self.actions.len() - 1
            } else {
                self.selected_action - 1
            };
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    async fn load_actions_for_selected_repo(&mut self) {
        if !self.repos.is_empty() {
            let repo_name = &self.repos[self.selected_repo];
            match self.provider.list_actions(repo_name).await {
                Ok(actions) => {
                    self.actions = actions;
                    self.selected_action = 0;
                    if !self.actions.is_empty() {
                        self.actions_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load actions for {}: {}", repo_name, e);
                    self.actions = Vec::new();
                }
            }
        }
        self.needs_action_reload = false;
    }

    async fn execute_selected_action(&mut self) {
        if !self.repos.is_empty() && !self.actions.is_empty() {
            let repo_name = &self.repos[self.selected_repo];
            let action_name = &self.actions[self.selected_action].name;
            match self.provider.execute_action(repo_name, action_name).await {
                Ok(run) => {
                    eprintln!("Triggered action {} on {}: {}", action_name, repo_name, run.id);
                }
                Err(e) => {
                    eprintln!("Failed to execute action {} on {}: {}", action_name, repo_name, e);
                }
            }
        }
    }
}

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
    loop {
        // Load actions if needed
        if app.needs_action_reload {
            app.load_actions_for_selected_repo().await;
        }

        terminal.draw(|f| ui(f, app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    crossterm::event::KeyCode::Up => app.previous_repo(),
                    crossterm::event::KeyCode::Down => app.next_repo(),
                    crossterm::event::KeyCode::Char('j') => app.next_action(),
                    crossterm::event::KeyCode::Char('k') => app.previous_action(),
                    crossterm::event::KeyCode::Char(' ') => app.execute_selected_action().await,
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ].as_ref())
        .split(f.area());

    // Render repo list panel
    let panel = RepoListPanel::new(&app.repos, app.selected_repo);
    render_repo_list_panel(f, chunks[0], &panel, &mut app.repo_list_state);

    // Render actions panel
    let actions_panel = ActionsPanel::new(&app.actions, app.selected_action);
    render_actions_panel(f, chunks[1], &actions_panel, &mut app.actions_list_state);
}
