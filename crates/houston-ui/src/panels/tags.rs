//! Tags panel - displays release tags.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;
use houston_api::Tag;

/// Renders the tags panel with the tag list.
pub fn render_tags_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    tags: &[Tag],
    selected: usize,
    list_state: &mut ListState,
    loading: bool,
) {
    let title = if ctx.zoomed {
        " Tags [ZOOMED - z to exit] "
    } else {
        " Tags "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    let items: Vec<ListItem> = if loading && tags.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else if tags.is_empty() {
        vec![ListItem::new("No tags").style(Style::default().fg(Color::DarkGray))]
    } else {
        tags.iter()
            .map(|t| ListItem::new(t.name.as_str()))
            .collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    if !tags.is_empty() {
        list_state.select(Some(selected));
    } else {
        list_state.select(None);
    }

    f.render_stateful_widget(list, ctx.area, list_state);
}
