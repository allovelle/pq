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

    for (line, row) in queue.iter().enumerate()
    {
        let indent = " ".repeat(4).repeat(level);

        if matches!(row.ty, NewObj)
        {
            println!("{}{{", indent);
        }
        else if matches!(row.ty, EndObj)
        {
            println!("{}}}", indent);
        }
        if matches!(row.ty, NewArr)
        {
            println!("{}[", indent);
        }
        else if matches!(row.ty, EndArr)
        {
            println!("{}]", indent);
        }
        else if matches!(row.ty, Null | Bool | Num)
        {
            print!("{:?}: {}", row.key, row.value);
            if line < queue.len() - 1
            {
                println!(",")
            }
        }
        else
        {
            print!("{:?}: {:?}", row.key, row.value);
            if line < queue.len() - 1
            {
                println!(",")
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

enum RowType
{
    NewArr,
    EndArr,
    NewObj,
    EndObj,
    Null,
    Bool,
    Str,
    Num,
}

struct Row
{
    ty: RowType,
    key: String,
    value: String,
    parent_id: usize,
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
            parent_id: parent,
        }),
        Value::Bool(tf) => queue.push(Row {
            ty: Bool,
            key: key.to_string(),
            value: tf.to_string(),
            parent_id: parent,
        }),
        Value::Number(num) => queue.push(Row {
            ty: Num,
            key: key.to_string(),
            value: num.to_string(),
            parent_id: parent,
        }),
        Value::String(txt) => queue.push(Row {
            ty: Str,
            key: key.to_string(),
            value: txt,
            parent_id: parent,
        }),
        Value::Array(arr) =>
        {
            queue.push(Row {
                ty: NewArr,
                key: key.clone(),
                value: String::new(),
                parent_id: parent,
            });
            for element in arr
            {
                traverse(queue, &key, element, parent + 1);
            }
            queue.push(Row {
                ty: EndArr,
                key: key.clone(),
                value: String::new(),
                parent_id: parent,
            });
        }
        Value::Object(map) =>
        {
            queue.push(Row {
                ty: NewObj,
                key: key.clone(),
                value: String::new(),
                parent_id: parent,
            });
            for (name, element) in map
            {
                traverse(queue, name, element, parent + 1);
            }
            queue.push(Row {
                ty: EndObj,
                key: key.clone(),
                value: String::new(),
                parent_id: parent,
            });
        }
    }
}
