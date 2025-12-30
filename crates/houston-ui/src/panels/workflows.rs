//! Workflows panel - displays the actions/workflows list.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::panel::PanelContext;
use houston_api::Action;

/// Renders the workflows panel with the actions list.
pub fn render_workflows_panel(
    f: &mut Frame,
    ctx: &PanelContext,
    actions: &[Action],
    selected: usize,
    list_state: &mut ListState,
    loading: bool,
) {
    let title = if ctx.zoomed {
        " Workflows [ZOOMED - z to exit] "
    } else {
        " Workflows "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ctx.title_style())
        .border_style(ctx.border_style());

    let items: Vec<ListItem> = if loading && actions.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else if actions.is_empty() {
        vec![ListItem::new("No workflows").style(Style::default().fg(Color::DarkGray))]
    } else {
        actions
            .iter()
            .map(|a| {
                let text = if let Some(desc) = &a.description {
                    if ctx.is_compact() {
                        a.name.clone()
                    } else {
                        format!("{} - {}", a.name, desc)
                    }
                } else {
                    a.name.clone()
                };
                ListItem::new(text)
            })
            .collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

    if !actions.is_empty() {
        list_state.select(Some(selected));
    } else {
        list_state.select(None);
    }

    f.render_stateful_widget(list, ctx.area, list_state);
}
