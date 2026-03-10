use thiserror::Error;

// ---------------------------------------------------------------------------
// Lex-level errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum LexError
{
    #[error("invalid UTF-8 at byte offset {offset}")]
    InvalidUtf8
    {
        offset: usize
    },

    #[error(
        "unexpected end of input while lexing {context} at offset {offset}"
    )]
    UnexpectedEof
    {
        offset: usize, context: &'static str
    },

    #[error("invalid JSON literal at offset {offset}: got {got:?}")]
    InvalidLiteral
    {
        offset: usize, got: String
    },

    #[error("unterminated string starting at offset {offset}")]
    UnterminatedString
    {
        offset: usize
    },

    #[error("invalid escape sequence '\\{ch}' at offset {offset}")]
    InvalidEscape
    {
        offset: usize, ch: char
    },

    #[error("invalid number at offset {offset}")]
    InvalidNumber
    {
        offset: usize
    },
}

// ---------------------------------------------------------------------------
// Parse-level errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ParseError
{
    #[error("lex error: {0}")]
    Lex(#[from] LexError),

    #[error("unexpected token {got:?} at offset {offset}, expected {expected}")]
    UnexpectedToken
    {
        offset: usize, got: String, expected: &'static str
    },

    #[error("trailing input after root value at offset {offset}")]
    TrailingInput
    {
        offset: usize
    },

    #[error(
        "mismatched {bracket:?}: opened at {open_offset}, got closing at {close_offset}"
    )]
    MismatchedBracket
    {
        bracket: char,
        open_offset: usize,
        close_offset: usize,
    },
}

// ---------------------------------------------------------------------------
// Top-level application error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AppError
{
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("lex error: {0}")]
    Lex(#[from] LexError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("terminal error: {0}")]
    Terminal(String),
}
