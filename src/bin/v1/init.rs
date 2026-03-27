enum IoStrategy
{
    Retain,
    Release,
}

enum Io
{
    Input(IoStrategy),
    Output(IoStrategy),
}

enum A0
{
    Stream,
    Buffer,
}

struct Utf8Buffer
{
    buffer: Vec<u8>,
    inputs: Io,
    ouputs: Io,
}

struct Lexer<'lex>
{
    utf8: &'lex Utf8Buffer,
}

// TODO: iteration api
// TODO: full-buffer collect api
// TODO: request-response api
// TODO: incremental api

/*
STREAM OR BUFFER REFERENCE & HANDLE

Any system can be single-threaded by just slurping all inputs into a buffer and
    storing all outputs in a buffer and yielding indexes to them

UTF8BUF
    in: raw byte stream
    ou: char stream as codepoint bytes become whole
    inputs: retain (BUFFER)
    outputs: release (STREAM)
    apis:
        iterator one item by one (initial pass over inputs)
        byte index (must be exact codepoint byte boundary)

LEXER
    "Does not retain any state, is a pure function"
    in: char stream
    ou: token indices into UTF8BUF, but only if UTF8BUF retains outputs
    inputs: release (STREAM)
    outputs: release (STREAM)
    apis:
        iterator 1 item by 1 (can dereference/retokenize indices later)
        piecemeal token index into original UTF8BUF source

PARSER:
    "Maintains giant doc of valid and expired jsonl documents. Particularly as
        the output from query stages"
    in: token stream
    ou: json row tree forming json lines document
    inputs: release (STREAM)
    outputs: retain (BUFFER)
        buffered as they are streamed, can index any prior element
    apis:
        random access index
        stream rows to query engine

QUERY:
    in: table of jsonl rows OR previous query outputs
    ou: table of jsonl rows
    inputs: retain (BUFFER)
    outputs: retain (BUFFER)

    request-response specific item random access by reference handle
    OR can stream them 1 by 1 cause the LEXER can retokenize them

FORMATTER:
    in: table of jsonl rows
    ou: formatted lines of text to stdout
    inputs: release (STREAM)
    outputs: release (STREAM)
    apis:
        invariant: table reference must point to the row and all predecessors
        iterator 1 item by 1
        random access format

The utf8buf needs to provide access to the buffer during iteration for
    piecemeal processing

Outputs can be: STREAMED OR a buffer of handles can be returned HANDLED BUFFERED
    this means either an parial buffer or full buffer sent
*/

use std::ops::Range;

//
// ─── CORE ACCESS ABSTRACTION ────────────────────────────────────────────────
//

pub enum Access<'a, T>
{
    Stream(Box<dyn Iterator<Item = T> + 'a>),
    Buffer(&'a [T]),
}

//
// ─── SOURCE ─────────────────────────────────────────────────────────────────
//

pub trait ByteSource
{
    fn next_chunk(&mut self) -> Option<&[u8]>;
}

//
// ─── UTF8 STAGE ─────────────────────────────────────────────────────────────
//

#[derive(Clone, Debug)]
pub struct Codepoint
{
    pub ch: char,
    pub byte_range: Range<usize>,
}

pub struct Utf8Stream<S: ByteSource>
{
    pub source: S,
}

impl<S: ByteSource> Utf8Stream<S>
{
    pub fn into_stream(self) -> Access<'static, Codepoint>
    {
        // stub iterator
        Access::Stream(Box::new(std::iter::empty()))
    }
}

//
// ─── LEXER ──────────────────────────────────────────────────────────────────
//

#[derive(Clone, Debug)]
pub enum TokenKind
{
    // stub
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Token
{
    pub kind: TokenKind,
    pub span: Range<usize>,
}

impl<'lex> Lexer<'lex>
{
    pub fn run(&self, input: Access<'lex, Codepoint>) -> Access<'lex, Token>
    {
        match input
        {
            Access::Stream(_it) =>
            {
                // stub
                Access::Stream(Box::new(std::iter::empty()))
            }
            Access::Buffer(_buf) =>
            {
                // optional path
                Access::Stream(Box::new(std::iter::empty()))
            }
        }
    }
}

//
// ─── PARSER ─────────────────────────────────────────────────────────────────
//

#[derive(Clone, Debug)]
pub enum JsonValue
{
    // stub
    Null,
}

#[derive(Clone, Debug)]
pub struct JsonRow
{
    pub root: JsonValue,
    pub span: Range<usize>,
}

pub struct JsonTable
{
    pub rows: Vec<JsonRow>,
}

pub struct Parser;

impl Parser
{
    pub fn run<'a>(&self, input: Access<'a, Token>) -> JsonTable
    {
        match input
        {
            Access::Stream(_it) => JsonTable { rows: vec![] },
            Access::Buffer(_buf) => JsonTable { rows: vec![] },
        }
    }
}

//
// ─── QUERY ──────────────────────────────────────────────────────────────────
//

pub trait Query
{
    fn eval(&self, input: &JsonTable) -> JsonTable;
}

// example no-op query
pub struct IdentityQuery;

impl Query for IdentityQuery
{
    fn eval(&self, input: &JsonTable) -> JsonTable
    {
        JsonTable { rows: input.rows.clone() }
    }
}

//
// ─── FORMATTER ──────────────────────────────────────────────────────────────
//

pub trait Formatter
{
    fn format<'a>(&self, rows: Access<'a, &'a JsonRow>) -> Access<'a, String>;
}

// example formatter
pub struct SimpleFormatter;

impl Formatter for SimpleFormatter
{
    fn format<'a>(&self, _rows: Access<'a, &'a JsonRow>) -> Access<'a, String>
    {
        Access::Stream(Box::new(std::iter::empty()))
    }
}

//
// ─── PIPELINE GLUE ──────────────────────────────────────────────────────────
//

pub struct Pipeline<'pipe, Q: Query, F: Formatter>
{
    pub lexer: Lexer<'pipe>,
    pub parser: Parser,
    pub query: Q,
    pub formatter: F,
}

impl<'pipe, Q: Query, F: Formatter> Pipeline<'pipe, Q, F>
{
    pub fn run<S: ByteSource>(self, source: S) -> Access<'static, String>
    {
        // UTF8
        let utf8 = Utf8Stream { source };
        let codepoints = utf8.into_stream();

        // LEX
        let tokens = self.lexer.run(codepoints);

        // PARSE
        let table = self.parser.run(tokens);

        // QUERY
        let result = self.query.eval(&table);

        // FORMAT
        let row_iter = result.rows.iter();
        let access = Access::Stream(Box::new(row_iter));

        self.formatter.format(access)
    }
}

fn main()
{
    struct DummySource;

    impl ByteSource for DummySource
    {
        fn next_chunk(&mut self) -> Option<&[u8]> { None }
    }

    fn main()
    {
        let pipeline = Pipeline {
            lexer: Lexer,
            parser: Parser,
            query: IdentityQuery,
            formatter: SimpleFormatter,
        };

        let _output = pipeline.run(DummySource);
    }
}
