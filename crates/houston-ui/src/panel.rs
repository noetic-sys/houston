//! Panel abstractions for the multi-panel UI system.
//!
//! This module provides the core types for the Bloomberg terminal-style
//! panel-based interface.

use ratatui::prelude::*;
use std::hash::Hash;

/// Unique identifier for each panel type in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    /// Repository list panel
    Repos,
    /// Workflows/Actions list panel
    Workflows,
    /// Action runs table panel
    Runs,
    /// Log viewer panel (jobs & steps)
    Logs,
    /// Deployments panel (grouped by environment)
    Deployments,
    /// Tags list panel
    Tags,
}

impl PanelId {
    /// Returns all panel IDs in display order.
    pub fn all() -> &'static [PanelId] {
        &[
            PanelId::Repos,
            PanelId::Workflows,
            PanelId::Runs,
            PanelId::Logs,
            PanelId::Deployments,
            PanelId::Tags,
        ]
    }

    /// Returns the default title for this panel.
    pub fn title(&self) -> &'static str {
        match self {
            PanelId::Repos => "Repositories",
            PanelId::Workflows => "Workflows",
            PanelId::Runs => "Runs",
            PanelId::Logs => "Logs",
            PanelId::Deployments => "Deployments",
            PanelId::Tags => "Tags",
        }
    }

    /// Returns the toggle key for this panel (if any).
    pub fn toggle_key(&self) -> Option<char> {
        match self {
            PanelId::Repos => Some('r'),
            PanelId::Workflows => Some('w'),
            PanelId::Runs => None, // Runs is always visible when in Runs preset
            PanelId::Logs => Some('l'),
            PanelId::Deployments => Some('d'),
            PanelId::Tags => Some('t'),
        }
    }

    /// Returns the short name for header display.
    pub fn short_name(&self) -> &'static str {
        match self {
            PanelId::Repos => "Repo",
            PanelId::Workflows => "Wkfl",
            PanelId::Runs => "Runs",
            PanelId::Logs => "Logs",
            PanelId::Deployments => "Deps",
            PanelId::Tags => "Tags",
        }
    }
}

/// Context passed to panels during rendering.
#[derive(Debug, Clone, Copy)]
pub struct PanelContext {
    /// The area allocated to this panel.
    pub area: Rect,
    /// Whether this panel currently has focus.
    pub focused: bool,
    /// Whether this panel is zoomed to full screen.
    pub zoomed: bool,
}

impl PanelContext {
    /// Creates a new panel context.
    pub fn new(area: Rect, focused: bool, zoomed: bool) -> Self {
        Self { area, focused, zoomed }
    }

    /// Returns the border style based on focus state.
    pub fn border_style(&self) -> Style {
        if self.zoomed {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else if self.focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    }

    /// Returns the title style based on focus state.
    pub fn title_style(&self) -> Style {
        if self.zoomed {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else if self.focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        }
    }

    /// Returns whether the panel should render in compact mode.
    pub fn is_compact(&self) -> bool {
        self.area.height < 10 || self.area.width < 30
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_id_all() {
        let all = PanelId::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&PanelId::Repos));
        assert!(all.contains(&PanelId::Logs));
    }

    #[test]
    fn test_panel_context_styles() {
        let ctx_focused = PanelContext::new(Rect::default(), true, false);
        assert_eq!(ctx_focused.border_style().fg, Some(Color::Cyan));

        let ctx_zoomed = PanelContext::new(Rect::default(), true, true);
        assert_eq!(ctx_zoomed.border_style().fg, Some(Color::Yellow));

        let ctx_unfocused = PanelContext::new(Rect::default(), false, false);
        assert_eq!(ctx_unfocused.border_style().fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_panel_context_compact() {
        let small = PanelContext::new(Rect::new(0, 0, 20, 5), false, false);
        assert!(small.is_compact());

        let large = PanelContext::new(Rect::new(0, 0, 80, 24), false, false);
        assert!(!large.is_compact());
    }
}
