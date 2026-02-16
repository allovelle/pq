mod codepoints;
mod formatter;
mod lexer;
mod parser;
mod query;
mod table;
#[cfg(test)]
mod tests;

use std::thread::current;

use codepoints::*;
use formatter::{FormatConfig, print_formatted};
use lexer::*;
use parser::*;
use query::*;

fn main()
{
    let code = r#"{
        "k": "v", "num": 123, "bit": true, "nil": null,
        "arr": [1, 2, 3],
        "obj": { "a": "b", "c": "d" }
    }"#;

    println!("Input JSON: {}", code);
    for token in json_tokens_from_str(code)
    {
        match token
        {
            Ok(at) =>
            {
                let ty = classify_token(code, at);
                let val = token_value(code, at);
                println!("{:<3?} {:<6} {:?}", at, format!("{ty:?}"), val);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!();

    println!("Input JSON: {}", code);
    let mut table = parser::parse_from_str(code).unwrap();
    for r in table.iter()
    {
        println!("{:?}", r);
    }

    print_formatted(&table, &FormatConfig::new());

    enum Query
    {
        SelectKey(&'static str),
    }

    let mut id = 0;
    let queries = vec![Query::SelectKey("obj"), Query::SelectKey("c")];

    let mut current_value = 0u32;
    for que in queries
    {
        match que
        {
            Query::SelectKey(key) =>
            {
                if table[current_value as usize].ty != RowType::Obj
                {
                    panic!("Cannot select key from non-object value");
                }

                // Parent, sibling, next node
                for node in table[current_value as usize].subnodes(&table)
                {
                    if node.key == key
                    {
                        println!("Found key: {}", key);
                        current_value = node.id;
                        break;
                    }
                }
            }
        }
    }

    let final_value = &table[current_value as usize ..];
    for row in final_value.iter()
    {
        println!("{:?}", row);
    }

    // Whatever is leftover from the query process is the JSON to format
    print_formatted(final_value, &FormatConfig::new());
}
