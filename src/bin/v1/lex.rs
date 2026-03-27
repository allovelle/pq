use std::str;

/// A [Tok] is literally these things:
/// 1. An index into the source buffer
/// 2. The start of a literal slice of text representing structural tokens (`{`,
///     `[`, `,`, or `:`).
/// 3. The start of a literal slice of text representing string keys and values
///     such as boolean, number, and null.
pub type Tok = u32;

fn tok_on(idx: Tok) -> Tok
{
    0
}

/// Vital: the next byte may not be next token start so the next token (or EOF)
/// must be found as well.
///
/// The first time this is called, it returns the token at the index provided,
/// if that index is a valid token (should be). Using the original token, and
/// the bytes read from that call, the end byte of the next token can be
/// calculated. This allows straightforward iteration by maintaining an offset.
/// ### Return tuple
/// `(offset, size)`
///
/// - **offset** — byte index of the token start
/// - **size** — number of bytes consumed by the token
fn tok_from(idx: Tok) -> Option<(Tok, Tok)>
{
    None
}

/// A [crate::lex::TokTy]
fn tok_ty(idx: Tok) {}

/// A [crate::utf8::Text] value able to be dereferenced at any time.
fn tok_view(idx: Tok) {}

fn tok_val() {}

// ! All token values are slices into the source buffer. They are all strings.
// ? Is TokVal really just TokVal(u32)?

/// Getting a token's value is only needed during querying and formatting.
#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokVal
{
    NewObj, EndObj, NewArr, EndArr,
    Col, Com, Nil,
    Bit(bool), Num(f64), Txt(Tok),
}

/// Getting a token's type is a fundamental operation.
#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokTy
{
    NewObj, EndObj,
    NewArr, EndArr,
    Col, Com,
    Nil, Bit,
    Num, Txt,
}

#[derive(Debug)]
pub enum TokError
{
    UnexpectedByte(u8),
    UnterminatedString,
    InvalidLiteral,
}

// ── public API ───────────────────────────────────────────────────────────────

/// Start at any byte offset. Skips whitespace, returns the next token index
/// and total bytes consumed (whitespace + token). Call again at idx + n.
pub fn tok_from(buf: &[u8], idx: usize) -> Result<(TokIdx, usize), TokError>
{
    let start = skip_ws(buf, idx);
    if start >= buf.len()
    {
        return Err(TokError::UnexpectedByte(0)); // Eof
    }

    let len = match buf[start]
    {
        b'{' | b'}' | b'[' | b']' | b':' | b',' => 1,
        b'"' => scan_string(buf, start)?,
        b't' => expect_lit(buf, start, b"true")?,
        b'f' => expect_lit(buf, start, b"false")?,
        b'n' => expect_lit(buf, start, b"null")?,
        b'-' | b'0' ..= b'9' => scan_number(buf, start),
        b => return Err(TokError::UnexpectedByte(b)),
    };

    Ok((start, (start - idx) + len))
}

/// Only valid on a TokIdx returned by tok_from. Single byte read for kind,
/// minimal rescan only to recover val slice.
pub fn tok_at<'a>(buf: &'a [u8], tok: TokIdx) -> Tok<'a>
{
    let kind = tok_kind(buf, tok);
    let len = tok_len(buf, tok, &kind);
    Tok { kind, val: &buf[tok .. tok + len] }
}

// ── internals ────────────────────────────────────────────────────────────────

fn tok_kind(buf: &[u8], tok: TokIdx) -> TokKind
{
    match buf[tok]
    {
        b'{' => TokKind::LBrace,
        b'}' => TokKind::RBrace,
        b'[' => TokKind::LBracket,
        b']' => TokKind::RBracket,
        b':' => TokKind::Colon,
        b',' => TokKind::Comma,
        b'"' => TokKind::String,
        b't' | b'f' => TokKind::Bool,
        b'n' => TokKind::Null,
        _ => TokKind::Number,
    }
}

fn tok_len(buf: &[u8], tok: TokIdx, kind: &TokKind) -> usize
{
    match kind
    {
        TokKind::LBrace
        | TokKind::RBrace
        | TokKind::LBracket
        | TokKind::RBracket
        | TokKind::Colon
        | TokKind::Comma => 1,
        TokKind::String => scan_string(buf, tok).unwrap_or(0),
        TokKind::Bool =>
        {
            if buf[tok] == b't'
            {
                4
            }
            else
            {
                5
            }
        }
        TokKind::Null => 4,
        TokKind::Number => scan_number(buf, tok),
    }
}

fn skip_ws(buf: &[u8], idx: usize) -> usize
{
    let mut i = idx;
    while i < buf.len() && matches!(buf[i], b' ' | b'\t' | b'\n' | b'\r')
    {
        i += 1;
    }
    i
}

fn scan_string(buf: &[u8], pos: usize) -> Result<usize, TokError>
{
    let mut i = pos + 1;
    while i < buf.len()
    {
        match buf[i]
        {
            b'\\' => i += 2,
            b'"' => return Ok(i - pos + 1),
            _ => i += 1,
        }
    }
    Err(TokError::UnterminatedString)
}

fn scan_number(buf: &[u8], pos: usize) -> usize
{
    let mut i = pos;
    if i < buf.len() && buf[i] == b'-'
    {
        i += 1;
    }
    while i < buf.len() && buf[i].is_ascii_digit()
    {
        i += 1;
    }
    if i < buf.len() && buf[i] == b'.'
    {
        i += 1;
        while i < buf.len() && buf[i].is_ascii_digit()
        {
            i += 1;
        }
    }
    if i < buf.len() && matches!(buf[i], b'e' | b'E')
    {
        i += 1;
        if i < buf.len() && matches!(buf[i], b'+' | b'-')
        {
            i += 1;
        }
        while i < buf.len() && buf[i].is_ascii_digit()
        {
            i += 1;
        }
    }
    i - pos
}

fn expect_lit(buf: &[u8], pos: usize, lit: &[u8]) -> Result<usize, TokError>
{
    if buf[pos ..].starts_with(lit)
    {
        Ok(lit.len())
    }
    else
    {
        Err(TokError::InvalidLiteral)
    }
}
