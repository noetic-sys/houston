use houston_api::{GitHubProvider, VcsProvider, Action};
use houston_ui::{RepoListPanel, render_repo_list_panel, ActionsPanel, render_actions_panel};
use ratatui::{prelude::*, widgets::{ListState, Paragraph, Block, Borders, ListItem, List}};
use std::io;
use std::time::{Duration, Instant};

struct AppState {
    repos: Vec<String>,
    filtered_repos: Vec<String>,
    search_query: String,
    selected_repo: usize,
    repo_list_state: ListState,
    actions: Vec<Action>,
    selected_action: usize,
    actions_list_state: ListState,
    provider: GitHubProvider,
    needs_action_reload: bool,
    last_action_load: Option<Instant>,
    action_load_debounce: Duration,
    notification: Option<String>,
    notification_time: Option<Instant>,
}

impl AppState {
    fn new(repos: Vec<String>, provider: GitHubProvider) -> Self {
        let mut repo_list_state = ListState::default();
        let selected_repo = 0;
        let filtered_repos = repos.clone();
        if !filtered_repos.is_empty() {
            repo_list_state.select(Some(selected_repo));
        }
        Self { 
            repos, 
            filtered_repos,
            search_query: String::new(),
            selected_repo, 
            repo_list_state,
            actions: Vec::new(),
            selected_action: 0,
            actions_list_state: ListState::default(),
            provider,
            needs_action_reload: true,
            last_action_load: None,
            action_load_debounce: Duration::from_millis(500),
            notification: None,
            notification_time: None,
        }
    }

    fn filter_repos(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_repos = self.repos.clone();
        } else {
            self.filtered_repos = self.repos
                .iter()
                .filter(|repo| repo.to_lowercase().contains(&self.search_query.to_lowercase()))
                .cloned()
                .collect();
        }
        
        // Adjust selection if current selection is out of bounds
        if self.selected_repo >= self.filtered_repos.len() {
            self.selected_repo = if self.filtered_repos.is_empty() { 0 } else { self.filtered_repos.len() - 1 };
        }
        
        if !self.filtered_repos.is_empty() {
            self.repo_list_state.select(Some(self.selected_repo));
        } else {
            self.repo_list_state.select(None);
        }
    }

    fn add_to_search(&mut self, c: char) {
        self.search_query.push(c);
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn remove_from_search(&mut self) {
        self.search_query.pop();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn next_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = (self.selected_repo + 1) % self.filtered_repos.len();
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    fn previous_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = if self.selected_repo == 0 {
                self.filtered_repos.len() - 1
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
        if !self.filtered_repos.is_empty() {
            let repo_name = &self.filtered_repos[self.selected_repo];
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
        self.last_action_load = Some(Instant::now());
    }

    fn should_load_actions(&self) -> bool {
        if !self.needs_action_reload {
            return false;
        }
        
        if let Some(last_load) = self.last_action_load {
            if Instant::now().duration_since(last_load) < self.action_load_debounce {
                return false;
            }
        }
        
        true
    }

    fn set_notification(&mut self, msg: impl Into<String>) {
        self.notification = Some(msg.into());
        self.notification_time = Some(Instant::now());
    }

    fn maybe_clear_notification(&mut self) {
        if let Some(time) = self.notification_time {
            if Instant::now().duration_since(time) > Duration::from_secs(3) {
                self.notification = None;
                self.notification_time = None;
            }
        }
    }

    async fn execute_selected_action(&mut self) {
        if !self.filtered_repos.is_empty() && !self.actions.is_empty() {
            let repo_name = &self.filtered_repos[self.selected_repo];
            let action_name = &self.actions[self.selected_action].name;
            match self.provider.execute_action(repo_name, action_name).await {
                Ok(run) => {
                    self.set_notification(format!("Triggered action {} on {}: {}", action_name, repo_name, run.id));
                }
                Err(e) => {
                    self.set_notification(format!("Failed to execute action {} on {}: {}", action_name, repo_name, e));
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
    let mut search_mode = false;
    
    loop {
        app.maybe_clear_notification();
        // Load actions if needed and debounce period has passed
        if app.should_load_actions() {
            app.load_actions_for_selected_repo().await;
        }

        terminal.draw(|f| ui(f, app, search_mode))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    crossterm::event::KeyCode::Char('/') => {
                        search_mode = true;
                        app.clear_search();
                    }
                    crossterm::event::KeyCode::Esc => {
                        search_mode = false;
                        app.clear_search();
                    }
                    crossterm::event::KeyCode::Enter if search_mode => {
                        search_mode = false;
                    }
                    crossterm::event::KeyCode::Char(c) if search_mode => {
                        app.add_to_search(c);
                    }
                    crossterm::event::KeyCode::Backspace if search_mode => {
                        app.remove_from_search();
                    }
                    crossterm::event::KeyCode::Up if !search_mode => app.previous_repo(),
                    crossterm::event::KeyCode::Down if !search_mode => app.next_repo(),
                    crossterm::event::KeyCode::Char('j') if !search_mode => app.next_action(),
                    crossterm::event::KeyCode::Char('k') if !search_mode => app.previous_action(),
                    crossterm::event::KeyCode::Char(' ') if !search_mode => app.execute_selected_action().await,
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut AppState, search_mode: bool) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ].as_ref())
        .split(f.size());

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2), // Notification bar
        ].as_ref())
        .split(f.size());

    // Repo list title with search info
    let repo_title = if search_mode {
        format!("Repositories (search: {})", app.search_query)
    } else if !app.search_query.is_empty() {
        format!("Repositories (filtered: {})", app.search_query)
    } else {
        "Repositories".to_string()
    };

    // Render repo list panel (left)
    let panel = RepoListPanel::new(&app.filtered_repos, app.selected_repo);
    render_repo_list_panel_with_title(f, main_chunks[0], &panel, &mut app.repo_list_state, &repo_title);

    // Render actions panel (right)
    let actions_panel = ActionsPanel::new(&app.actions, app.selected_action);
    render_actions_panel(f, main_chunks[1], &actions_panel, &mut app.actions_list_state);

    // Render notification bar at the bottom
    if let Some(msg) = &app.notification {
        let block = Block::default()
            .title(msg.as_str())
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Red));
        f.render_widget(block, outer_chunks[1]);
    }
}

fn render_repo_list_panel_with_title(
    f: &mut Frame,
    area: Rect,
    panel: &houston_ui::RepoListPanel,
    state: &mut ListState,
    title: &str,
) {
    let items: Vec<ListItem> = panel
        .repos
        .iter()
        .map(|r| ListItem::new(r.as_str()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    state.select(Some(panel.selected));
    f.render_stateful_widget(list, area, state);
}
