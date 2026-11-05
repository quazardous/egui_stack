use egui::Rect;

use crate::NodeIndex;

/// The direction in which a [`LinearNode`] lays out its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum LinearDir {
    /// Side by side, from left to right.
    #[default]
    Horizontal,

    /// One above the other, from top to bottom.
    Vertical,
}

/// The inner data of a [`Node::Linear`](crate::Node): two or more children in a row or a
/// column, each with a share of the space.
///
/// The shares are relative weights: a child gets `share / sum of shares` of the space left
/// once the separators are drawn. A separator between two neighbours resizes that pair only.
///
/// The children and the shares are kept in step; [`Tree`](crate::Tree) adds and removes
/// children.
///
/// The model (n-ary linear containers with shares) follows egui_tiles
/// (<https://github.com/rerun-io/egui_tiles>, MIT OR Apache-2.0).
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
pub struct LinearNode {
    /// The rectangle in which all children of this node are drawn.
    pub rect: Rect,

    /// The direction of the children.
    pub dir: LinearDir,

    /// The children, from left to right or from top to bottom.
    pub(crate) children: Vec<NodeIndex>,

    /// One share per child.
    pub(crate) shares: Vec<f32>,

    /// Whether all subnodes are collapsed.
    pub fully_collapsed: bool,

    /// The number of collapsed leaf subnodes.
    pub collapsed_leaf_count: i32,
}

impl Default for LinearNode {
    fn default() -> Self {
        Self::new(LinearDir::default(), Vec::new(), Vec::new())
    }
}

impl LinearNode {
    /// Creates a linear node holding `children`, each with its share.
    ///
    /// # Panics
    ///
    /// If `children` and `shares` differ in length.
    pub fn new(dir: LinearDir, children: Vec<NodeIndex>, shares: Vec<f32>) -> Self {
        assert_eq!(
            children.len(),
            shares.len(),
            "a linear node has one share per child"
        );
        Self {
            rect: Rect::NOTHING,
            dir,
            children,
            shares,
            fully_collapsed: false,
            collapsed_leaf_count: 0,
        }
    }

    /// Set the area which this node occupies.
    #[inline]
    pub fn set_rect(&mut self, new_rect: Rect) {
        self.rect = new_rect;
    }

    /// Get the area which this node occupies.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The children, from left to right or from top to bottom.
    pub fn children(&self) -> &[NodeIndex] {
        &self.children
    }

    /// The shares, one per child, in the order of [`children`](Self::children).
    pub fn shares(&self) -> &[f32] {
        &self.shares
    }

    /// The number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Whether the node has no children (never the case in a [`Tree`](crate::Tree)).
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Where `child` is among the children.
    pub fn position(&self, child: NodeIndex) -> Option<usize> {
        self.children.iter().position(|&c| c == child)
    }

    /// The part of the space (0 to 1) the child at `index` gets.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn fraction(&self, index: usize) -> f32 {
        let total = self.total();
        if total > 0.0 {
            self.shares[index] / total
        } else {
            1.0 / self.shares.len() as f32
        }
    }

    /// Where the separator after the child at `index` stands, from 0 (the start of the node)
    /// to 1 (its end): the sum of the fractions up to and including `index`.
    ///
    /// For two children, `boundary(0)` is the split's fraction.
    pub fn boundary(&self, index: usize) -> f32 {
        (0..=index).map(|i| self.fraction(i)).sum()
    }

    /// Replaces the shares.
    ///
    /// # Panics
    ///
    /// If there is not one share per child, or a share is negative or not finite.
    pub fn set_shares(&mut self, shares: Vec<f32>) {
        assert_eq!(
            shares.len(),
            self.children.len(),
            "a linear node has one share per child"
        );
        assert!(
            shares.iter().all(|s| s.is_finite() && *s >= 0.0),
            "shares are finite and not negative"
        );
        self.shares = shares;
    }

    /// The sum of the shares.
    fn total(&self) -> f32 {
        self.shares.iter().sum()
    }

    /// Scales the shares so they sum to 1, or makes them equal if they sum to 0.
    pub(crate) fn normalize_shares(&mut self) {
        let total = self.total();
        let n = self.shares.len() as f32;
        for share in &mut self.shares {
            *share = if total > 0.0 { *share / total } else { 1.0 / n };
        }
    }

    /// Moves the separator after the child at `index` by `delta`, a part of the whole node
    /// (0 to 1), between that child and the next one. Each keeps at least `min`, a part of the
    /// whole node too. The other children keep their size.
    pub(crate) fn move_boundary(&mut self, index: usize, delta: f32, min: f32) {
        self.normalize_shares();
        let pair = self.shares[index] + self.shares[index + 1];
        let min = min.min(pair / 2.0);
        let first = (self.shares[index] + delta).clamp(min, pair - min);
        self.shares[index] = first;
        self.shares[index + 1] = pair - first;
    }

    /// Gives the child at `index` and the next one the same size.
    pub(crate) fn even_pair(&mut self, index: usize) {
        self.normalize_shares();
        let half = (self.shares[index] + self.shares[index + 1]) / 2.0;
        self.shares[index] = half;
        self.shares[index + 1] = half;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(shares: &[f32]) -> LinearNode {
        LinearNode::new(
            LinearDir::Horizontal,
            (0..shares.len()).map(NodeIndex).collect(),
            shares.to_vec(),
        )
    }

    #[test]
    fn fractions_and_boundaries() {
        let n = node(&[1.0, 2.0, 1.0]);
        assert_eq!(n.fraction(1), 0.5);
        assert_eq!(n.boundary(0), 0.25);
        assert_eq!(n.boundary(1), 0.75);
        assert!((n.boundary(2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_boundary_moves_its_pair_only() {
        let mut n = node(&[0.25, 0.5, 0.25]);
        n.move_boundary(1, 0.1, 0.05);
        assert_eq!(n.shares()[0], 0.25);
        assert!((n.shares()[1] - 0.6).abs() < 1e-6);
        assert!((n.shares()[2] - 0.15).abs() < 1e-6);
        // Clamped so the next child keeps its minimum.
        n.move_boundary(1, 1.0, 0.05);
        assert!((n.shares()[2] - 0.05).abs() < 1e-6);
        assert_eq!(n.shares()[0], 0.25);
    }

    #[test]
    fn an_even_pair() {
        let mut n = node(&[2.0, 1.0, 1.0]);
        n.even_pair(0);
        assert_eq!(n.shares(), &[0.375, 0.375, 0.25]);
    }
}
