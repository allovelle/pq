use std::{
    fmt::Debug,
    ops::{Deref, DerefMut, RangeInclusive},
};

pub type NodeId = u32;
pub const ROOT: NodeId = u32::MAX;

// Only store copy types for string keys, use string interning
#[cfg(not(feature = "store_heap_strs"))]
pub trait Node: Debug + Clone + Copy {}
#[cfg(not(feature = "store_heap_strs"))]
impl<T: Debug + Clone + Copy> Node for T {}

// Don't store interned string, store String directly
#[cfg(feature = "store_heap_strs")]
pub trait Node: Debug + Clone {}
#[cfg(feature = "store_heap_strs")]
impl<T: Debug + Clone> Node for T {}

#[derive(Debug, Clone)]
pub struct Row<T>
{
    #[cfg(not(feature = "implicit_row_ids"))]
    pub id: NodeId,
    pub parent: NodeId,
    value: T,
}

impl<T: Node> Deref for Row<T>
{
    type Target = T;

    fn deref(&self) -> &T
    {
        &self.value
    }
}

impl<T: Node> DerefMut for Row<T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        &mut self.value
    }
}

/// Trade-offs:
/// O(1) first child
/// O(1) next sibling
/// O(n) sibling iteration
/// O(depth) upward traversal
/// zero extra memory
/// no maps
/// no child vectors
pub struct RowTree<T>
{
    rows: Vec<Row<T>>,
}

pub struct Cursor<'a, T>
{
    tree: &'a RowTree<T>,
    pub id: NodeId,
}

impl<T: Node> RowTree<T>
{
    pub fn cursor(&self, id: NodeId) -> Cursor<'_, T>
    {
        Cursor { tree: self, id }
    }

    fn row(&self, id: NodeId) -> &Row<T>
    {
        &self.rows[id as usize]
    }

    /// Mount a copy of an entire subtree onto the end of the table.
    /// 1. Reworks ids to maintain the invariant that ids are equal to indices.
    /// 2. Adjusts the provided base node's parent id to equal it's own id.
    pub fn graft(&mut self, base: NodeId) -> NodeId
    {
        let base = base as usize;

        let mut end = base + 1;

        while end < self.rows.len() && self.rows[end].parent >= base as NodeId
        {
            end += 1;
        }

        let new_base = self.rows.len() as NodeId;
        let offset = new_base - base as NodeId;

        self.rows.reserve(end - base);

        for i in base .. end
        {
            let src = &self.rows[i];

            let new_id = src.id + offset;

            let new_parent =
                if i == base { new_id } else { src.parent + offset };

            self.rows.push(Row {
                id: new_id,
                parent: new_parent,
                value: src.value.clone(),
            });
        }

        new_base
    }
}

impl<'a, T: Node> Cursor<'a, T>
{
    pub fn first_child(&self) -> Option<NodeId>
    {
        let start = self.id + 1;

        if start as usize >= self.tree.rows.len()
        {
            return None;
        }

        let row = self.tree.row(start);

        if row.parent == self.id { Some(start) } else { None }
    }

    pub fn siblings(&self) -> impl Iterator<Item = NodeId> + '_
    {
        let parent = self.tree.row(self.id).parent;

        let mut start = self.id as usize;
        let mut end = self.id as usize;

        if parent != ROOT
        {
            while start > 0
            {
                let prev = start - 1;
                if self.tree.rows[prev].parent == parent
                {
                    start = prev;
                }
                else
                {
                    break;
                }
            }

            while end + 1 < self.tree.rows.len()
            {
                let next = end + 1;
                if self.tree.rows[next].parent == parent
                {
                    end = next;
                }
                else
                {
                    break;
                }
            }
        }

        self.tree.rows[start ..= end]
            .iter()
            .map(|row| row.id)
            .filter(move |&id| id != self.id)
    }

    pub fn parent_sibling(&self) -> Option<NodeId>
    {
        let parent = self.tree.row(self.id).parent;

        if parent == ROOT
        {
            return None;
        }

        let grandparent = self.tree.row(parent).parent;

        let mut next = parent + 1;

        while (next as usize) < self.tree.rows.len()
        {
            let row = self.tree.row(next);

            if row.parent == grandparent
            {
                return Some(next);
            }

            // stop if we leave parent's sibling block
            if row.parent < grandparent
            {
                break;
            }

            next += 1;
        }

        None
    }

    pub fn next_sibling(&self) -> Option<NodeId>
    {
        let parent = self.tree.row(self.id).parent;
        let next = self.id + 1;

        if (next as usize) >= self.tree.rows.len()
        {
            return None;
        }

        if self.tree.row(next).parent == parent { Some(next) } else { None }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    // Helper to build a RowTree from a list of (id, parent, value) tuples
    fn build_tree(nodes: Vec<(NodeId, NodeId, i32)>) -> RowTree<i32>
    {
        let rows = nodes
            .into_iter()
            .map(|(id, parent, value)| Row { id, parent, value })
            .collect();
        RowTree { rows }
    }

    // Build a simple tree:
    //   0 (parent=ROOT)
    //   ├── 1
    //   │   ├── 2
    //   │   └── 3
    //   └── 4
    fn sample_tree() -> RowTree<i32>
    {
        build_tree(vec![
            (0, ROOT, 0),
            (1, 0, 1),
            (2, 1, 2),
            (3, 1, 3),
            (4, 0, 4),
        ])
    }

    // --- Cursor::first_child ---

    #[test]
    fn first_child_exists()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(0).first_child(), Some(1));
    }

    #[test]
    fn first_child_nested()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(1).first_child(), Some(2));
    }

    #[test]
    fn first_child_leaf_returns_none()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(2).first_child(), None);
    }

    #[test]
    fn first_child_last_node_returns_none()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(4).first_child(), None);
    }

    // --- Cursor::next_sibling ---

    #[test]
    fn next_sibling_exists()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(2).next_sibling(), Some(3));
    }

    #[test]
    fn next_sibling_last_child_returns_none()
    {
        let tree = sample_tree();
        assert_eq!(tree.cursor(3).next_sibling(), None);
    }

    #[test]
    fn next_sibling_across_depths_returns_none()
    {
        // Node 3 is the last child of node 1; node 4 is a child of 0, not 1
        let tree = sample_tree();
        assert_eq!(tree.cursor(3).next_sibling(), None);
    }

    // --- Cursor::siblings ---

    #[test]
    fn siblings_returns_all_but_self()
    {
        let tree = sample_tree();
        let mut sibs: Vec<NodeId> = tree.cursor(2).siblings().collect();
        sibs.sort();
        assert_eq!(sibs, vec![3]);
    }

    #[test]
    fn siblings_of_only_child_is_empty()
    {
        let tree = sample_tree();
        let sibs: Vec<NodeId> = tree.cursor(4).siblings().collect();
        assert_eq!(sibs, Vec::<NodeId>::new());
    }

    #[test]
    fn siblings_of_root_is_empty()
    {
        let tree = sample_tree();
        let sibs: Vec<NodeId> = tree.cursor(0).siblings().collect();
        assert_eq!(sibs, Vec::<NodeId>::new());
    }

    #[test]
    fn siblings_from_middle_child()
    {
        // Build: root -> 1, 2, 3 all children of 0
        let tree =
            build_tree(vec![(0, ROOT, 0), (1, 0, 1), (2, 0, 2), (3, 0, 3)]);
        let mut sibs: Vec<NodeId> = tree.cursor(2).siblings().collect();
        sibs.sort();
        assert_eq!(sibs, vec![1, 3]);
    }

    // --- Cursor::parent_sibling ---

    #[test]
    fn parent_sibling_exists()
    {
        // Node 2's parent is 1, and 1's next sibling is 4
        let tree = sample_tree();
        assert_eq!(tree.cursor(2).parent_sibling(), Some(4));
    }

    #[test]
    fn parent_sibling_root_child_returns_none()
    {
        // Node 1's parent is 0, which is ROOT child (parent == ROOT)
        let tree = sample_tree();
        assert_eq!(tree.cursor(1).parent_sibling(), None);
    }

    #[test]
    fn parent_sibling_when_parent_is_last_sibling_returns_none()
    {
        // Node 3's parent is 1; node 1's next sibling would be 4, but
        // that requires parent(4)==0==ROOT child, so parent_sibling of 3 == Some(4)
        let tree = sample_tree();
        // 4 has no next sibling, so a child of 4 has no parent_sibling
        let tree2 = build_tree(vec![
            (0, ROOT, 0),
            (1, 0, 1),
            (2, 1, 2),
            (3, 0, 3),
            (4, 3, 4), // child of 3, which has no next sibling
        ]);
        assert_eq!(tree2.cursor(4).parent_sibling(), None);
    }

    // --- RowTree::graft ---

    #[test]
    fn graft_single_node()
    {
        let mut tree = build_tree(vec![(0, ROOT, 42)]);
        let new_id = tree.graft(0);
        assert_eq!(new_id, 1);
        assert_eq!(*tree.row(new_id), 42);
        // Grafted root should be self-parented
        assert_eq!(tree.row(new_id).parent, new_id);
    }

    #[test]
    fn graft_subtree_preserves_structure()
    {
        let mut tree = sample_tree();
        // Graft subtree rooted at node 1 (contains 1, 2, 3)
        let new_base = tree.graft(1);
        assert_eq!(new_base, 5);

        // New ids should be 5, 6, 7
        // new_base (5) is self-parented
        assert_eq!(tree.row(5).parent, 5);
        // 6 and 7 should be children of 5
        assert_eq!(tree.row(6).parent, 5);
        assert_eq!(tree.row(7).parent, 5);

        // Values should be copied
        assert_eq!(*tree.row(5), 1);
        assert_eq!(*tree.row(6), 2);
        assert_eq!(*tree.row(7), 3);
    }

    #[test]
    fn graft_does_not_include_later_siblings()
    {
        let mut tree = sample_tree();
        let original_len = tree.rows.len();
        // Graft node 1 subtree (nodes 1, 2, 3) — should NOT include node 4
        tree.graft(1);
        // 3 new nodes added (1, 2, 3)
        assert_eq!(tree.rows.len(), original_len + 3);
    }

    // --- Deref / DerefMut ---

    #[test]
    fn deref_accesses_value()
    {
        let tree = sample_tree();
        assert_eq!(*tree.row(2), 2);
    }

    #[test]
    fn deref_mut_modifies_value()
    {
        let mut tree = sample_tree();
        *tree.rows[2] = 99;
        assert_eq!(*tree.row(2), 99);
    }
}
