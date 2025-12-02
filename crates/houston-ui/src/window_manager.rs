//! Window manager for coordinating panel visibility, focus, and layout.
//!
//! The WindowManager is the central coordinator for the multi-panel UI.
//! It tracks which panels are visible, which has focus, and handles
//! zoom/layout operations.

use ratatui::prelude::*;
use std::collections::HashSet;

use crate::layout::{LayoutNode, PresetLayout};
use crate::panel::{PanelContext, PanelId};

/// The window manager coordinates panel visibility, focus, and layout.
#[derive(Debug, Clone)]
pub struct WindowManager {
    /// Set of currently visible panels.
    visible: HashSet<PanelId>,
    /// The currently focused panel.
    focused: PanelId,
    /// Panel that is zoomed to full screen (if any).
    zoomed: Option<PanelId>,
    /// The current preset layout.
    current_preset: PresetLayout,
    /// Custom visibility overrides (panels toggled on/off from preset).
    visibility_overrides: HashSet<PanelId>,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager {
    /// Creates a new window manager with default settings.
    pub fn new() -> Self {
        let preset = PresetLayout::default();
        Self {
            visible: preset.default_visible(),
            focused: preset.default_focus(),
            zoomed: None,
            current_preset: preset,
            visibility_overrides: HashSet::new(),
        }
    }

    /// Creates a window manager starting with a specific preset.
    pub fn with_preset(preset: PresetLayout) -> Self {
        Self {
            visible: preset.default_visible(),
            focused: preset.default_focus(),
            zoomed: None,
            current_preset: preset,
            visibility_overrides: HashSet::new(),
        }
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Returns the current preset layout.
    pub fn current_preset(&self) -> PresetLayout {
        self.current_preset
    }

    /// Returns the currently focused panel.
    pub fn focused(&self) -> PanelId {
        self.focused
    }

    /// Returns the zoomed panel (if any).
    pub fn zoomed(&self) -> Option<PanelId> {
        self.zoomed
    }

    /// Returns whether the UI is in zoomed mode.
    pub fn is_zoomed(&self) -> bool {
        self.zoomed.is_some()
    }

    /// Returns whether a panel is currently visible.
    pub fn is_visible(&self, panel: PanelId) -> bool {
        self.visible.contains(&panel)
    }

    /// Returns an iterator over visible panels.
    pub fn visible_panels(&self) -> impl Iterator<Item = &PanelId> {
        self.visible.iter()
    }

    /// Returns the number of visible panels.
    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }

    // =========================================================================
    // Panel Visibility
    // =========================================================================

    /// Toggles a panel's visibility.
    pub fn toggle_panel(&mut self, panel: PanelId) {
        if self.visible.contains(&panel) {
            // Don't allow hiding the last visible panel
            if self.visible.len() > 1 {
                self.visible.remove(&panel);
                self.visibility_overrides.insert(panel);

                // If we hid the focused panel, move focus
                if self.focused == panel {
                    self.focus_first_visible();
                }
            }
        } else {
            self.visible.insert(panel);
            self.visibility_overrides.insert(panel);
        }
    }

    /// Shows a panel (makes it visible).
    pub fn show_panel(&mut self, panel: PanelId) {
        self.visible.insert(panel);
    }

    /// Hides a panel.
    pub fn hide_panel(&mut self, panel: PanelId) {
        if self.visible.len() > 1 {
            self.visible.remove(&panel);
            if self.focused == panel {
                self.focus_first_visible();
            }
        }
    }

    // =========================================================================
    // Focus Management
    // =========================================================================

    /// Sets focus to a specific panel.
    pub fn set_focus(&mut self, panel: PanelId) {
        if self.visible.contains(&panel) {
            self.focused = panel;
        }
    }

    /// Cycles focus to the next visible panel.
    pub fn focus_next(&mut self) {
        let panels = self.panels_in_display_order();
        if panels.is_empty() {
            return;
        }

        let current_idx = panels.iter().position(|p| *p == self.focused).unwrap_or(0);
        let next_idx = (current_idx + 1) % panels.len();
        self.focused = panels[next_idx];
    }

    /// Cycles focus to the previous visible panel.
    pub fn focus_prev(&mut self) {
        let panels = self.panels_in_display_order();
        if panels.is_empty() {
            return;
        }

        let current_idx = panels.iter().position(|p| *p == self.focused).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            panels.len() - 1
        } else {
            current_idx - 1
        };
        self.focused = panels[prev_idx];
    }

    /// Focuses the first visible panel.
    fn focus_first_visible(&mut self) {
        let panels = self.panels_in_display_order();
        if let Some(first) = panels.first() {
            self.focused = *first;
        }
    }

    /// Focus the panel to the left of current (if in horizontal layout).
    pub fn focus_left(&mut self) {
        self.focus_prev();
    }

    /// Focus the panel to the right of current (if in horizontal layout).
    pub fn focus_right(&mut self) {
        self.focus_next();
    }

    // =========================================================================
    // Zoom
    // =========================================================================

    /// Toggles zoom on the currently focused panel.
    pub fn toggle_zoom(&mut self) {
        if self.zoomed.is_some() {
            self.zoomed = None;
        } else {
            self.zoomed = Some(self.focused);
        }
    }

    /// Zooms a specific panel.
    pub fn zoom_panel(&mut self, panel: PanelId) {
        self.zoomed = Some(panel);
        self.focused = panel;
    }

    /// Exits zoom mode.
    pub fn unzoom(&mut self) {
        self.zoomed = None;
    }

    // =========================================================================
    // Layout/Preset Management
    // =========================================================================

    /// Applies a preset layout.
    pub fn apply_preset(&mut self, preset: PresetLayout) {
        self.current_preset = preset;
        self.visible = preset.default_visible();
        self.focused = preset.default_focus();
        self.zoomed = None;
        self.visibility_overrides.clear();
    }

    /// Returns the current layout, accounting for visibility overrides and zoom.
    pub fn get_layout(&self) -> LayoutNode {
        if let Some(zoomed_panel) = self.zoomed {
            // Zoomed: single panel full screen
            return LayoutNode::Panel(zoomed_panel);
        }

        // Get the base layout from preset
        let base_layout = self.current_preset.to_layout();

        // Filter to only visible panels
        self.filter_layout(&base_layout)
    }

    /// Filters a layout tree to only include visible panels.
    fn filter_layout(&self, node: &LayoutNode) -> LayoutNode {
        match node {
            LayoutNode::Panel(id) => {
                // If not visible, this shouldn't happen in normal flow
                // but we handle it gracefully
                LayoutNode::Panel(*id)
            }
            LayoutNode::Split { direction, children, ratios } => {
                // Filter children to only visible ones
                let filtered: Vec<(LayoutNode, f32)> = children
                    .iter()
                    .zip(ratios.iter())
                    .filter_map(|(child, ratio)| {
                        let child_panels = child.collect_panels();
                        if child_panels.iter().any(|p| self.visible.contains(p)) {
                            Some((self.filter_layout(child), *ratio))
                        } else {
                            None
                        }
                    })
                    .collect();

                if filtered.is_empty() {
                    // Shouldn't happen, but default to first panel
                    LayoutNode::Panel(self.focused)
                } else if filtered.len() == 1 {
                    // Single child, unwrap the split
                    filtered.into_iter().next().unwrap().0
                } else {
                    // Re-normalize ratios
                    let total: f32 = filtered.iter().map(|(_, r)| r).sum();
                    let (children, ratios): (Vec<_>, Vec<_>) = filtered
                        .into_iter()
                        .map(|(c, r)| (c, r / total))
                        .unzip();

                    LayoutNode::Split {
                        direction: *direction,
                        children,
                        ratios,
                    }
                }
            }
        }
    }

    /// Computes panel rectangles for the given area.
    pub fn compute_panel_rects(&self, area: Rect) -> Vec<(PanelId, Rect)> {
        let layout = self.get_layout();
        layout.compute_rects(area)
    }

    /// Returns panels in display order (for Tab navigation).
    fn panels_in_display_order(&self) -> Vec<PanelId> {
        let layout = self.get_layout();
        layout
            .panels_in_order()
            .into_iter()
            .filter(|p| self.visible.contains(p))
            .collect()
    }

    // =========================================================================
    // Panel Context Generation
    // =========================================================================

    /// Creates a PanelContext for the given panel and area.
    pub fn panel_context(&self, panel: PanelId, area: Rect) -> PanelContext {
        PanelContext::new(
            area,
            self.focused == panel,
            self.zoomed == Some(panel),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_manager_new() {
        let wm = WindowManager::new();
        assert_eq!(wm.current_preset(), PresetLayout::Repos);
        assert!(wm.is_visible(PanelId::Repos));
        assert!(wm.is_visible(PanelId::Workflows));
        assert!(wm.is_visible(PanelId::Tags));
        assert!(!wm.is_visible(PanelId::Runs));
    }

    #[test]
    fn test_apply_preset() {
        let mut wm = WindowManager::new();
        wm.apply_preset(PresetLayout::Runs);

        assert_eq!(wm.current_preset(), PresetLayout::Runs);
        assert!(wm.is_visible(PanelId::Runs));
        assert!(wm.is_visible(PanelId::Logs));
        assert!(!wm.is_visible(PanelId::Repos));
        assert_eq!(wm.focused(), PanelId::Runs);
    }

    #[test]
    fn test_toggle_panel() {
        let mut wm = WindowManager::new();
        assert!(wm.is_visible(PanelId::Tags));

        wm.toggle_panel(PanelId::Tags);
        assert!(!wm.is_visible(PanelId::Tags));

        wm.toggle_panel(PanelId::Tags);
        assert!(wm.is_visible(PanelId::Tags));
    }

    #[test]
    fn test_cannot_hide_last_panel() {
        let mut wm = WindowManager::with_preset(PresetLayout::Workflows);
        // Only Workflows is visible
        assert_eq!(wm.visible_count(), 1);

        wm.toggle_panel(PanelId::Workflows);
        // Should still be visible
        assert!(wm.is_visible(PanelId::Workflows));
    }

    #[test]
    fn test_focus_cycling() {
        let mut wm = WindowManager::new();
        assert_eq!(wm.focused(), PanelId::Repos);

        wm.focus_next();
        assert_eq!(wm.focused(), PanelId::Workflows);

        wm.focus_next();
        assert_eq!(wm.focused(), PanelId::Tags);

        wm.focus_next();
        assert_eq!(wm.focused(), PanelId::Repos); // Wraps around

        wm.focus_prev();
        assert_eq!(wm.focused(), PanelId::Tags);
    }

    #[test]
    fn test_zoom() {
        let mut wm = WindowManager::new();
        assert!(!wm.is_zoomed());

        wm.toggle_zoom();
        assert!(wm.is_zoomed());
        assert_eq!(wm.zoomed(), Some(PanelId::Repos));

        wm.toggle_zoom();
        assert!(!wm.is_zoomed());
        assert_eq!(wm.zoomed(), None);
    }

    #[test]
    fn test_compute_panel_rects() {
        let wm = WindowManager::with_preset(PresetLayout::Runs);
        let area = Rect::new(0, 0, 100, 50);

        let rects = wm.compute_panel_rects(area);
        assert_eq!(rects.len(), 2);

        // Runs should be 40%, Logs should be 60%
        let runs_rect = rects.iter().find(|(id, _)| *id == PanelId::Runs).unwrap().1;
        let logs_rect = rects.iter().find(|(id, _)| *id == PanelId::Logs).unwrap().1;

        assert!(runs_rect.width < logs_rect.width);
    }

    #[test]
    fn test_zoomed_layout() {
        let mut wm = WindowManager::with_preset(PresetLayout::Runs);
        wm.zoom_panel(PanelId::Logs);

        let area = Rect::new(0, 0, 100, 50);
        let rects = wm.compute_panel_rects(area);

        // Should only have one panel (Logs) taking full area
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, PanelId::Logs);
        assert_eq!(rects[0].1, area);
    }
}
