use crossterm::style::Stylize;
use serde_json::{Number, Value};
use std::convert::From;

fn main() -> Result<(), std::io::Error>
{
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

    let mut table = Vec::new();
    traverse(&mut table, String::new(), value, 0);
    view_table(&table);

    Ok(())
}

/// Idea: for any single row, render with proper indents using only the table
fn view_table(table: &[Row])
{
    use RowType::*;

    for (id, row) in table.iter().enumerate()
    {
        println!("ROW[{id}]: {row:?}");
    }

    for row in table
    {
        // TODO: Emit {[ends]} *first*: prev elems could've been multi-lvls-deep

        // Prepare styled key-value emission:
        let tab = "    ".repeat(row.indent as usize);
        let key = Stylize::blue(format!("{:?}", row.key));
        // let val = Stylize::yellow(format!("{:?}", row.value));

        // let fmt_dbg = matches!(row.ty, Obj | Arr | Txt);
        let val = Stylize::yellow(row.value.clone());
        let quote = Stylize::yellow(["", "\""][(row.ty == Txt) as usize]);

        // TODO: [parent][row][next]

        println!("{}{}: {}{}{}", tab, key, quote, val, quote);
    }
}

fn traverse(table: &mut Vec<Row>, key: String, value: Value, parent: u32)
{
    let new_id = table.len() as u32;
    let indent = table.get(parent as usize).map_or(0, |row| row.indent + 1);

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
            for (i, element) in arr.into_iter().enumerate()
            {
                traverse(table, i.to_string(), element, new_id);
            }
        }
        Value::Object(map) =>
        {
            table.push(Row::obj(new_id, parent, key, indent));
            for (name, element) in map
            {
                traverse(table, name, element, new_id);
            }
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum RowType { Arr, Obj, Nil, Bool, Txt, Num, }

/// Invariant: Self::Id is the index within it's container.
#[derive(Debug, Clone)]
struct Row
{
    id: u32,
    parent: u32,
    key: String,
    value: String,
    ty: RowType,
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
        Self::new(id, parent, key, "[", RowType::Arr, indent)
    }

    fn obj<K: ToString>(id: u32, parent: u32, key: K, indent: u32) -> Self
    {
        Self::new(id, parent, key, "{", RowType::Obj, indent)
    }

    fn indent_level(&self, table: &[Row]) -> u32
    {
        let (mut indents, mut row) = (0, self);

        while let Some(next) = table.get(row.parent as usize)
            && row.id != 0
        {
            row = next;
            indents += 1;
        }

        indents
    }
}
