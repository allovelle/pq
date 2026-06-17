/*
1. The types keep pointing back to the previous stages.

2. The stage types like Parser must be all public so helper functions can have a
    flexible API.

3. Do not start at the beginning or the end: start without IO focus.
*/

mod cli;
mod io;
mod mem;
mod txt;

struct State
{
    cli: Option<()>,
    stdin: Option<In>,
    files: Vec<String>,
    format_output: bool,
    color_output: bool,
}

// * StdIn, Files, mmap concatenate stream of bytes:
// ? To Utf8Buf or maintain entire buffer and send bytes directly to Lexer?
// ? Who keeps the giant buffer of all those files? Are files closed?
struct Io;
struct Cli;
struct In;
struct Tui;

// All of these just yield indexes and keep append-only buffers:
struct Lexer;
impl Lexer
{
    fn tokenize(&self, _source_stream: &mut std::str::Chars<'_>) -> Vec<Tok>
    {
        todo!()
    }
}
struct Tok;
#[derive(Debug, Clone, Copy)]
struct Row;
#[derive(Debug, Clone, Copy)]
struct Attr;
struct Parser;
impl Parser
{
    fn parse(&self, _token_stream: &mut Vec<Tok>) -> Vec<Row> { todo!() }
}
struct Querier;
impl Querier
{
    fn query(&self, _table_stream: &mut Vec<Row>) -> Vec<Row> { todo!() }
}
struct Formatter;

impl Formatter
{
    // ! This is going to be a lifetime deathtrap for back-referencing mutables:
    // ! ---EVERY stage needs to consume and emit a piecemeal iterator.
    // ? Perhaps the only way thru is to keep append-only buffers in each stage
    // ? allowing random access for all stored items and also an iterator-like
    // ? interface yielding indexes:
    // * Utf8Buf => Lexer => Parser => Querier => Formatter
    // ? Do any of the above require iter-from from valid base index?
    // TODO: Proved: Random-Access, Concurrent-Modification, and Iteration-From:
    // TODO: Utf8Buf is cursor over mmapio bytes
    // TODO: Lexer is cursor over Utf8Buf chars
    // TODO: Parser is cursor over Lexer tokens
    // TODO: Querier iterates, modifies, and indexes Parser rows
    // TODO: Formatter is cursor over Parser/Querier rows
    fn format(&self, query_stream: &[Row]) -> impl Iterator<Item = String>
    {
        struct Attributer;
        impl Attributer
        {
            fn annotate(&self, _query_stream: &[Row]) -> Vec<(Row, Vec<Attr>)>
            {
                todo!()
            }
        }
        let attributer = Attributer;
        let row_attrs = attributer.annotate(query_stream);
        row_attrs.into_iter().map(|(row, _attrs)| format!("{:?}", row))
    }
}

// ? Can all of these reference each other if they were stored in a global pipe?
fn pipeline()
{
    // ? Where does the source come from
    let source = "818";
    let mut source_stream = source.chars();

    let lexer = Lexer;
    let mut token_stream = lexer.tokenize(&mut source_stream);

    let parser = Parser;
    let mut table_stream = parser.parse(&mut token_stream);

    // ? Where do the queries come from
    let querier = Querier;
    let mut query_stream = querier.query(&mut table_stream);

    let formatter = Formatter;
    let line_stream = formatter.format(&mut query_stream);

    for line in line_stream
    {
        println!("{line}");
    }
}

// * CLI query expr & stdin & multiple files. Invoke with:
// * bat json.json | cargo run --bin pq -- k1.k2.k3 -f json.json -f json.json
fn main()
{
    use clap::Parser;
    use cli::{Cli, Mode};
    use std::io::IsTerminal;

    let cli = Cli::parse();
    let has_stdin = !std::io::stdin().is_terminal();
    let mode = cli.mode(has_stdin);

    match mode
    {
        Mode::FormatHighlight =>
        {
            eprintln!(
                "[FormatHighlight] files={:?}, stdin={}",
                cli.files, has_stdin
            );
        }

        Mode::Query(expr) =>
        {
            eprintln!(
                "[Query] expr={:?}, files={:?}, stdin={}",
                expr, cli.files, has_stdin
            );
        }

        Mode::Interactive =>
        {
            eprintln!(
                "[Interactive] files={:?}, stdin={}",
                cli.files, has_stdin
            );
        }
    }

    // TODO: Send the input files to mem mapper
    // TODO: Send the stdin to the UTF-8 buffer & iter

    let (total_len, lengths) = txt::lengths(&cli);
    eprintln!("total_len={total_len}, lengths={:?}", lengths);
}
