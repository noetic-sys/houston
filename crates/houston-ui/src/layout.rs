//! Layout system for multi-panel arrangements.
//!
//! This module provides a tree-based layout system that supports
//! horizontal and vertical splits with configurable ratios.

use ratatui::prelude::*;
use std::collections::HashSet;

use crate::panel::PanelId;

/// Direction of a split in the layout tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Side-by-side (left | right)
    Horizontal,
    /// Stacked (top / bottom)
    Vertical,
}

/// A node in the layout tree.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A leaf node containing a single panel.
    Panel(PanelId),
    /// A split containing multiple children.
    Split {
        direction: SplitDirection,
        children: Vec<LayoutNode>,
        /// Ratios for each child (should sum to 1.0).
        ratios: Vec<f32>,
    },
}

impl LayoutNode {
    /// Creates a horizontal split with equal ratios.
    pub fn horizontal(children: Vec<LayoutNode>) -> Self {
        let count = children.len() as f32;
        let ratios = vec![1.0 / count; children.len()];
        Self::Split {
            direction: SplitDirection::Horizontal,
            children,
            ratios,
        }
    }

    /// Creates a horizontal split with custom ratios.
    pub fn horizontal_with_ratios(children: Vec<LayoutNode>, ratios: Vec<f32>) -> Self {
        Self::Split {
            direction: SplitDirection::Horizontal,
            children,
            ratios,
        }
    }

    /// Creates a vertical split with equal ratios.
    pub fn vertical(children: Vec<LayoutNode>) -> Self {
        let count = children.len() as f32;
        let ratios = vec![1.0 / count; children.len()];
        Self::Split {
            direction: SplitDirection::Vertical,
            children,
            ratios,
        }
    }

    /// Creates a vertical split with custom ratios.
    pub fn vertical_with_ratios(children: Vec<LayoutNode>, ratios: Vec<f32>) -> Self {
        Self::Split {
            direction: SplitDirection::Vertical,
            children,
            ratios,
        }
    }

    /// Computes the rectangles for all panels in this layout.
    pub fn compute_rects(&self, area: Rect) -> Vec<(PanelId, Rect)> {
        let mut result = Vec::new();
        self.compute_rects_recursive(area, &mut result);
        result
    }

    fn compute_rects_recursive(&self, area: Rect, result: &mut Vec<(PanelId, Rect)>) {
        match self {
            LayoutNode::Panel(id) => {
                result.push((*id, area));
            }
            LayoutNode::Split { direction, children, ratios } => {
                let constraints: Vec<Constraint> = ratios
                    .iter()
                    .map(|r| Constraint::Ratio((*r * 100.0) as u32, 100))
                    .collect();

                let layout_direction = match direction {
                    SplitDirection::Horizontal => Direction::Horizontal,
                    SplitDirection::Vertical => Direction::Vertical,
                };

                let chunks = Layout::default()
                    .direction(layout_direction)
                    .constraints(constraints)
                    .split(area);

                for (child, chunk) in children.iter().zip(chunks.iter()) {
                    child.compute_rects_recursive(*chunk, result);
                }
            }
        }
    }

    /// Collects all panel IDs in this layout.
    pub fn collect_panels(&self) -> HashSet<PanelId> {
        let mut result = HashSet::new();
        self.collect_panels_recursive(&mut result);
        result
    }

    fn collect_panels_recursive(&self, result: &mut HashSet<PanelId>) {
        match self {
            LayoutNode::Panel(id) => {
                result.insert(*id);
            }
            LayoutNode::Split { children, .. } => {
                for child in children {
                    child.collect_panels_recursive(result);
                }
            }
        }
    }

    /// Returns the panel IDs in order (left-to-right, top-to-bottom).
    pub fn panels_in_order(&self) -> Vec<PanelId> {
        let mut result = Vec::new();
        self.panels_in_order_recursive(&mut result);
        result
    }

    fn panels_in_order_recursive(&self, result: &mut Vec<PanelId>) {
        match self {
            LayoutNode::Panel(id) => {
                result.push(*id);
            }
            LayoutNode::Split { children, .. } => {
                for child in children {
                    child.panels_in_order_recursive(result);
                }
            }
        }
    }
}

/// Preset layouts corresponding to the 1-5 keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresetLayout {
    /// 1: Repos | Workflows | Tags (the original 3-panel view)
    #[default]
    Repos,
    /// 2: Workflows full screen
    Workflows,
    /// 3: Runs | Logs (the key use case)
    Runs,
    /// 4: Deployments | Runs
    Deployments,
    /// 5: Tags | Workflows
    Tags,
}

impl PresetLayout {
    /// Converts a key character to a preset layout.
    pub fn from_key(c: char) -> Option<Self> {
        match c {
            '1' => Some(PresetLayout::Repos),
            '2' => Some(PresetLayout::Workflows),
            '3' => Some(PresetLayout::Runs),
            '4' => Some(PresetLayout::Deployments),
            '5' => Some(PresetLayout::Tags),
            _ => None,
        }
    }

    /// Returns the key for this preset.
    pub fn key(&self) -> char {
        match self {
            PresetLayout::Repos => '1',
            PresetLayout::Workflows => '2',
            PresetLayout::Runs => '3',
            PresetLayout::Deployments => '4',
            PresetLayout::Tags => '5',
        }
    }

    /// Returns the display name for this preset.
    pub fn name(&self) -> &'static str {
        match self {
            PresetLayout::Repos => "Repos",
            PresetLayout::Workflows => "Workflows",
            PresetLayout::Runs => "Runs",
            PresetLayout::Deployments => "Deployments",
            PresetLayout::Tags => "Tags",
        }
    }

    /// Converts the preset to a layout tree.
    pub fn to_layout(&self) -> LayoutNode {
        match self {
            PresetLayout::Repos => {
                // 30% | 40% | 30%
                LayoutNode::horizontal_with_ratios(
                    vec![
                        LayoutNode::Panel(PanelId::Repos),
                        LayoutNode::Panel(PanelId::Workflows),
                        LayoutNode::Panel(PanelId::Tags),
                    ],
                    vec![0.30, 0.40, 0.30],
                )
            }
            PresetLayout::Workflows => {
                LayoutNode::Panel(PanelId::Workflows)
            }
            PresetLayout::Runs => {
                // 40% | 60%
                LayoutNode::horizontal_with_ratios(
                    vec![
                        LayoutNode::Panel(PanelId::Runs),
                        LayoutNode::Panel(PanelId::Logs),
                    ],
                    vec![0.40, 0.60],
                )
            }
            PresetLayout::Deployments => {
                // 50% | 50%
                LayoutNode::horizontal(vec![
                    LayoutNode::Panel(PanelId::Deployments),
                    LayoutNode::Panel(PanelId::Runs),
                ])
            }
            PresetLayout::Tags => {
                // 40% | 60%
                LayoutNode::horizontal_with_ratios(
                    vec![
                        LayoutNode::Panel(PanelId::Tags),
                        LayoutNode::Panel(PanelId::Workflows),
                    ],
                    vec![0.40, 0.60],
                )
            }
        }
    }

    /// Returns the default visible panels for this preset.
    pub fn default_visible(&self) -> HashSet<PanelId> {
        self.to_layout().collect_panels()
    }

    /// Returns the default focused panel for this preset.
    pub fn default_focus(&self) -> PanelId {
        match self {
            PresetLayout::Repos => PanelId::Repos,
            PresetLayout::Workflows => PanelId::Workflows,
            PresetLayout::Runs => PanelId::Runs,
            PresetLayout::Deployments => PanelId::Deployments,
            PresetLayout::Tags => PanelId::Tags,
        }
    }

    /// Returns all presets in order.
    pub fn all() -> &'static [PresetLayout] {
        &[
            PresetLayout::Repos,
            PresetLayout::Workflows,
            PresetLayout::Runs,
            PresetLayout::Deployments,
            PresetLayout::Tags,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_node_panel() {
        let node = LayoutNode::Panel(PanelId::Runs);
        let area = Rect::new(0, 0, 100, 50);
        let rects = node.compute_rects(area);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, PanelId::Runs);
        assert_eq!(rects[0].1, area);
    }

    #[test]
    fn test_layout_node_horizontal_split() {
        let node = LayoutNode::horizontal(vec![
            LayoutNode::Panel(PanelId::Runs),
            LayoutNode::Panel(PanelId::Logs),
        ]);
        let area = Rect::new(0, 0, 100, 50);
        let rects = node.compute_rects(area);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].0, PanelId::Runs);
        assert_eq!(rects[1].0, PanelId::Logs);
        // Each should be roughly half width
        assert!(rects[0].1.width >= 48 && rects[0].1.width <= 52);
        assert!(rects[1].1.width >= 48 && rects[1].1.width <= 52);
    }

    #[test]
    fn test_preset_layout_to_layout() {
        let repos = PresetLayout::Repos.to_layout();
        let panels = repos.collect_panels();
        assert!(panels.contains(&PanelId::Repos));
        assert!(panels.contains(&PanelId::Workflows));
        assert!(panels.contains(&PanelId::Tags));
        assert_eq!(panels.len(), 3);
    }

    #[test]
    fn test_preset_from_key() {
        assert_eq!(PresetLayout::from_key('1'), Some(PresetLayout::Repos));
        assert_eq!(PresetLayout::from_key('3'), Some(PresetLayout::Runs));
        assert_eq!(PresetLayout::from_key('x'), None);
    }

    #[test]
    fn test_panels_in_order() {
        let node = LayoutNode::horizontal(vec![
            LayoutNode::Panel(PanelId::Runs),
            LayoutNode::Panel(PanelId::Logs),
        ]);
        let order = node.panels_in_order();
        assert_eq!(order, vec![PanelId::Runs, PanelId::Logs]);
    }
}
