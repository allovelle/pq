use crossterm::style::{Color, Stylize};
use serde_json::{Number, Value};
use std::convert::From;

const DEBUG_TAGS: bool = true;

fn main() -> Result<(), std::io::Error>
{
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

    let mut table = Vec::new();
    traverse(&mut table, String::new(), value, 0);
    view_table(&table);

    Ok(())
}

fn udx<T>(val: T) -> usize
where
    T: TryInto<usize>,
    <T as TryInto<usize>>::Error: std::fmt::Debug,
{
    val.try_into().expect("failed to convert")
}

/// Idea: for any single row, render with proper indents using only the table
fn view_table(table: &[Row])
{
    use RowType::*;

    // ! Invariants:
    // ! Table must not allow row modification (immutable)
    // Prevents: row reordering, even with ID updates (obj|arr child reorder)
    // ! Row IDs must start at 0 and increase only by 1 (table indices == ids).
    // Allows: parent, first child, & next offset-calculation

    // Color theme settings:
    let style_key = <&str as Stylize>::green;
    let style_quote_key = <&str as Stylize>::dark_green;
    let style_open = <&str as Stylize>::red;
    let style_end = <&str as Stylize>::red;
    let style_quote_val = <&str as Stylize>::dark_red;
    let style_txt = <&str as Stylize>::red;
    let style_nil = <&str as Stylize>::red;
    let style_num = <&str as Stylize>::cyan;
    let _style_dot = <&str as Stylize>::cyan; // floats
    let style_bit = <&str as Stylize>::yellow; // bool

    let mut accumulate_indent = 0;

    // ! Everything looks to be in order except for root node key showing and
    // ! dedentation commas missing

    // ! Everything looks to be in order except for root node key showing and
    // ! dedentation commas missing

    // ! Everything looks to be in order except for root node key showing and
    // ! dedentation commas missing

    // ! Everything looks to be in order except for root node key showing and
    // ! dedentation commas missing

    for row in table
    {
        // TODO: Print the calculated attributes for each row
        let parent = table.get(udx(row.parent)).unwrap_or(row);
        let first = table.get(udx(row.parent) + 1).unwrap_or(row);
        let next = table.get(udx(row.id) + 1).unwrap_or(row);

        debug_assert!(matches!(parent.ty, Obj | Arr), "parent isn't obj/arr"); // TODO: REMOVE THIS
        debug_assert_eq!(first.parent, row.parent, "sibling isn't sibling"); // TODO: REMOVE THIS

        let key = udx(parent.ty != Arr && row.id != 0);
        let arr = udx(!matches!(row.ty, Obj));
        let var = udx(matches!(row.ty, Nil | Bit | Num | Txt));
        let val = ["object, ", "array, ", "value, "][arr + var];
        let is_parent = row.id == next.parent;
        let empty = matches!(row.ty, Obj | Arr) && row.id != next.parent;
        let last = next.id == row.id || next.parent < row.parent; // First row, last row, last elem in collection, or ending bracket/brace
        let first_is_not_self = row.id != 0 && first.id < row.id;
        let next_same_parent = row.id != 0 && next.parent == row.parent;
        let is_sibling = first_is_not_self || next_same_parent;

        // Tags (except for implicit rows)
        let tag_lvl = "  ".repeat(accumulate_indent);
        let tags = format!(
            "{}{}{}{}{}{}{}{}",
            "|| ".red(),
            tag_lvl,
            "key, ".repeat(key),
            val,
            "parent, ".repeat(udx(is_parent)),
            "empty, ".repeat(udx(empty)),
            "sibling, ".repeat(udx(is_sibling)),
            "last".repeat(udx(last)),
        );

        // Key-Value (except for implicit brackend end rows)
        let indent = "    ".repeat(accumulate_indent);
        let colon = ": ";
        let comma = ",".repeat(udx((empty || !is_parent) && !last));
        let key =
            format!("{0}{1}{0}", style_quote_key("\""), style_key(&row.key));
        let val = match row.ty
        {
            Arr if empty =>
            {
                let open = style_open("[");
                let end = style_end(if empty { "]" } else { "" });
                format!("{open}{end}")
            }
            Arr => style_open("[").to_string(),
            Obj if empty =>
            {
                let open = style_open("{");
                let end = style_end(if empty { "}" } else { "" });
                format!("{open}{end}")
            }
            Obj => style_open("{").to_string(),
            Nil => style_nil(&row.value).to_string(),
            Bit => style_bit(&row.value).to_string(),
            Txt =>
            {
                let quote = style_quote_val("\"");
                let txt = style_txt(&row.value);
                format!("{quote}{txt}{quote}")
            }
            Num => style_num(&row.value).to_string(),
        };

        if DEBUG_TAGS
        {
            println!("{tags:<66}|{indent}{key}{colon}{val}{comma}");
        }
        else
        {
            println!("{indent}{key}{colon}{val}{comma}");
        };

        if is_parent && !empty
        {
            accumulate_indent += 1;
        }
        else if last
        {
            accumulate_indent -= 1;
        }

        // End means end of collection (place end brackets all the way up)
        let is_end = |node: &Row| {
            let parent = table.get(node.parent as usize).unwrap_or(node);
            let first = table.get(node.parent as usize + 1).unwrap_or(node);
            let next = table.get(node.id as usize + 1).unwrap_or(node);
            next.id == node.id || next.parent < node.parent
        };

        let mut node = row;
        let mut increment_dedent = accumulate_indent;
        while node.parent > next.parent || is_end(node)
        {
            let parent = table.get(udx(node.parent)).unwrap_or(node);
            let val = ["array", "object"][udx(parent.ty == Obj)];
            let tag_lvl = " ".repeat(increment_dedent);
            let tags =
                format!("{}{}<implicit end {}>", "|| ".red(), tag_lvl, val);
            let indent = "    ".repeat(increment_dedent);
            let end = style_end(["]", "}"][udx(parent.ty == Obj)]);

            // TODO: CALCULATE THIS USING PARENT IDS NOT ROW IDS. THE 'NEXT' ROW
            // TODO: IS THE PREVIOUS PARENT ALL THE WAY TO THE ROOT.
            let comma = ",".repeat(udx(!last));

            if DEBUG_TAGS
            {
                println!("{tags:<66}|{indent}{end}{comma}");
            }
            else
            {
                println!("{indent}{end}{comma}");
            };

            node = parent;
            increment_dedent -= 1;
        }

        if accumulate_indent - increment_dedent > 1
        {
            accumulate_indent -= 1;
        }

        // TODO: Merge this up into the above, just break out for root node
        if let Some(root) = table.first()
        {
            let last_before_root = next.id == row.id;
            if last_before_root
            {
                let tag_lvl = "  ".repeat(increment_dedent);
                let tags =
                    format!("{}{}<implicit end of root>", "|| ".red(), tag_lvl);
                let indent = "    ".repeat(increment_dedent);
                let end = style_end(["]", "}"][udx(root.ty == Obj)]);

                if DEBUG_TAGS
                {
                    println!("{tags:<66}|{indent}{end}");
                }
                else
                {
                    println!("{indent}{end}");
                };
            }
        }
    }

    // let header = ("id", "parent", "key", "value", "type", "indent");
    // println!(
    //     "{}",
    //     format!(
    //         "{:<4.4}{:<8.7}{:<22.21}{:<22.21}{:<6.6}{:<6.6}",
    //         header.0, header.1, header.2, header.3, header.4, header.5
    //     )
    //     .red()
    // );
    for (id, row) in table.iter().enumerate().take(0)
    {
        println!(
            "{:<4.4}{:<8.7}{:<22.21}{:<22.21}{:<6.6}{:<6.6}",
            row.id.to_string(),
            row.parent.to_string(),
            row.key,
            row.value,
            format!("{:?}", row.ty),
            row.indent_level(table)
        );
    }

    for row in table.iter().take(0)
    {
        // TODO: Emit {[ends]} *first*: prev elems could've been multi-lvls-deep

        // TODO: tabulation with cost minimization to hit 80 char line-len-lim
        // TODO: use the 'only take 1/2' rule (example) or other constraints

        // Prepare styled key-value emission:

        // ? let parent = table.get(row.parent as usize).expect("invalid parent id");
        // ? let next = table.get(row.id as usize + 1);
        // ? Why not next=tab.get(row.id + 1).or(row) since root.parent=root?
        let parent = table.get(row.parent as usize).unwrap_or(row);
        let next = table.get(row.id as usize + 1).unwrap_or(row);

        let tab = "    ".repeat(row.indent as usize);
        let key = Stylize::blue(format!(
            "{1}{0}{1}",
            row.key.repeat((row.id != 0 && parent.ty != Arr) as usize),
            "\"".repeat((row.id != 0 && parent.ty != Arr) as usize),
        ));

        let colon = Stylize::underline_red(
            ["", ": "][(row.id != 0 && parent.ty != Arr) as usize],
        );
        let val = Stylize::yellow(row.value.clone());
        let quote = Stylize::yellow(["", "\""][(row.ty == Txt) as usize]);

        // Omit comma when last/lone for all types
        // Omit comma when obj/arr unless empty
        // let empty = matches!(row.ty, Obj | Arr if next.parent != row.id);
        // let last = next.parent != row.parent && next.parent != row.id;
        // let emit_comma = lone || (empty && !last);

        // Justification: next.parent can be row.id if parent. Last if less.
        let last = next.parent < row.parent;

        // Justification: nested elements have higher id than parents
        let lone = next.parent <= row.parent;

        let dense_collection = true;
        let empty_collection = true;
        let omit = (last || lone || dense_collection) && !empty_collection;
        let omit = (last || lone);

        // prev element can be a child, yet prev elements are not considered.

        let comma = Stylize::grey(",".repeat(!omit as usize));

        println!("{}{}{}{}{}{}{}", tab, key, colon, quote, val, quote, comma);
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
            table.push(Row::bit(new_id, parent, key, tf, indent))
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
enum RowType { Arr, Obj, Nil, Bit, Txt, Num, }

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

    fn bit<K: ToString>(
        id: u32,
        parent: u32,
        key: K,
        val: bool,
        indent: u32,
    ) -> Self
    {
        Self::new(id, parent, key, val.to_string(), RowType::Bit, indent)
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

    fn indent_level(&self, table: &[Row]) -> usize
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
