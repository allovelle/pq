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


    request-response specific item random access by reference handle
    OR can stream them 1 by 1 cause the LEXER can retokenize them

The utf8buf needs to provide access to the buffer during iteration for
    piecemeal processing

Outputs can be: STREAMED OR a buffer of handles can be returned HANDLED BUFFERED
    this means either an parial buffer or full buffer sent
*/
