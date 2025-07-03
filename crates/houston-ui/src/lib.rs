use ratatui::{prelude::*, widgets::*};

pub struct RepoListPanel<'a> {
    pub repos: &'a [String],
    pub selected: usize,
}

impl<'a> RepoListPanel<'a> {
    pub fn new(repos: &'a [String], selected: usize) -> Self {
        Self { repos, selected }
    }
}

pub fn render_repo_list_panel(
    f: &mut Frame,
    area: Rect,
    panel: &RepoListPanel,
    state: &mut ListState,
) {
    let items: Vec<ListItem> = panel
        .repos
        .iter()
        .map(|r| ListItem::new(r.as_str()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Repositories"))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    state.select(Some(panel.selected));
    f.render_stateful_widget(list, area, state);
}