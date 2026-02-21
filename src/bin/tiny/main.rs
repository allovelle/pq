mod attr;
mod codepoints;
mod formatter;
mod lexer;
mod parser;
mod query;
mod table;
mod tok_str_buf;
mod token_index;
mod tree;

#[cfg(test)]
mod tests;


use formatter::{FormatConfig, print_formatted};
use lexer::*;
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

    println!("\nQuery Pipeline:");
    let pipeline = vec![Query::SelectKey("obj".to_string())];
    let last_root_doc_index =
        query::execute_pipeline(&mut table, &pipeline).unwrap();
    let final_value = &table[last_root_doc_index ..];
    println!("\nCompleted");

    print_formatted(final_value, &FormatConfig::new());

    for r in &final_value[last_root_doc_index ..]
    {
        println!("{:?}", r);
    }
}
