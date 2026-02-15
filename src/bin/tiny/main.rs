mod codepoints;
mod formatter;
mod lexer;
mod parser;
mod query;
#[cfg(test)]
mod tests;

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
    let rows = parser::parse_from_str(code).unwrap();
    for r in rows.iter()
    {
        println!("{:?}", r);
    }

    print_formatted(&rows, &FormatConfig::new());
}
