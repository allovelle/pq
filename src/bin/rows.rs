//! Convert JSON to rows
//!
use serde_json::Value;

fn main() -> Result<(), std::io::Error>
{
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

    let mut queue = Vec::new();
    traverse(&mut queue, "", value, 0);
    view_table(&queue);

    Ok(())
}

fn view_table(queue: &Vec<Row>)
{
    use RowType::*;

    let mut level = 0;
    let mut prev_parent = 0;

    for (row_id, row) in queue.iter().skip(1).enumerate()
    {
        println!("{row:?}");
        let indent = " ".repeat(4).repeat(level);

        if row.parent > prev_parent
        {
            match &queue[prev_parent].ty
            {
                Arr => println!("{} | {:?}: [", row_id, row.key),
                Obj => println!("{} | {:?}: {{", row_id, row.key),
                _ => todo!(),
            }
            prev_parent = row.parent;
        }
        else
        {
            match row.ty
            {
                Null | Bool | Num =>
                {
                    print!("{} | {:?}: {}", row_id, row.key, row.value)
                }
                Str => print!("{} | {:?}: {:?}", row_id, row.key, row.value),
                _ => todo!(),
            }
        }
    }
    println!();
}

fn show(txt: String, indent: usize)
{
    let tab = "    ".repeat(indent);
    println!("{tab}{txt}");

    /*
    Value::Null => show("null".to_string(), parent),
    Value::Bool(tf) => show(format!("{tf}"), parent),
    Value::Number(num) => show(format!("{num}"), parent),
    Value::String(txt) => show(txt.to_string(), parent),
    */
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum RowType
{
    Arr,
    Obj,
    Null,
    Bool,
    Str,
    Num,
}

/// Invariant: Self::Id is the index within it's container.
#[derive(Debug, Clone)]
struct Row
{
    ty: RowType,
    key: String,
    value: String,
    parent: usize,
}

fn traverse(
    queue: &mut Vec<Row>,
    key: impl AsRef<str>,
    value: Value,
    parent: usize,
)
{
    use RowType::*;

    let key = key.as_ref().to_string();

    // Return Some/None based on value/container?
    match value
    {
        Value::Null => queue.push(Row {
            ty: Null,
            key: key.to_string(),
            value: "null".to_string(),
            parent,
        }),
        Value::Bool(tf) => queue.push(Row {
            ty: Bool,
            key: key.to_string(),
            value: tf.to_string(),
            parent,
        }),
        Value::Number(num) => queue.push(Row {
            ty: Num,
            key: key.to_string(),
            value: num.to_string(),
            parent,
        }),
        Value::String(txt) => queue.push(Row {
            ty: Str,
            key: key.to_string(),
            value: txt,
            parent,
        }),
        Value::Array(arr) =>
        {
            queue.push(Row {
                ty: Arr,
                key: key.to_string(),
                value: "".to_string(),
                parent,
            });

            for element in arr
            {
                traverse(queue, &key, element, parent + 1);
            }
        }
        Value::Object(map) =>
        {
            queue.push(Row {
                ty: Obj,
                key: key.to_string(),
                value: "".to_string(),
                parent,
            });

            for (name, element) in map
            {
                traverse(queue, name, element, parent + 1);
            }
        }
    }
}
