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

    for (id, row) in table.iter().enumerate()
    {
        println!("ROW[{id}]: {row:?}");
    }

    for (id, row) in table.iter().enumerate()
    {
        debug_assert_eq!(row.id, id as u32);

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
        // ! Emit comma in *all* cases except *new* arr|obj or *end* elem
        // ! Omit colon when root row or arr elem
        // ! Increment indent when ...
        // ! Decrement indent when ...

        // Row { id: 0, parent: 0, key: "", value: "", ty: Obj, indent: 0 }
        // Row { id: 1, parent: 0, key: "name", value: "Allovelle", ty: Txt, indent: 1 }

        /*
        emit key
        emit colon
        emit new arr/obj
        emit val
        emit comma
        emit end arr/obj
        */
        let tab = "    ".repeat(row.indent_level(table) as usize);

        let parent = table
            .get(row.parent as usize)
            .expect("All rows should have a parent, even the root row");

        let Some(next) = table.get(id + 1)
        else
        {
            let mut prev = row;
            return while let Some(parent) = table.get(prev.parent as usize)
                && prev.id != 0
            {
                // && prev.id > parent.id
                let tab = "    ".repeat(parent.indent_level(table) as usize);
                let end = ["]", "}"][(parent.ty == Obj) as usize];
                println!("{tab}{end}");
                prev = parent;
            };
        };

        let is_first_elem = id == 0;
        let in_arr_or_first_elem = id == 0
            || table
                .get(row.parent as usize)
                .map(|parent| parent.ty == Arr)
                .unwrap_or_default();
        let is_last_sibling = table
            .get(id + 1)
            .map(|next| row.parent != next.parent)
            .unwrap_or_default();

        let _breadcrumbs = [Arr, Obj, Arr, Arr, Obj];

        // let brace = ['\0', '{'][usize::from(row.ty == Obj)];
        // let block = ['\0', '['][usize::from(row.ty == Arr)];
        // let comma = ["", ","][usize::from(row.ty != Obj || row.ty != Arr)];
        // let begin = String::from(brace) + block;
        // let close = "";

        // * Predicates for emission rules
        // let last_sibling = row.parent != next.parent; // ************** ! wrong when it's first child node
        let last_sibling = next.parent < row.parent;

        let is_dedenting_possibly_more_than_1 = row.id <= next.parent;
        let is_parent = row.id == next.parent;
        // let is_empty = !is_parent && matches!(row.ty, Obj | Arr);
        let is_empty = matches!(row.ty, Obj | Arr if row.id != next.parent);

        let is_root = row.id == 0;
        let is_new = row.id == next.parent; // *******************
        let is_end = next.parent == parent.parent; // *******************

        // Doesn't work when it's a new arr or new obj
        // let place_comma = !last_sibling && !is_empty;
        let place_comma = !last_sibling && !is_parent && !is_empty;

        // ! This can be fixed by: storing { or {} during parsing (it's easy)
        // ? [is_parent as usize * 2 + (row.ty == Obj) as usize]
        let begin = [[["", ""], ["", ""]], [["[]", "{}"], ["[", "{"]]]
            [(matches!(row.ty, Obj | Arr)) as usize][is_parent as usize]
            [(row.ty == Obj) as usize];

        // let comma = [",", ""]
        //     [(row.ty == Obj || row.ty == Arr || last_sibling) as usize];
        let comma = ["", ","][(place_comma) as usize]; // * wrong when it's end arr/obj

        let colon = [": ", ""][(row.id == 0 || parent.ty == Arr) as usize];
        let close = [["", ""], ["]", "}"]][last_sibling as usize]
            [(parent.ty == Obj) as usize];

        // The comma should not be part of the table the same way the indent
        // shouldn't be. They can both be calculated using the row and table.
        // ? Should the brace be a val?

        let key = [key, ""][(row.id == 0 || parent.ty == Arr) as usize];
        let key = key.red();

        println!(
            "{}{}{}{}{}{}",
            tab,
            key,
            colon,
            begin,
            val,
            comma.on_dark_green()
        );

        if last_sibling && !is_root
        {
            println!("  {}, {}, {}", parent.id, row.id, next.id);
            // Root has indent level of 0 (underflow)
            let tab = "    ".repeat(row.indent_level(table) as usize - 1);
            println!("{}{}", tab, close);
        }

        continue;

        // If next row and not sibling: no comma, dedent, and place } or ]
        if let Some(next) = table.get(row.id as usize + 1)
            && row.parent != next.parent
        {
            println!("{}{}", tab, comma);
        }
        // If no next row, end output
        else
        {
            // * Emit opens ] or } for each parent but don't place commas
            // TODO: if table.get(row.id + 1).is_none() {}
            // * Emit ends ] and } for all parents all the way up the tree

            // let mut prev = row;
            // while let Some(parent) = table.get(prev.parent as usize)
            //     && prev.id > parent.id
            // {
            //     let tab = "    ".repeat(parent.indent as usize);
            //     match parent.ty
            //     {
            //         Arr => println!("{tab}]"),
            //         Obj => println!("{tab}}}"),
            //         _ => todo!(),
            //     }
            //     prev = parent;
            // }
        }

        // let parent = table.get(row.parent as usize);
        // let not_sibling = rows.peek().filter(|next| row.parent != next.parent);

        // * Put ] or } to finish each parent collection but don't place commas
        // TODO: if table.get(row.id + 1).is_none() {}
        // * Place end braces ] and } for all parents all the way up the tree
        /*
        if table.get(row.id as usize + 1).is_none()
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
        */
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
