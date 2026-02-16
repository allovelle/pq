use crate::parser::{Row, RowType};

// TODO: Root node key is filename path:
// TODO: { id: 0, par: 0, key: "path/to/json.json", val: "{", ty: Root }
// TODO: This is a path to a subnode of a document for easy error reporting.
// TODO: They are node IDs that allow walking up the tree to reconstruct a
// TODO: nested path for error reporting. Can stop at any indentation to fit.
// TODO: { id: 0, par: 0, key: "path/to/json.json/13/818", val: "{", ty: Root }

// TODO: Design: node parent = node id means root document without losing the
// TODO: property that row id = table index.
/*

0: [id 0, par 0]
1: [id 1, par 0]
2: [id 2, par 2]  <- allows subnodes to directly index other nodes
3: [id 3, par 2]
4: [id 4, par 2]
5: [id 5, par 4]

*/

// TODO: Design: node id = 0 means root document. Parent id != 0 means what?
// TODO: Perhaps id = 0 and parent != 0 means the row index offset for id calc?
// TODO: That means walk up parents until an instance of id = 0 allows that 0 id
// TODO: node to store the table index offset basis within the parent id field.
// TODO: This would genuinely allow cheaply calculating the actual row id of any
// TODO: node within a root document. This also means: subnode ids restart at 0.

/*

0: [id 0, par 0]
1: [id 1, par 0]
2: [id 0, par 2]  <- allows subnodes to directly index other nodes
3: [id 1, par 0]  <- parent index = row index - node id
4: [id 2, par 0]  <- offset basis is the parent of root (row index - node id)
5: [id 3, par 2]

*/

/// # Row-based JSON table
/// A table of JSON rows, each containing a key and value, with an ID and parent
/// that allows for tree navigation. Allows any row to be accessed by index, and
/// supports multiple document roots. All operations work using only the row and
/// the table itself to allow full tree traversal. The table is an append-only
/// data structure to preclude the need for sorting or rearranging or updating
/// indexes.
///
/// The row structure allows JSON Lines data by treating multiple root documents
/// with ID 0 as separate JSON documents.
///
/// Row ID 0 with parent > 0 is not invalid: it signals connection to another
/// JSON Lines document within the same table (data provenance for querying).
///
/// Invariants:
/// - Tables can contain multiple document roots.
/// - Root node ID is 0 in every document.
/// - Roots with table indices > 0 calculate their index as row index + node id.
/// - End of object/array is determined by next sibling with different parent
///   or end of table.
/// - Nodes with parent = node ID are not valid roots as they should use 0.
/// - Indentation level is equal to counting parent links until reaching a root.
/// - Roots do not need to be structured types (array/object).
/// - Is last node = next sibling has different parent and next node's parent is
///   not current node's parent.
/// - Is first child = current node ID = parent ID + 1
/// - Is sibling = current node's parent equals previous or next node's parent
/// -
/// - Child node IDs are strictly greater than parent ID.
/// - Child nodes must be contiguous within parent.
/// - End object/array is determined by next sibling with different parent or
///   end of table. Walk back up the tree by parent links until the next node's
///   parent is found, in which case it will equal the parent of the current
///   node's parent.
pub struct JsonTable
{
    rows: Vec<Row>,
}

impl JsonTable
{
    // Core navigation
    pub fn parent(&self, row_id: u32) -> Option<u32>
    {
        let row = &self.rows[row_id as usize];
        if row.par == row.id
        {
            None // Root
        }
        else
        {
            Some(row.par)
        }
    }

    pub fn next_sibling(&self, row_id: u32) -> Option<u32>
    {
        let row = &self.rows[row_id as usize];
        let next_id = row_id + 1;

        if next_id >= self.rows.len() as u32
        {
            return None;
        }

        let next = &self.rows[next_id as usize];
        if next.par == row.par { Some(next_id) } else { None }
    }

    pub fn first_child(&self, row_id: u32) -> Option<u32>
    {
        let row = &self.rows[row_id as usize];
        if !matches!(row.ty, RowType::Obj | RowType::Arr)
        {
            return None;
        }

        let child_id = row_id + 1;
        if child_id >= self.rows.len() as u32
        {
            return None;
        }

        let child = &self.rows[child_id as usize];
        if child.par == row_id { Some(child_id) } else { None }
    }

    // Multi-root support
    pub fn roots(&self) -> impl Iterator<Item = u32> + '_
    {
        self.rows
            .iter()
            .enumerate()
            .filter(|(i, r)| r.par == r.id)
            .map(|(i, _)| i as u32)
    }

    // For slicing
    pub fn extract_subtree(&self, root_id: u32) -> JsonTable
    {
        let mut new_rows = Vec::new();
        let mut stack = vec![root_id];

        while let Some(id) = stack.pop()
        {
            let mut row = self.rows[id as usize].clone();
            let new_id = new_rows.len() as u32;

            // Remap IDs
            if row.par == row.id
            {
                // Root
                row.id = new_id;
                row.par = new_id;
            }
            else
            {
                let parent_offset = row.par as usize;
                row.id = new_id;
                row.par = (new_id - (id - parent_offset as u32)) as u32;
            }

            new_rows.push(row);

            // Add children to stack
            if let Some(child) = self.first_child(id)
            {
                stack.push(child);
            }
        }

        JsonTable { rows: new_rows }
    }
}

pub struct JsonRoots
{
    /// Supports queries that fanout and create new JSON trees. This structure
    /// allows table-specific indexing and jumping to other trees seamlessly.
    /// Adjacency is important here so that queries can refer to distant trees.
    tables: Vec<JsonTable>,
}

mod v2
{
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    #[rustfmt::skip]
    pub enum RowType
    { Obj, Arr, Str, Num, Bit, Nil, }

    #[derive(Debug, Clone)]
    pub struct Row
    {
        /// Document root in the table. Multiple roots are JSON Lines documents.
        pub id: u32,
        pub par: u32,
        pub root: u32,
        pub key: String,
        pub val: String,
        pub ty: RowType,
    }

    /// id == row index
    /// par == parent row index
    /// root iff id == par
    impl Row {}

    /// - Tradeoff: not storing **row type *or* row id**.
    /// - Tradeoff: storing root allows reference to whole documents in the table
    /// - id == row index
    /// - par == parent row index
    /// - is_root iff: node par == node id
    ///
    /// **Benefits:**
    /// - append-only
    /// - O(1) parent lookup
    /// - O(1) node lookup
    /// - O(1) append
    /// - no rebasing
    /// - no ambiguity
    /// - perfect JSON Lines support
    /// - perfect subtree referencing
    /// - memory efficiency
    pub struct RowMini
    {
        /// Document root in the table. Multiple roots are JSON Lines documents.
        pub root: u32,
        pub par: u32,
        pub key: u32,
        pub val: u32,
        // * Not storing row type, storing val strings with leading quote, store
        // * key strings without quotes
    }

    impl RowMini
    {
        /// Doesn't store row ID so must provide it here.
        #[inline(always)]
        pub const fn is_root(&self, id: u32, table: &[Row]) -> bool
        {
            id == table[id as usize].par
        }

        #[inline(always)]
        pub const fn is_sibling(&self, table: &[Row]) -> bool
        {
            false
        }

        // TODO: This cannot classify strings unless the `"` are stored for each.
        // TODO: This is much more expensive than storing a byte for the token type.
        // * Solution: store first `"` for strings: no larger than RowType as u8
        // * Solution: keys don't need `"` because they're always strings, have
        // * the interned string be stored without quotes
        pub fn row_type(&self) -> RowType
        {
            // Lookup string value in string interner
            let val = ".".repeat(self.val as usize);
            let first_byte = val.as_bytes()[0];
            match first_byte
            {
                b'{' | b'}' => RowType::Obj,
                b'[' | b']' => RowType::Arr,
                b'"' => RowType::Str,
                b'-' | b'0' ..= b'9' => RowType::Num,
                b't' => RowType::Bit,
                b'f' => RowType::Bit,
                b'n' => RowType::Nil,
                _ => RowType::Nil,
            }
        }
    }

    pub trait JsonTable
    {
        fn parent(&self, row_id: u32) -> Option<u32>;
        fn next_sibling(&self, row_id: u32) -> Option<u32>;
        fn first_child(&self, row_id: u32) -> Option<u32>;
        fn roots(&self) -> impl Iterator<Item = u32> + '_;
        fn extract_subtree(&self, root_id: u32) -> Self;
        fn subnodes(&self, row_id: u32) -> impl Iterator<Item = &Row> + '_; // TODO: return node with id and parent remapped to 0 for subtree
    }

    fn r#try()
    {
        let table: &[Row] = &[];
        table.subnodes(0).for_each(|row| println!("{:?}", row));
        table.first_child(0);
    }

    impl JsonTable for &[Row]
    {
        fn parent(&self, row_id: u32) -> Option<u32>
        {
            let row = &self[row_id as usize];
            if row.par == row_id
            {
                None // Root
            }
            else
            {
                Some(row.par)
            }
        }

        fn next_sibling(&self, row_id: u32) -> Option<u32>
        {
            let row = &self[row_id as usize];
            let next_id = row_id + 1;

            if next_id >= self.len() as u32
            {
                return None;
            }

            let next = &self[next_id as usize];
            if next.par == row.par { Some(next_id) } else { None }
        }

        fn first_child(&self, row_id: u32) -> Option<u32>
        {
            let row = &self[row_id as usize];
            if !matches!(row.ty, RowType::Obj | RowType::Arr)
            {
                return None;
            }

            let child_id = row_id + 1;
            if child_id >= self.len() as u32
            {
                return None;
            }

            let child = &self[child_id as usize];
            if child.par == row_id { Some(child_id) } else { None }
        }

        fn roots(&self) -> impl Iterator<Item = u32> + '_
        {
            self.iter()
                .enumerate()
                .filter(|(i, r)| r.par == *i as u32)
                .map(|(i, _)| i as u32)
        }

        fn extract_subtree(&self, root_id: u32) -> Self
        {
            unimplemented!()
        }

        fn subnodes(&self, row_id: u32) -> impl Iterator<Item = &Row> + '_
        {
            self.iter()
                .enumerate()
                .filter(move |(id, row)| {
                    *id as u32 > row_id && row_id == row.par
                })
                .map(|(_, row)| row)
        }
    }
}
