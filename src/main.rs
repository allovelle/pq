/*
1. The types keep pointing back to the previous stages.

2. The stage types like Parser must be all public so helper functions can have a
    flexible API.

3. Do not start at the beginning or the end: start without IO focus.
*/

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
struct Tok;
struct Row;
struct Parser;
struct Querier;
struct Formatter;

impl Formatter
{
    // ! This is going to be a lifetime deathtrap for back-referencing mutables:
    // ! ---EVERY stage needs to consume and emit a piecemeal iterator.
    // ? Perhaps the only way thru is to keep append-only buffers in each stage
    // ? allowing random access for all stored items and also an iterator-like
    // ? interface yielding indexes:
    // * Utf8Buf => Lexer => Parser => Querier => Formatter
    // TODO: Proved: Random-Access, Concurrent-Modification, and Iteration work
    fn format(query_stream: &[Row]) -> impl Iterator<Item = String>
    {
        struct Attributer;
        let mut attributer = Attributer::new();
        let mut row_attrs = attributer.annotate(&mut query_stream);
    }
}

// ? Can all of these reference each other if they were stored in a global pipe?
fn main()
{
    // ? Where does the source come from
    let source = "818";
    let mut source_stream = source.chars();

    let mut lexer = Lexer;
    let mut token_stream = lexer.tokenize(&mut source_stream);

    let mut parser = Parser;
    let mut table_stream = parser.parse(&mut token_stream);

    // ? Where do the queries come from
    let mut querier = Querier;
    let mut query_stream = querier.query(&mut table_stream);

    let mut formatter = Formatter;
    let mut line_stream = formatter.format(&mut query_stream);

    for line in line_stream
    {
        println!("{line}");
    }
}
