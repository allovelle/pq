use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::parser::RowType;

// Only store copy types for string keys, use string interning
// Don't store interned string, store String directly
#[cfg(feature = "store_heap_strs")]
pub trait Node: Debug + Clone {}
#[cfg(feature = "store_heap_strs")]
impl<T: Debug + Clone> Node for T {}

#[cfg(feature = "store_heap_strs")]
#[derive(Debug, Clone, PartialEq)]
pub struct Row
{
    #[cfg(not(feature = "implicit_row_ids"))]
    pub id: NodeId,
    pub par: NodeId,
    key: String,
    val: String,
    ty: RowType,
}

// ? 1. Use dedicated Row type for the tree table parsing that doesn't require
// ?     the <T> AND
// ? 2. Create row type that the parser can use that utilizes the txt buffer for
// ?     strings
// ? 3. Create string interning type that hands out byte index offsets instead
// ?     of references because offsets can be 32-bit instead of usize 64-bit

// TODO: JUST MODIFY THE TXT BUF TO HAND OUT U32 HANDLE AND DEREF U32 HANDLE
// TODO: INTO &'static str because it's already tracking byte-offsets anyway!

#[cfg(not(feature = "store_heap_strs"))]
#[cfg(feature = "store_heap_strs")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row
{
    #[cfg(not(feature = "implicit_row_ids"))]
    pub id: NodeId,
    pub par: NodeId,
    // ! THIS REQUIRES UTF8_BUFFER YIELDING _STATIC_ STRING SLICE REFERENCES
    // ! EVEN THOUGH THEY ARE _NOT_ STATIC
    key: &'static str,
    val: &'static str,
    ty: RowType,
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////

// TODO: *Remove the .root field gradually using .root() instead until removed*

pub type NodeId = u32;

// Only store copy types for string keys, use string interning
#[cfg(not(feature = "store_heap_strs"))]
pub trait Node: Debug + Clone + Copy + PartialEq {}
#[cfg(not(feature = "store_heap_strs"))]
impl<T: Debug + Clone + Copy + PartialEq> Node for T {}

// Don't store interned string, store String directly
#[cfg(feature = "store_heap_strs")]
pub trait Node: Debug + Clone {}
#[cfg(feature = "store_heap_strs")]
impl<T: Debug + Clone> Node for T {}

#[derive(Debug, Clone, PartialEq)]
pub struct Row<T>
{
    #[cfg(not(feature = "implicit_row_ids"))]
    pub id: NodeId,
    pub par: NodeId,
    val: T,
}

impl<T> Row<T>
{
    pub fn new(id: NodeId, par: NodeId, val: T) -> Self
    {
        Self { id, par, val }
    }

    pub fn is_doc(&self) -> bool
    {
        self.par == self.id
    }
}

impl<T: Node> Deref for Row<T>
{
    type Target = T;

    fn deref(&self) -> &T
    {
        &self.val
    }
}

impl<T: Node> DerefMut for Row<T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        &mut self.val
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
/// Row ids are monotonic so later nodes always have greater id.
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
    /// New RowTree with backing table.
    pub fn new(rows: Vec<Row<T>>) -> Self
    {
        Self { rows }
    }

    /// New cursor at node id (& offset)
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

        while end < self.rows.len()
            && self.rows[end].par >= base as NodeId
            && !self.rows[end].is_doc()
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

            let new_parent = if i == base { new_id } else { src.par + offset };

            self.rows.push(Row { id: new_id, par: new_parent, val: src.val });
        }

        new_base
    }
}

impl<'a, T: Node> Cursor<'a, T>
{
    /// New cursor over provided tree at default node id (& offset 0)
    pub fn new(tree: &'a RowTree<T>) -> Self
    {
        Self { tree, id: 0 }
    }

    pub fn first_child(&self) -> Option<NodeId>
    {
        let start = self.id + 1;

        if start as usize >= self.tree.rows.len()
        {
            return None;
        }

        let row = self.tree.row(start);

        if row.par == self.id { Some(start) } else { None }
    }

    pub fn siblings(&self) -> impl Iterator<Item = NodeId> + '_
    {
        let parent = self.tree.row(self.id).par;

        let mut start = self.id as usize;
        let mut end = self.id as usize;

        if parent != self.id
        {
            while start > 0
            {
                let prev = start - 1;
                if self.tree.rows[prev].par == parent
                    && self.tree.rows[prev].par != self.tree.rows[prev].id
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
                if self.tree.rows[next].par == parent
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
        let parent = self.tree.row(self.id).par;

        // Parent is a root → no parent siblings
        if parent == self.tree.row(parent).par
        {
            return None;
        }

        let grandparent = self.tree.row(parent).par;
        let mut next = parent + 1;

        while (next as usize) < self.tree.rows.len()
        {
            let row = self.tree.row(next);

            if row.par == grandparent
            {
                return Some(next);
            }

            // stop if we leave the parent's sibling block
            if row.par < grandparent
            {
                break;
            }

            next += 1;
        }

        None
    }

    pub fn next_sibling(&self) -> Option<NodeId>
    {
        let parent = self.tree.row(self.id).par;
        let next = self.id + 1;

        if (next as usize) >= self.tree.rows.len()
        {
            return None;
        }

        if self.tree.row(next).par == parent { Some(next) } else { None }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    // Helper to build a RowTree from a list of (id, parent, value) tuples
    fn build_tree(nodes: Vec<(NodeId, NodeId, char)>) -> RowTree<char>
    {
        let rows = nodes
            .into_iter()
            .map(|(id, parent, value)| Row { id, par: parent, val: value })
            .collect();
        RowTree { rows }
    }

    /// Build a simple tree:
    /// ```
    /// ┌── 0 (par=0, new-doc, cannot have siblings)
    /// │   ├── 1
    /// │   ├── 2
    /// │   └── 3
    /// └── 4 (par=4, new-doc, cannot have siblings)
    ///     └── 5
    ///     │   └── 6
    ///     └── 7
    /// ```
    fn sample_tree() -> RowTree<char>
    {
        build_tree(vec![
            // id, par, VALUE, NOT doc root
            (0, 0, '0'),
            (1, 0, '0'),
            (2, 0, '0'),
            (3, 0, '0'),
            (4, 4, '4'),
            (5, 4, '4'),
            (6, 5, '4'),
            (7, 4, '4'),
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
        assert_eq!(tree.cursor(4).first_child(), Some(5));
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
        assert_eq!(tree.cursor(6).first_child(), None);
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
        assert_eq!(sibs, vec![1, 3]);
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
        let tree = build_tree(vec![
            (0, 0, '0'),
            (1, 0, '1'),
            (2, 0, '2'),
            (3, 0, '3'),
        ]);
        let mut sibs: Vec<NodeId> = tree.cursor(2).siblings().collect();
        sibs.sort();
        assert_eq!(sibs, vec![1, 3]);
    }

    // --- Cursor::parent_sibling ---

    #[test]
    fn parent_sibling_exists()
    {
        // Node 6's parent, 5, who's next sibling is 7
        let tree = sample_tree();
        assert_eq!(tree.cursor(6).parent_sibling(), Some(7));
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
            (0, 0, '0'),
            (1, 0, '1'),
            (2, 1, '2'),
            (3, 0, '3'),
            (4, 3, '4'), // child of 3, which has no next sibling
        ]);
        assert_eq!(tree2.cursor(4).parent_sibling(), None);
    }

    // --- RowTree::graft ---

    #[test]
    fn graft_single_node()
    {
        let mut tree = build_tree(vec![(0, 0, '8')]);
        let new_id = tree.graft(0);
        assert_eq!(new_id, 1);
        assert_eq!(**tree.row(new_id), '8');
        // Grafted root should be self-parented
        assert_eq!(tree.row(new_id).par, new_id);
    }

    #[test]
    fn graft_subtree_preserves_structure()
    {
        // ! Graft node with no subnodes
        let mut tree = sample_tree();
        let new_base = tree.graft(1);
        assert_eq!(new_base, 8);
        assert_eq!(tree.rows.len(), 9);

        // ! Graft entire root document with subnodes
        let mut tree = sample_tree();
        let new_base = tree.graft(0);
        assert_eq!(new_base, 8);
        assert_eq!(tree.row(8).par, 8);
        assert_eq!(tree.row(9).par, 8);
        assert_eq!(tree.row(10).par, 8);
        assert_eq!(tree.row(11).par, 8);

        // Values should be copied
        assert_eq!(tree.row(8).val, '0');
        assert_eq!(tree.row(9).val, '0');
        assert_eq!(tree.row(10).val, '0');
        assert_eq!(tree.row(11).val, '0');

        // Should not have copied rows outside the current doc or tree
        assert_eq!(tree.rows.len(), 12);

        // ! Graft the second/other root document with subtree
        let mut tree = sample_tree();
        let new_base = tree.graft(4);
        assert_eq!(new_base, 8);
        assert_eq!(tree.row(8).par, 8);
        assert_eq!(tree.row(9).par, 8);
        assert_eq!(tree.row(10).par, 9);
        assert_eq!(tree.row(11).par, 8);

        // Values should be copied
        assert_eq!(tree.row(8).val, '4');
        assert_eq!(tree.row(9).val, '4');
        assert_eq!(tree.row(10).val, '4');
        assert_eq!(tree.row(11).val, '4');
    }

    #[test]
    fn graft_does_not_include_later_siblings()
    {
        let mut tree = sample_tree();
        let original_len = tree.rows.len();
        let new_root = tree.graft(0);
        let grafted_len = tree.rows.len();

        // Seems as if there's no other nodes beyond last subnode of new_root
        assert_eq!(tree.cursor(new_root).parent_sibling(), None);

        // Yet actual grafted count can be higher if the algo overshot
        assert_eq!(original_len, 8, "should be 2 root docs, 4 nodes each");
        assert_eq!(grafted_len, 12, "should be 3 root docs, 4 nodes each");
    }

    // --- Deref / DerefMut ---

    #[test]
    fn deref_accesses_value()
    {
        let tree = sample_tree();
        assert_eq!(**tree.row(2), '0');
    }

    #[test]
    fn deref_mut_modifies_value()
    {
        let mut tree = sample_tree();
        *tree.rows[2] = '9';
        assert_eq!(**tree.row(2), '9');
    }
}
