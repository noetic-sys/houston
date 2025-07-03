use ratatui::{prelude::*, widgets::*};
use houston_api::{Action, Tag};

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

pub struct ActionsPanel<'a> {
    pub actions: &'a [crate::Action],
    pub selected: usize,
}

impl<'a> ActionsPanel<'a> {
    pub fn new(actions: &'a [crate::Action], selected: usize) -> Self {
        Self { actions, selected }
    }
}

pub fn render_actions_panel(
    f: &mut Frame,
    area: Rect,
    panel: &ActionsPanel,
    state: &mut ListState,
    loading: bool,
    border_style: Style,
) {
    let items: Vec<ListItem> = if loading && panel.actions.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else {
        panel
            .actions
            .iter()
            .map(|a| {
                let text = if let Some(desc) = &a.description {
                    format!("{} - {}", a.name, desc)
                } else {
                    a.name.clone()
                };
                ListItem::new(text)
            })
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions").border_style(border_style))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    if !panel.actions.is_empty() {
        state.select(Some(panel.selected));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(list, area, state);
}

pub struct TagsPanel<'a> {
    pub tags: &'a [Tag],
    pub selected: usize,
}

impl<'a> TagsPanel<'a> {
    pub fn new(tags: &'a [Tag], selected: usize) -> Self {
        Self { tags, selected }
    }
}

pub fn render_tags_panel(
    f: &mut Frame,
    area: Rect,
    panel: &TagsPanel,
    state: &mut ListState,
    loading: bool,
    border_style: Style,
) {
    let items: Vec<ListItem> = if loading && panel.tags.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else {
        panel
            .tags
            .iter()
            .map(|t| ListItem::new(t.name.as_str()))
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tags").border_style(border_style))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    if !panel.tags.is_empty() {
        state.select(Some(panel.selected));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(list, area, state);
}