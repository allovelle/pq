//! Convert JSON to rows
//!
use std::convert::From;

use serde_json::{Number, Value};

fn main() -> Result<(), std::io::Error>
{
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

    let mut table = Vec::new();
    traverse(&mut table, String::new(), value, 0, 0);
    view_table(&table);

    Ok(())
}

fn view_table(table: &Vec<Row>)
{
    for row in table
    {
        println!("{row:?}");
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum RowType { Arr, Obj, Nil, Bool, Txt, Num, }

/// Invariant: Self::Id is the index within it's container.
#[derive(Debug, Clone)]
struct Row
{
    #[cfg(not(feature = "implicit_row_ids"))]
    id: u32,
    parent: u32,
    key: String,
    value: String,
    ty: RowType,
    #[cfg(not(feature = "log_n_indentation"))]
    indent: u32,
}

impl Row
{
    fn new<K: ToString, V: ToString>(
        id: u32,
        parent: u32,
        key: K,
        value: V,
        ty: RowType,
        indent: u32,
    ) -> Self
    {
        let key = key.to_string();
        let value = value.to_string();
        Self { id, parent, key, value, ty, indent }
    }

    fn nil<K: ToString>(id: u32, parent: u32, key: K, indent: u32) -> Self
    {
        Self::new(id, parent, key.to_string(), "null", RowType::Nil, indent)
    }

    fn txt<K: ToString, V: ToString>(
        id: u32,
        parent: u32,
        key: K,
        val: V,
        indent: u32,
    ) -> Self
    {
        Self::new(id, parent, key, val, RowType::Txt, indent)
    }

    fn boolean<K: ToString>(
        id: u32,
        parent: u32,
        key: K,
        val: bool,
        indent: u32,
    ) -> Self
    {
        Self::new(id, parent, key, val.to_string(), RowType::Bool, indent)
    }

    fn num<K: ToString, V>(
        id: u32,
        parent: u32,
        key: K,
        val: V,
        indent: u32,
    ) -> Self
    where
        Number: From<V>,
    {
        let val = Number::from(val).to_string();
        Self::new(id, parent, key, val, RowType::Num, indent)
    }

    fn arr<K: ToString>(id: u32, parent: u32, key: K, indent: u32) -> Self
    {
        Self::new(id, parent, key, String::new(), RowType::Arr, indent)
    }

    fn obj<K: ToString>(id: u32, parent: u32, key: K, indent: u32) -> Self
    {
        Self::new(id, parent, key, String::new(), RowType::Obj, indent)
    }

    fn indent_level(&self, table: &Vec<Row>) -> u32
    {
        if cfg!(feature = "log_n_indentation")
        {
            let mut parent = self.parent;
            let mut levels_deep = 0;
            while let Some(row) = table.get(parent as usize)
            {
                parent = row.parent;
                levels_deep += 1;
            }
            levels_deep
        }
        else
        {
            self.indent
        }
    }
}

fn traverse(
    table: &mut Vec<Row>,
    key: String,
    value: Value,
    parent: u32,
    indent: u32,
)
{
    let new_id = table.len() as u32;

    // Return Some/None based on value/container?
    match value
    {
        Value::Null => table.push(Row::nil(new_id, parent, key, indent)),
        Value::Bool(tf) =>
        {
            table.push(Row::boolean(new_id, parent, key, tf, indent))
        }
        Value::Number(num) =>
        {
            table.push(Row::num(new_id, parent, key, num, indent))
        }
        Value::String(txt) =>
        {
            table.push(Row::txt(new_id, parent, key, txt, indent))
        }
        Value::Array(arr) =>
        {
            table.push(Row::arr(new_id, parent, key.clone(), indent));
            for element in arr
            {
                traverse(table, key.clone(), element, new_id, indent + 1);
            }
        }
        Value::Object(map) =>
        {
            table.push(Row::obj(new_id, parent, key, 0));
            for (name, element) in map
            {
                traverse(table, name, element, new_id, indent + 1);
            }
        }
    }
}
