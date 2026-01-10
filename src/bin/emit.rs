#![allow(clippy::unit_arg)]

use crossterm::style::{Color, Stylize};
use pq::{
    PqResult,
    tok::{Tok, tokenize},
};
use serde_json::{Number, Value};
use std::{
    convert::From,
    env, fmt, fs,
    io::{self, IsTerminal, Write},
};
use strum::VariantNames;

const DEBUG_TAGS: bool = true;

fn udx<T>(val: T) -> usize
where
    T: TryInto<usize>,
    <T as TryInto<usize>>::Error: fmt::Debug,
{
    val.try_into().expect("failed to convert")
}

// #[derive(Default)]
// struct Theme<V>
// {
//     enabled: bool,
//     style_key: <V as Stylize>::green,
//     style_quote_key = <V as Stylize>::dark_green,
// }

// impl Theme
// {
//     fn txt() -> Self
//     {
//         Self::default()
//     }

//     fn build() {}
// }

// TODO: Can style and output individual rows
// TODO: JsonStyler::new(theme1).key("k1").id(0).empty(false).style();
// TODO: JsonStyler::new(theme2).key("k1").val(3.14).style();

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

    /*
    TODO: use text attributes in addition to colors

    use crossterm::style::Attribute;
    println!(
        "{} Underlined {} No Underline",
        Attribute::Underlined,
        Attribute::NoUnderline
    );
    */

    let mut accumulate_indent = 0;

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
        let key =
            format!("{0}{1}{0}", style_quote_key("\""), style_key(&row.key))
                .repeat(udx(row.id != 0));
        let colon = ": ".repeat(udx(row.id != 0));
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
        let comma = ",".repeat(udx((empty || !is_parent) && !last));

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
        while node.parent > next.parent || (next.id == node.id)
        {
            let parent = table.get(udx(node.parent)).unwrap_or(node);

            let tags = format!(
                "{} node{}{{par={}}} next{}{{par={}}} row{}{{par={}}}",
                "||".grey(),
                node.id,
                node.parent,
                next.id,
                next.parent,
                row.id,
                row.parent
            );

            // NEED TO DETERMINE IS LAST
            // parent.is_last_from_pov(row <- specifically (landmark))
            // Parent is nested deeper than next parent OR node is last in file
            let brace = if node.parent > next.parent || node.id == next.id
            {
                if parent.ty == Obj { "}" } else { "]" }
            }
            else
            {
                ""
            };

            // Next node is sibling of current node's parent
            let comma = if next.parent == parent.parent { "," } else { "" };

            let indent = "    ".repeat(increment_dedent);
            if DEBUG_TAGS
            {
                println!("{tags:<66}|{}{}{}", indent, brace.red(), comma);
            }
            else
            {
                println!("{}{}{}", indent, brace.red(), comma);
            }

            node = parent;
            increment_dedent -= 1;
        }

        if accumulate_indent - increment_dedent > 1
        {
            accumulate_indent -= 1;
        }

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

    // TODO: tabulation with cost minimization to hit 80 char line-len-lim
    // TODO: use the 'only take 1/2' rule (example) or other constraints
}

fn traverse(table: &mut Vec<Row>, key: String, value: Value, parent: u32)
{
    use RowType::*;

    let new_id = table.len() as u32;

    match value
    {
        Value::Null => table.push(Row::new(new_id, parent, key, "", Nil)),
        Value::Bool(tf) => table.push(Row::new(new_id, parent, key, tf, Bit)),
        Value::Number(num) =>
        {
            table.push(Row::new(new_id, parent, key, num, Num))
        }
        Value::String(txt) =>
        {
            table.push(Row::new(new_id, parent, key, txt, Txt))
        }
        Value::Array(arr) =>
        {
            table.push(Row::new(new_id, parent, key.clone(), "[", Arr));

            for (i, element) in arr.into_iter().enumerate()
            {
                traverse(table, i.to_string(), element, new_id);
            }
        }
        Value::Object(map) =>
        {
            table.push(Row::new(new_id, parent, key, "{", Obj));

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
}

impl Row
{
    fn new<K: ToString, V: ToString>(
        id: u32,
        parent: u32,
        key: K,
        value: V,
        ty: RowType,
    ) -> Self
    {
        let key = key.to_string();
        let value = value.to_string();
        Self { id, parent, key, value, ty }
    }
}

fn tokens_to_rows(tokens: Vec<Tok>) -> io::Result<Vec<Row>>
{
    use RowType::*;
    use Tok::*;
    use TokTy::*;

    let mut rows = Vec::new();

    let mut container_stack: Vec<RowType> = Vec::new();
    let mut parent_stack: Vec<u32> = vec![u32::MAX]; // root sentinel
    let mut index_stack: Vec<u32> = vec![0];

    let mut current_key = String::new();

    let mut state = State::BEG;

    let mut i = 0;
    while i < tokens.len()
    {
        let tok = &tokens[i];

        match tok
        {
            ObjectOpen =>
            {
                let key = if matches!(container_stack.last(), Some(Obj))
                {
                    std::mem::take(&mut current_key)
                }
                else
                {
                    index_stack.last().unwrap().to_string()
                };

                let parent = *parent_stack.last().unwrap();

                let row_id = rows.len() as u32;
                rows.push(Row::new(
                    *index_stack.last().unwrap(),
                    parent,
                    key,
                    "{",
                    RowType::Obj,
                ));

                *index_stack.last_mut().unwrap() += 1;

                container_stack.push(Obj);
                parent_stack.push(row_id);
                index_stack.push(0);

                state = State::KEY;
            }

            ArrayOpen =>
            {
                let key = index_stack.last().unwrap().to_string();
                let parent = *parent_stack.last().unwrap();

                let row_id = rows.len() as u32;
                rows.push(Row::new(
                    *index_stack.last().unwrap(),
                    parent,
                    key,
                    "[",
                    RowType::Arr,
                ));

                *index_stack.last_mut().unwrap() += 1;

                container_stack.push(Arr);
                parent_stack.push(row_id);
                index_stack.push(0);

                state = State::VAL;
            }

            ObjectClose | ArrayClose =>
            {
                container_stack.pop();
                parent_stack.pop();
                index_stack.pop();
                state = State::END;
            }

            Text(s) =>
            {
                if state == State::KEY
                {
                    current_key = s.clone();
                    state = State::COL;
                }
                else
                {
                    let key = if matches!(container_stack.last(), Some(Obj))
                    {
                        std::mem::take(&mut current_key)
                    }
                    else
                    {
                        index_stack.last().unwrap().to_string()
                    };

                    let parent = *parent_stack.last().unwrap();

                    rows.push(Row::new(
                        *index_stack.last().unwrap(),
                        parent,
                        key,
                        s,
                        RowType::Obj,
                    ));

                    *index_stack.last_mut().unwrap() += 1;
                    state = State::END;
                }
            }

            Number(n) =>
            {
                let key = if matches!(container_stack.last(), Some(Obj))
                {
                    std::mem::take(&mut current_key)
                }
                else
                {
                    index_stack.last().unwrap().to_string()
                };

                let parent = *parent_stack.last().unwrap();

                rows.push(Row::new(
                    *index_stack.last().unwrap(),
                    parent,
                    key,
                    n.to_string(),
                    RowType::Obj,
                ));

                *index_stack.last_mut().unwrap() += 1;
                state = State::END;
            }

            True | False =>
            {
                let key = if matches!(container_stack.last(), Some(Obj))
                {
                    std::mem::take(&mut current_key)
                }
                else
                {
                    index_stack.last().unwrap().to_string()
                };

                let parent = *parent_stack.last().unwrap();

                rows.push(Row::new(
                    *index_stack.last().unwrap(),
                    parent,
                    key,
                    matches!(tok, Tok::True),
                    RowType::Obj,
                ));

                *index_stack.last_mut().unwrap() += 1;
                state = State::END;
            }

            Null =>
            {
                let key = if matches!(container_stack.last(), Some(Obj))
                {
                    std::mem::take(&mut current_key)
                }
                else
                {
                    index_stack.last().unwrap().to_string()
                };

                let parent = *parent_stack.last().unwrap();

                rows.push(Row::new(
                    *index_stack.last().unwrap(),
                    parent,
                    key,
                    "null",
                    RowType::Obj,
                ));

                *index_stack.last_mut().unwrap() += 1;
                state = State::END;
            }

            Colon => state = State::VAL,
            Comma =>
            {
                state = if matches!(container_stack.last(), Some(Obj))
                {
                    State::KEY
                }
                else
                {
                    State::VAL
                }
            }

            _ =>
            {}
        }

        i += 1;
    }

    Ok(rows)
}

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
enum TokTy
{
    TRU = 1, FAL, NUL, TXT, ESC, HEX, NUM, NEW_ARR, END_ARR, NEW_OBJ, END_OBJ,
    COM, COL,
}

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
pub enum State
{
    BEG = 1, END, OBJ, ARR, TXT, KEY, COL, VAL
}

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
pub enum Action
{
    FIN = 1,
    KEY,
}

// Expect/Accept
// Tab/Nest record parent ID, clip/take astnode id
// [ID][PARENT][KEY][VALUE][TYPE]
struct AstRow(usize, usize, String, String, TokTy);
struct Transition(State, TokTy, State, Action);

impl From<Tok> for TokTy
{
    fn from(value: Tok) -> Self
    {
        use TokTy::*;
        match value
        {
            Tok::True => TRU,
            Tok::False => FAL,
            Tok::Null => NUL,
            Tok::Text(_) => TXT,
            Tok::Escape(_) => ESC,
            Tok::EscapeHex(_) => HEX,
            Tok::Number(_) => NUM,
            Tok::ArrayOpen => NEW_ARR,
            Tok::ArrayClose => END_ARR,
            Tok::ObjectOpen => NEW_OBJ,
            Tok::ObjectClose => END_OBJ,
            Tok::Comma => COM,
            Tok::Colon => COL,
        }
    }
}

fn main() -> PqResult<()>
{
    let value: Value;
    let stdin = io::stdin();

    let interactive_session = stdin.is_terminal();
    let filename = env::args().nth(1);

    match (interactive_session, filename)
    {
        // Interactive no filename, nothing to read from
        (true, None) =>
        {
            const BLUE: &str = "\x1b[34m";
            print!("{BLUE}> ");
            io::stdout().flush()?;
            let mut input = String::new();
            let _ = stdin.read_line(&mut input)?;
            value = serde_json::from_str(input.trim())?;
        }
        // Interactive with filename, read from file
        (true, Some(path)) =>
        {
            value = serde_json::from_str(&fs::read_to_string(path)?)?;
        }
        // Piped, no filename, read from stdin
        (false, None) =>
        {
            value = serde_json::from_reader(stdin)?;
        }
        // Piped, filename given, have to choose just one
        (false, Some(_)) =>
        {
            return Ok(println!("Usage: bat json.json | emit\nemit json.json"));
        }
    }

    let mut table = Vec::new();
    traverse(&mut table, String::new(), value.clone(), 0);
    // ! view_table(&table);
    println!("{:#?}", table);

    let tokens = tokenize(&value.to_string())?;

    tokens_to_rows(tokens);

    println!("{:?}", tokens);

    Ok(())
}
