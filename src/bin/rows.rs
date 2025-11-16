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

/// Idea: for any single row, render with proper indents using only the table
fn view_table(table: &Vec<Row>)
{
    use RowType::*;

    let style_new_arr = <&str as Stylize>::red;
    let style_new_obj = <&str as Stylize>::red;
    let style_nil = <&str as Stylize>::red;
    let style_bool = <&str as Stylize>::yellow;
    let style_txt = <&str as Stylize>::blue;
    let style_num = <&str as Stylize>::cyan;

    // let style_value = |val: &Row| match val.ty
    // {
    //     Nil => <&str as Stylize>::blue(&val.value),
    //     Bool => todo!(),
    //     Txt => todo!(),
    //     Num => todo!(),
    //     Arr | Obj => todo!(),
    // };

    for row in table.iter().skip(1)
    {
        println!("{row:?}");
    }

    let mut rows = table.iter().peekable();

    while let Some(row) = rows.next()
    {
        let key = &format!("{:?}", row.key);

        let mut val = row.value.clone();
        if row.ty == Txt
        {
            val = format!("{:?}", &row.value);
        }

        let key_style = <&str as Stylize>::green;
        let val_style = match row.ty
        {
            Nil => <&str as Stylize>::red,
            Bool => Stylize::yellow,
            Txt => Stylize::blue,
            Num => Stylize::cyan,
            _ => Stylize::green,
        };

        let tab = "    ".repeat(row.indent as usize);

        // * Rendering rules:
        // Omit key if id is 0 or parent is arr
        // Emit comma if next row is sibling
        // Omit comma if next row is not sibling
        // Emit ] if ty is arr and next row is not sibling
        // Emit } if ty is obj and next row is not sibling

        // ! Emit comma either way, unless it's the toplevel or last elem

        // TODO: Emit color for all quotes (keys & txt vals). Follow `bat`

        /*
               match row.ty
               {
                   Obj if row.id == 0 =>
                   {
                       // ? This could be: Row {val: "{", ty: Obj}
                       println!("{tab}{{");

                       // * Don't print the key
                   }
                   Obj if row.id > 0 =>
                   {
                       // ? This could be: Row {val: "{", ty: Obj}
                       println!("{tab}{}: {{", key_style(key));
                   }
                   Arr =>
                   {
                       // ? This could be: Row {val: "[", ty: Arr}
                       println!("{tab}{}: [", key_style(key));

                       // TODO: EmitCommand(Indent, NewArr, StayOnOneLine)
                   }
                   Obj | Nil | Bool | Txt | Num
                       if table
                           .get(row.parent as usize)
                           .filter(|prev| prev.ty == Arr)
                           .is_some() =>
                   {
                       // TODO: table.get(row.id + 1).filter().map_or()
                       let comma = rows
                           .peek()
                           .filter(|next| row.parent == next.parent)
                           .map_or("", |_| ",");
                       println!("{tab}{}{comma}", val_style(&val));

                       // * Don't print the key
                   }
                   Obj | Nil | Bool | Txt | Num =>
                   {
                       // * Place comma if next row *is* a sibling (continue obj/arr)
                       // TODO: table.get(row.id + 1).filter().map_or()
                       let comma = rows
                           .peek()
                           .filter(|next| row.parent == next.parent)
                           .map_or("", |_| ",");

                       println!("{tab}{}: {}{comma}", key_style(key), val_style(&val));
                   }
               }
        */

        // ? Compare next node indent: if less, don't place comma, place end }]

        // ! Emit key in all cases unless inside arr or is first element (id 0)
        // ! Emit end brace ] } when *end* arr|obj
        // ! Emit comma in *all* cases unless *new* arr|obj
        // ! Omit comma when next row is not sibling
        // ! Increment indent when ...
        // ! Decrement indent when ...

        // let braces = ["{", ""];
        // let blocks = ["[", ""];
        // let commas = [",", ""];
        // let brace = braces[usize::from(row.ty == Obj)];
        // let block = blocks[usize::from(row.ty == Arr)];
        // let comma = commas[usize::from(row.ty == Obj || row.ty == Arr)];
        // let brace = ['\0', '{'][usize::from(row.ty == Obj)];
        // let block = ['\0', '['][usize::from(row.ty == Arr)];
        // let comma = ["", ","][usize::from(row.ty != Obj || row.ty != Arr)];
        // let begin = String::from(brace) + block;

        let begin = [["", ""], ["[", "{"]]
            [(row.ty == Obj || row.ty == Arr) as usize]
            [(row.ty == Obj) as usize];
        let comma = [",", ""][(row.ty == Obj || row.ty == Arr) as usize];

        // table.get(row.id + 1)

        // The comma should not be part of the table the same way the indent
        // shouldn't be. They can both be calculated using the row and table.
        // ? Should the brace be a val?
        println!("{}{}{}{}{}{}", tab, key, ": ".to_owned(), begin, val, comma);

        let parent = table.get(row.parent as usize);
        let not_sibling = rows.peek().filter(|next| row.parent != next.parent);

        // * Put ] or } to finish each parent collection but don't place commas
        // TODO: if table.get(row.id + 1).is_none() {}
        // * Place end braces ] and } for all parents all the way up the tree
        if rows.peek().is_none()
        {
            let mut prev = row;
            while let Some(parent) = table.get(prev.parent as usize)
                && prev.id > parent.id
            {
                let tab = "    ".repeat(parent.indent as usize);
                match parent.ty
                {
                    Arr => println!("{tab}]"),
                    Obj => println!("{tab}}}"),
                    _ => todo!(),
                }
                prev = parent;
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
