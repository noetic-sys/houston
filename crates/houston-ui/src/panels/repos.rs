//! Repos panel - displays the repository list.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;

/// Renders the repos panel with the repository list.
pub fn render_repos_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    repos: &[String],
    selected: usize,
    list_state: &mut ListState,
    search_query: &str,
    pinned: &std::collections::HashSet<String>,
) {
    let title = if ctx.zoomed {
        " Repositories [ZOOMED - z to exit] ".to_string()
    } else if !search_query.is_empty() {
        format!(" Repositories [/{}] ", search_query)
    } else {
        " Repositories ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    let items: Vec<ListItem> = if repos.is_empty() {
        vec![ListItem::new("No repositories").style(Style::default().fg(Color::DarkGray))]
    } else {
        repos
            .iter()
            .map(|r| {
                if pinned.contains(r.as_str()) {
                    ListItem::new(format!("★ {}", r)).style(Style::default().fg(Color::Yellow))
                } else {
                    ListItem::new(r.as_str()).style(Style::default())
                }
            })
            .collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    if !repos.is_empty() {
        list_state.select(Some(selected));
    } else {
        list_state.select(None);
    }

    f.render_stateful_widget(list, ctx.area, list_state);
}
