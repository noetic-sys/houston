use houston_api::{GitHubProvider, VcsProvider};
use houston_ui::{RepoListPanel, render_repo_list_panel};
use ratatui::{prelude::*, widgets::{ListState, Paragraph, Block, Borders}};
use std::io;

struct AppState {
    repos: Vec<String>,
    selected: usize,
    repo_list_state: ListState,
}

impl AppState {
    fn new(repos: Vec<String>) -> Self {
        let mut repo_list_state = ListState::default();
        let selected = 0;
        if !repos.is_empty() {
            repo_list_state.select(Some(selected));
        }
        Self { repos, selected, repo_list_state }
    }

    fn next(&mut self) {
        if !self.repos.is_empty() {
            self.selected = (self.selected + 1) % self.repos.len();
            self.repo_list_state.select(Some(self.selected));
        }
    }

    fn previous(&mut self) {
        if !self.repos.is_empty() {
            self.selected = if self.selected == 0 {
                self.repos.len() - 1
            } else {
                self.selected - 1
            };
            self.repo_list_state.select(Some(self.selected));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For now, use no token, no org, no user (will use authenticated user)
    let provider = GitHubProvider::new(None, None, None);
    
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
    
    let mut app = AppState::new(repos);

    // Terminal setup
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Enable raw mode for input handling
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    crossterm::event::KeyCode::Up => app.previous(),
                    crossterm::event::KeyCode::Down => app.next(),
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
    let panel = RepoListPanel::new(&app.repos, app.selected);
    render_repo_list_panel(f, chunks[0], &panel, &mut app.repo_list_state);

    // Placeholder for future panels (tags, actions, etc.)
    let placeholder = Paragraph::new("Tags and Actions will go here")
        .block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(placeholder, chunks[1]);
}
