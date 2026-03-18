// Only valid on a TokIdx previously returned by tok_from.
// One byte read for kind, length re-scanned only to get val.
// TODO: pub fn tok_at(buf: &[u8], tok: TokIdx) -> Tok;

// Start anywhere. Skips whitespace, returns next token + bytes consumed.
// idx + bytes_read = where to call tok_from again.
// TODO: pub fn tok_from(buf: &[u8], idx: usize) -> Option<(TokIdx, usize)>;
/*
/// iterate
let mut idx = 0;
while let Some((tok, n)) = tok_from(buf, idx) {
    idx += n;
    /// tok is now valid for tok_at
}

/// random access on previously collected tok
let t = tok_at(buf, tokens[2]);
*/

// ---------------------------------------------------------------------------

/// [`Tok`] — a byte-offset pointer into a [`utf8::Utf8Buf`].
///
/// Stores the **start byte** of a token's slice in the buffer.  The token's
/// end is determined by the lexer that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
// TODO: Without an end byte, strings from tokens are unclear. They are paired
// TODO: with a lexer that can rebuild them from the backing buffer.
// pub type Tok = u32;
pub struct Tok(pub u32);

impl Tok
{
    /// Dereference this token back into a byte offset.
    #[inline]
    pub fn offset(self) -> usize
    {
        self.0 as usize
    }
}

#[derive(Debug, PartialEq)]
pub enum TokVal
{
    NewObj,
    EndObj,
    NewArr,
    EndArr,
    Col,
    Com,
    Nil,
    Bit(bool),
    Num(f64),
    Txt(Tok),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokTy
{
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    True,
    False,
    Null,
    Number,
    String,
    Unknown,
}

pub fn classify_token(src: &str, start: usize) -> TokenKind
{
    let b = src.as_bytes()[start];
    match b
    {
        b'{' => TokenKind::LBrace,
        b'}' => TokenKind::RBrace,
        b'[' => TokenKind::LBracket,
        b']' => TokenKind::RBracket,
        b':' => TokenKind::Colon,
        b',' => TokenKind::Comma,
        b'"' => TokenKind::String,
        b'-' | b'0' ..= b'9' => TokenKind::Number,
        b't' => TokenKind::True,
        b'f' => TokenKind::False,
        b'n' => TokenKind::Null,
        _ => TokenKind::Unknown,
    }
}

pub fn token_value<'a>(src: &'a str, start: usize) -> TokVal<'a>
{
    match classify_token(src, start)
    {
        TokenKind::LBrace => TokVal::NewObj,
        TokenKind::RBrace => TokVal::EndObj,
        TokenKind::LBracket => TokVal::NewArr,
        TokenKind::RBracket => TokVal::EndArr,
        TokenKind::Colon => TokVal::Col,
        TokenKind::Comma => TokVal::Com,

        TokenKind::True => TokVal::Bit(true),
        TokenKind::False => TokVal::Bit(false),
        TokenKind::Null => TokVal::Nil,

        TokenKind::String => TokVal::Txt(slice_string(src, start)),
        TokenKind::Number => TokVal::Num(slice_number(src, start)),

        TokenKind::Unknown => TokVal::Unknown,
    }
}

pub fn slice_string(src: &str, start: usize) -> &str
{
    let s = &src[start ..];
    let bytes = s.as_bytes();

    let mut i = 1; // skip initial quote
    while i < bytes.len()
    {
        if bytes[i] == b'"'
        {
            break;
        }
        i += 1;
    }

    &s[1 .. i] // inner contents only
}

pub fn slice_number(src: &str, start: usize) -> &str
{
    let bytes = src.as_bytes();
    let mut end = start;

    while end < bytes.len()
    {
        match bytes[end]
        {
            b'0' ..= b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => end += 1,
            _ => break,
        }
    }

    &src[start .. end]
}
