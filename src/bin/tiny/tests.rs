// use super::*;
// use crate::formatter::FormatConfig;
// use crate::formatter::format_table;
// use crate::lexer::*;
// use crate::parser::*;

/// Tests [json_tokens_from_str], [classify_token], and [token_value]
/// together to verify individual tokens are in the right order and
/// correctly classified while also proving actual token values.
/// Does not invoke the parser so invalid source is allowed.
#[test]
fn token_index_and_order()
{
    use crate::lexer::{
        TokVal, TokenKind, classify_token, json_tokens_from_str, token_value,
    };

    let code = r#"[1, 2, [3, 4], 5, 6]"#;
    for token in json_tokens_from_str(code)
    {
        match token
        {
            Ok(at) =>
            {
                let ty = classify_token(code, at);
                let val = token_value(code, at);
                println!("{:<3} {:<15} {:?}", at, format!("{ty:?}"), val);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    let expected = [
        (0, TokenKind::LBracket, TokVal::NewArr),
        (1, TokenKind::Number, TokVal::Num("1")),
        (2, TokenKind::Comma, TokVal::Com),
        (4, TokenKind::Number, TokVal::Num("2")),
        (5, TokenKind::Comma, TokVal::Com),
        (7, TokenKind::LBracket, TokVal::NewArr),
        (8, TokenKind::Number, TokVal::Num("3")),
        (9, TokenKind::Comma, TokVal::Com),
        (11, TokenKind::Number, TokVal::Num("4")),
        (12, TokenKind::RBracket, TokVal::EndArr),
        (13, TokenKind::Comma, TokVal::Com),
        (15, TokenKind::Number, TokVal::Num("5")),
        (16, TokenKind::Comma, TokVal::Com),
        (18, TokenKind::Number, TokVal::Num("6")),
        (19, TokenKind::RBracket, TokVal::EndArr),
    ];
    let token_stream =
        json_tokens_from_str(code).filter_map(|t| t.ok()).map(|at| {
            let ty = classify_token(code, at);
            let val = token_value(code, at);
            (at, ty, val)
        });
    for (resulted, expected) in token_stream.zip(expected.iter())
    {
        assert_eq!(resulted, *expected);
    }
}

/// This test contains invalid JSON but only tests tokenization which allows
/// scrambled tokens. The parser must enforce structure.
/// Does not invoke the parser so invalid source is allowed.
#[test]
fn valid_tokens_from_invalid_source()
{
    use crate::lexer::{classify_token, json_tokens_from_str, token_value};

    let code = r#"[1, 2, [3, 4], 5, 6"#;
    let tokens: Vec<usize> =
        json_tokens_from_str(code).map(|row| row.unwrap()).collect();
    let msg = "Missing end arr is an error for the parser, not the lexer";
    assert_eq!(tokens.len(), 14, "{}", msg);
}

/// Verifies that structured values correctly return their direct subnodes.
/// Invokes the parser so invalid source should be rejected.
#[test]
fn subnodes()
{
    use crate::parser::{RowType, parse_from_str};

    let code = r#"[1, 2, [3, 4], 5, 6]"#;
    let table = parse_from_str(code).unwrap();
    assert_eq!(table[0].ty, RowType::Arr);
    assert_eq!(table[3].ty, RowType::Arr);
    let subnodes = table[3].subnodes(&table).collect::<Vec<_>>();
    assert_eq!(subnodes.len(), 2);
    assert_eq!(subnodes[0].val, "3");
    assert_eq!(subnodes[1].val, "4");
}

#[test]
fn parser_json_structure()
{
    use crate::parser::{RowType, parse_from_str};

    let code = r#"[1, 2, [3, 4], 5, 6]"#;
    let table = parse_from_str(code).unwrap();

    assert_eq!(table.len(), 8);
    assert_eq!(table[0].val, "[");
    assert_eq!(table[1].val, "1");
    assert_eq!(table[2].val, "2");
    assert_eq!(table[3].ty, RowType::Arr);
    assert_eq!(table[3].key, "4"); // Array indices have numeric keys
    assert_eq!(table[3].val, "[");
    assert_eq!(table[4].val, "3");
    assert_eq!(table[5].val, "4");
    assert_eq!(table[6].val, "5");
    assert_eq!(table[7].val, "6");

    let code = r#"[1, 2, [3, 4], 5, 6"#;
    assert!(parse_from_str(code).is_err());

    // let code = r#"{ "k": "v", "num": 123, "bit": true, "nil": null, "arr": [1, 2, 3], "obj": { "a": "b", "c": "d" }"#;
    let code = r#"{
            "k": "v", "num": 123, "bit": true, "nil": null,
            "arr": [1, 2, 3],
            "obj": { "a": "b", "c": ["d"] }
        }"#;
    let table = parse_from_str(code).unwrap();

    assert_eq!(table.len(), 13);
    assert_eq!(
        (table[0].ty, table[0].key.as_str(), table[0].val.as_str()),
        (RowType::Obj, "", "{")
    );
    assert_eq!(
        (table[1].ty, table[1].key.as_str(), table[1].val.as_str()),
        (RowType::Str, "k", "v")
    );
    assert_eq!(
        (table[2].ty, table[2].key.as_str(), table[2].val.as_str()),
        (RowType::Num, "num", "123")
    );
    assert_eq!(
        (table[3].ty, table[3].key.as_str(), table[3].val.as_str()),
        (RowType::Bit, "bit", "true")
    );
    assert_eq!(
        (table[4].ty, table[4].key.as_str(), table[4].val.as_str()),
        (RowType::Nil, "nil", "null")
    );
    assert_eq!(
        (table[5].ty, table[5].key.as_str(), table[5].val.as_str()),
        (RowType::Arr, "arr", "[")
    );
    assert_eq!(
        (table[6].ty, table[6].key.as_str(), table[6].val.as_str()),
        (RowType::Num, "0", "1")
    );
    assert_eq!(
        (table[7].ty, table[7].key.as_str(), table[7].val.as_str()),
        (RowType::Num, "2", "2")
    );
    assert_eq!(
        (table[8].ty, table[8].key.as_str(), table[8].val.as_str()),
        (RowType::Num, "4", "3")
    );
    assert_eq!(
        (table[9].ty, table[9].key.as_str(), table[9].val.as_str()),
        (RowType::Obj, "obj", "{")
    );
    assert_eq!(
        (table[10].ty, table[10].key.as_str(), table[10].val.as_str()),
        (RowType::Str, "a", "b")
    );
    assert_eq!(
        (table[11].ty, table[11].key.as_str(), table[11].val.as_str()),
        (RowType::Arr, "c", "[")
    );
    assert_eq!(
        (table[12].ty, table[12].key.as_str(), table[12].val.as_str()),
        (RowType::Str, "0", "d")
    );

    // Row { id: 0, par: 0, key: "", val: "{", ty: Obj }
    // Row { id: 1, par: 0, key: "k", val: "v", ty: Str }
    // Row { id: 2, par: 0, key: "num", val: "123", ty: Num }
    // Row { id: 3, par: 0, key: "bit", val: "true", ty: Bit }
    // Row { id: 4, par: 0, key: "nil", val: "null", ty: Null }
    // Row { id: 5, par: 0, key: "arr", val: "[", ty: Arr }
    // Row { id: 6, par: 5, key: "0", val: "1", ty: Num }
    // Row { id: 7, par: 5, key: "2", val: "2", ty: Num }
    // Row { id: 8, par: 5, key: "4", val: "3", ty: Num }
    // Row { id: 9, par: 0, key: "obj", val: "{", ty: Obj }
    // Row { id: 10, par: 9, key: "a", val: "b", ty: Str }
    // Row { id: 11, par: 9, key: "c", val: "[", ty: Arr }
    // Row { id: 12, par: 11, key: "0", val: "d", ty: Str }
    for row in &table
    {
        println!("{:?}", row);
    }
}

#[test]
fn format_simple_array()
{
    use crate::formatter::{FormatConfig, format_table};
    use crate::parser::parse_from_str;

    let json = r#"[1, 2, 3]"#;
    let table = parse_from_str(json).unwrap();
    let config = FormatConfig::new().with_colors(false);
    let lines = format_table(&table, &config);

    // Should produce formatted output
    assert!(!lines.is_empty());
    println!("Formatted array:");
    for line in &lines
    {
        println!("{}", line);
    }
}

#[test]
fn format_nested_object()
{
    use crate::formatter::{FormatConfig, format_table};
    use crate::parser::parse_from_str;

    let json = r#"{"name": "test", "nested": {"key": "value"}}"#;
    let table = parse_from_str(json).unwrap();
    let config = FormatConfig::new().with_colors(false);
    let lines = format_table(&table, &config);

    assert!(!lines.is_empty());
    println!("Formatted object:");
    for line in &lines
    {
        println!("{}", line);
    }
}

#[test]
fn format_complex()
{
    use crate::formatter::{FormatConfig, format_table};
    use crate::parser::parse_from_str;

    let json = r#"[1, 2, [3, 4], 5, 6]"#;
    let table = parse_from_str(json).unwrap();
    let config = FormatConfig::new().with_colors(false);
    let lines = format_table(&table, &config);

    assert!(!lines.is_empty());
    println!("Formatted complex:");
    for line in &lines
    {
        println!("{}", line);
    }
}

#[test]
fn format_with_colors()
{
    use crate::formatter::{FormatConfig, format_table};
    use crate::parser::parse_from_str;

    let json = r#"{"key": "value", "num": 42}"#;
    let table = parse_from_str(json).unwrap();
    let config = FormatConfig::new().with_colors(true);
    let lines = format_table(&table, &config);

    println!("Formatted with colors:");
    for line in &lines
    {
        println!("{}", line);
    }
}

#[test]
fn table_api()
{
    use crate::table::{JsonTable, Row, RowType};

    let mut table = JsonTable::new(vec![
        Row::new(0, 0, 0, "", "{", RowType::Obj),
        Row::new(1, 0, 0, "a1", "a2", RowType::Str),
        Row::new(2, 0, 0, "a3", "818", RowType::Num),
    ]);

    let first = table.first_child(0).unwrap();
    assert_eq!(first, 1);

    let next = table.next_sibling(first).unwrap();
    assert_eq!(next, 2);

    let parent = table.parent(first).unwrap();
    assert_eq!(parent, 0);

    let parent = table.parent(next).unwrap();
    assert_eq!(parent, 0);
}

#[test]
fn tree_api()
{
    use crate::parser::RowType;
    use crate::tree::{Row, RowTree};

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Val
    {
        key: &'static str,
        val: &'static str,
        ty: RowType,
    }

    impl Val
    {
        pub fn new(key: &'static str, val: &'static str, ty: RowType) -> Self
        {
            Self { key, val, ty }
        }
    }

    let mut tree: RowTree<Val> = RowTree::new(vec![
        Row::new(0, 0, Val::new("", "{", RowType::Obj)),
        Row::new(1, 0, Val::new("a1", "a2", RowType::Str)),
        Row::new(2, 0, Val::new("a3", "818", RowType::Num)),
    ]);
}
