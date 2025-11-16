//! Convert JSON to rows
//!
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

fn view_table(table: &Vec<Row>)
{
    for row in table.iter().skip(1)
    {
        println!("{row:?}");
    }

    // Idea: for any given row, render it in JSON with proper indentation
    // Row { id: 7, parent: 3, key: "3", value: "818", ty: Num, indent: 2 };
    // `........"3": 818,`
    // ? Add row types for commas with id 0 and parent=node?

    let mut rows = table.iter().peekable();

    while let Some(row) = rows.next()
    {
        let tab = "    ".repeat(row.indent as usize);
        let key = row.key.clone();
        let mut val = row.value.clone();

        let key = format!("{:?}", key);
        if row.ty == RowType::Txt
        {
            val = format!("{:?}", &row.value);
        }

        let key_style = <&str as Stylize>::green;
        let val_style = match row.ty
        {
            RowType::Nil => <&str as Stylize>::red,
            RowType::Bool => Stylize::yellow,
            RowType::Txt => Stylize::blue,
            RowType::Num => Stylize::cyan,
            _ => Stylize::green,
        };

        if row.ty == RowType::Obj && row.id == 0
        {
            println!("{tab}{{")
        }
        else if row.ty == RowType::Obj && row.id > 0
        {
            println!("{tab}{}: {{", key_style(&key));
        }
        else if row.ty == RowType::Arr
        {
            // TODO: EmitCommand(Indent, NewArr, StayOnOneLine)
            println!("{tab}{}: [", key_style(&key));
        }
        else
        {
            // * Place comma if next element is not a sibling (obj/arr closing)
            let comma = rows
                .peek()
                .filter(|next| row.parent == next.parent)
                .map_or("", |_| ",");

            println!("{tab}{}: {}{comma}", key_style(&key), val_style(&val));
        }

        // * Put ] or } for each parent element until the root
        if rows.peek().is_none()
        {
            let mut prev_parent = row;
            while let Some(prev) = table.get(prev_parent.parent as usize)
                && prev_parent.id > prev.id
            {
                // Commas not needed since all of these will be the last element
                // of the parent collection, all the way to the root
                let tab = "    ".repeat(prev.indent as usize);
                match prev.ty
                {
                    RowType::Arr => println!("{tab}]"),
                    RowType::Obj => println!("{tab}}}"),
                    _ => todo!(),
                }

                prev_parent = prev;
            }
        }
    }
}

fn _format_table(_table: &Vec<Row>) {}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
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

fn traverse(table: &mut Vec<Row>, key: String, value: Value, parent: u32)
{
    let new_id = table.len() as u32;
    // let indent = table[parent as usize].indent + 1;
    let indent = table.get(parent as usize).map_or(0, |row| row.indent + 1);

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
            for (i, element) in arr.into_iter().enumerate()
            {
                traverse(table, i.to_string(), element, new_id);
            }
        }
        Value::Object(map) =>
        {
            table.push(Row::obj(new_id, parent, key, 0));
            for (name, element) in map
            {
                traverse(table, name, element, new_id);
            }
        }
    }
}
