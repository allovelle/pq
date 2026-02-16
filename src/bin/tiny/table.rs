use crate::parser::{Row, RowType};

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
