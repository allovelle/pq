/// lex.rs — Streaming JSON tokenizer.
///
/// Three types:
///
///   `Tok(u32)`  — byte-offset index.  The tokenizer yields only these.
///
///   `TokTy`     — kind without payload, O(1) from the leading byte.
///
///   `TokVal`    — fully decoded value; requires a `&[u8]` for re-scan.
///                 Callers pass `utf8_buf.as_bytes()` — always safe because
///                 tokens are guaranteed to reference already-consumed bytes.
///
/// `Lex` has the same accumulate / streaming duality as `Utf8Buf`:
///
///   - **Accumulating**: stores every `Tok` it emits internally.
///   - **Streaming**: forwards tokens without storing them.

// ── Tok ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tok(pub u32);

impl Tok
{
    #[inline]
    pub fn offset(self) -> usize
    {
        self.0 as usize
    }
}

// ── TokTy ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokTy
{
    Str,
    Number,
    Bool,
    Null,
    ObjOpen,
    ObjClose,
    ArrOpen,
    ArrClose,
    Colon,
    Comma,
    Unknown,
}

impl TokTy
{
    pub fn from_byte(b: u8) -> Self
    {
        match b
        {
            b'"' => TokTy::Str,
            b'-' | b'0' ..= b'9' => TokTy::Number,
            b't' | b'f' => TokTy::Bool,
            b'n' => TokTy::Null,
            b'{' => TokTy::ObjOpen,
            b'}' => TokTy::ObjClose,
            b'[' => TokTy::ArrOpen,
            b']' => TokTy::ArrClose,
            b':' => TokTy::Colon,
            b',' => TokTy::Comma,
            _ => TokTy::Unknown,
        }
    }

    pub fn label(self) -> &'static str
    {
        match self
        {
            TokTy::Str => "string",
            TokTy::Number => "number",
            TokTy::Bool => "bool",
            TokTy::Null => "null",
            TokTy::ObjOpen => "{",
            TokTy::ObjClose => "}",
            TokTy::ArrOpen => "[",
            TokTy::ArrClose => "]",
            TokTy::Colon => ":",
            TokTy::Comma => ",",
            TokTy::Unknown => "?",
        }
    }
}

// ── TokVal ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokVal
{
    Str(String),
    Number(f64),
    Bool(bool),
    Null,
    ObjOpen,
    ObjClose,
    ArrOpen,
    ArrClose,
    Colon,
    Comma,
}

impl TokVal
{
    pub fn display(&self) -> String
    {
        match self
        {
            TokVal::Str(s) => format!("Str({s:?})"),
            TokVal::Number(n) => format!("Number({n})"),
            TokVal::Bool(b) => format!("Bool({b})"),
            TokVal::Null => "Null".into(),
            TokVal::ObjOpen => "ObjOpen  {".into(),
            TokVal::ObjClose => "ObjClose }".into(),
            TokVal::ArrOpen => "ArrOpen  [".into(),
            TokVal::ArrClose => "ArrClose ]".into(),
            TokVal::Colon => "Colon    :".into(),
            TokVal::Comma => "Comma    ,".into(),
        }
    }
}

// ── deref helpers — callers supply the Utf8Buf's byte slice ──────────────────

/// Decode the full value of `tok` from `src`.
/// `src` is `utf8_buf.as_bytes()` — always valid because tokens reference
/// already-consumed bytes.
pub fn tok_val(src: &[u8], tok: Tok) -> TokVal
{
    let off = tok.offset();
    match src.get(off).copied().unwrap_or(0)
    {
        b'{' => TokVal::ObjOpen,
        b'}' => TokVal::ObjClose,
        b'[' => TokVal::ArrOpen,
        b']' => TokVal::ArrClose,
        b':' => TokVal::Colon,
        b',' => TokVal::Comma,
        b'"' => TokVal::Str(scan_string(src, off)),
        b't' => TokVal::Bool(true),
        b'f' => TokVal::Bool(false),
        b'n' => TokVal::Null,
        _ => TokVal::Number(scan_number(src, off)),
    }
}

/// Derive the kind of `tok` from its leading byte — O(1).
pub fn tok_ty(src: &[u8], tok: Tok) -> TokTy
{
    TokTy::from_byte(src.get(tok.offset()).copied().unwrap_or(0))
}

/// Return `(TokTy, TokVal)` in one pass.
pub fn tok_ty_val(src: &[u8], tok: Tok) -> (TokTy, TokVal)
{
    let val = tok_val(src, tok);
    let ty = match &val
    {
        TokVal::Str(_) => TokTy::Str,
        TokVal::Number(_) => TokTy::Number,
        TokVal::Bool(_) => TokTy::Bool,
        TokVal::Null => TokTy::Null,
        TokVal::ObjOpen => TokTy::ObjOpen,
        TokVal::ObjClose => TokTy::ObjClose,
        TokVal::ArrOpen => TokTy::ArrOpen,
        TokVal::ArrClose => TokTy::ArrClose,
        TokVal::Colon => TokTy::Colon,
        TokVal::Comma => TokTy::Comma,
    };
    (ty, val)
}

fn scan_string(src: &[u8], off: usize) -> String
{
    let mut buf = String::new();
    let mut i = off + 1;
    let mut esc = false;
    while i < src.len()
    {
        let b = src[i];
        i += 1;
        if esc
        {
            buf.push(b as char);
            esc = false;
        }
        else if b == b'\\'
        {
            esc = true;
        }
        else if b == b'"'
        {
            break;
        }
        else
        {
            buf.push(b as char);
        }
    }
    buf
}

fn scan_number(src: &[u8], off: usize) -> f64
{
    let mut i = off;
    if i < src.len() && src[i] == b'-'
    {
        i += 1;
    }
    while i < src.len() && src[i].is_ascii_digit()
    {
        i += 1;
    }
    if i < src.len() && src[i] == b'.'
    {
        i += 1;
        while i < src.len() && src[i].is_ascii_digit()
        {
            i += 1;
        }
    }
    if i < src.len() && (src[i] == b'e' || src[i] == b'E')
    {
        i += 1;
        if i < src.len() && (src[i] == b'+' || src[i] == b'-')
        {
            i += 1;
        }
        while i < src.len() && src[i].is_ascii_digit()
        {
            i += 1;
        }
    }
    std::str::from_utf8(&src[off .. i])
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(f64::NAN)
}

// ── Lex ───────────────────────────────────────────────────────────────────────
// Accumulate / streaming duality mirrors Utf8Buf.

pub struct Lex
{
    buf: Vec<Tok>,
    accumulate: bool,
}

impl Lex
{
    pub fn new() -> Self
    {
        Self { buf: Vec::new(), accumulate: true }
    }

    pub fn streaming() -> Self
    {
        Self { buf: Vec::new(), accumulate: false }
    }

    /// All tokens accumulated so far (empty in streaming mode).
    pub fn tokens(&self) -> &[Tok]
    {
        &self.buf
    }

    /// Record a token (called by `Tokenizer` when it emits one).
    fn push(&mut self, tok: Tok)
    {
        if self.accumulate
        {
            self.buf.push(tok);
        }
    }
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum State
{
    Idle,
    InString
    {
        start: u32,
        escaped: bool,
    },
    InNumber
    {
        start: u32,
    },
    InKeyword
    {
        start: u32,
        len: u8,
    },
}

/// Wraps a `(usize, char)` iterator and yields `Tok` values, optionally
/// accumulating them into a `Lex` buffer.
pub struct Tokenizer<'lex, I: Iterator<Item = (usize, char)>>
{
    chars: I,
    lex: &'lex mut Lex,
    state: State,
    pending: Option<Tok>,
    done: bool,
}

impl<'lex, I: Iterator<Item = (usize, char)>> Tokenizer<'lex, I>
{
    pub fn new(chars: I, lex: &'lex mut Lex) -> Self
    {
        Self { chars, lex, state: State::Idle, pending: None, done: false }
    }

    fn emit(&mut self, tok: Tok) -> Tok
    {
        self.lex.push(tok);
        tok
    }

    fn flush(&mut self) -> Option<Tok>
    {
        match self.state
        {
            State::InNumber { start } | State::InKeyword { start, .. } =>
            {
                self.state = State::Idle;
                Some(self.emit(Tok(start)))
            }
            _ => None,
        }
    }

    fn step(
        &mut self,
        offset: usize,
        ch: char,
        reinject: &mut Option<(usize, char)>,
    ) -> Option<Tok>
    {
        let off = offset as u32;
        match &mut self.state
        {
            State::Idle => match ch
            {
                ' ' | '\t' | '\r' | '\n' => None,
                '{' => Some(self.emit(Tok(off))),
                '}' => Some(self.emit(Tok(off))),
                '[' => Some(self.emit(Tok(off))),
                ']' => Some(self.emit(Tok(off))),
                ':' => Some(self.emit(Tok(off))),
                ',' => Some(self.emit(Tok(off))),
                '"' =>
                {
                    self.state = State::InString { start: off, escaped: false };
                    None
                }
                '-' | '0' ..= '9' =>
                {
                    self.state = State::InNumber { start: off };
                    None
                }
                't' | 'f' | 'n' =>
                {
                    self.state = State::InKeyword { start: off, len: 1 };
                    None
                }
                _ => None,
            },
            State::InString { start, escaped } =>
            {
                if *escaped
                {
                    *escaped = false;
                }
                else if ch == '\\'
                {
                    *escaped = true;
                }
                else if ch == '"'
                {
                    let start = *start;
                    self.state = State::Idle;
                    return Some(self.emit(Tok(start)));
                }
                None
            }
            State::InNumber { .. } => match ch
            {
                '0' ..= '9' | '.' | 'e' | 'E' | '+' | '-' => None,
                _ =>
                {
                    *reinject = Some((offset, ch));
                    self.flush()
                }
            },
            State::InKeyword { len, .. } =>
            {
                *len += 1;
                if *len >= 4 { self.flush() } else { None }
            }
        }
    }
}

impl<'lex, I: Iterator<Item = (usize, char)>> Iterator for Tokenizer<'lex, I>
{
    type Item = Tok;

    fn next(&mut self) -> Option<Tok>
    {
        if let Some(t) = self.pending.take()
        {
            return Some(t);
        }
        if self.done
        {
            return None;
        }

        let mut reinjected: Option<(usize, char)> = None;
        loop
        {
            let (offset, ch) = if let Some(p) = reinjected.take()
            {
                p
            }
            else
            {
                match self.chars.next()
                {
                    Some(p) => p,
                    None =>
                    {
                        self.done = true;
                        return self.flush();
                    }
                }
            };

            let mut reinject = None;
            if let Some(tok) = self.step(offset, ch, &mut reinject)
            {
                if let Some(ri) = reinject
                {
                    let mut ri2 = None;
                    self.pending = self.step(ri.0, ri.1, &mut ri2);
                }
                return Some(tok);
            }
            reinjected = reinject;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::utf8::{Utf8Iter, buf_from_slice};

    fn run(src: &[u8]) -> (Vec<Tok>, Vec<TokVal>)
    {
        let mut buf = buf_from_slice(src);
        let mut lex = Lex::new();
        let toks: Vec<Tok> =
            Tokenizer::new(Utf8Iter::new(&mut buf), &mut lex).collect();
        let vals = toks.iter().map(|&t| tok_val(buf.as_bytes(), t)).collect();
        (toks, vals)
    }

    #[test]
    fn structural()
    {
        let (_, vals) = run(b"{}[],:");
        assert_eq!(vals, vec![
            TokVal::ObjOpen,
            TokVal::ObjClose,
            TokVal::ArrOpen,
            TokVal::ArrClose,
            TokVal::Comma,
            TokVal::Colon,
        ]);
    }

    #[test]
    fn string_val()
    {
        let (_, vals) = run(br#""hello""#);
        assert_eq!(vals, vec![TokVal::Str("hello".into())]);
    }

    #[test]
    fn number_val()
    {
        let (_, vals) = run(b"42");
        assert_eq!(vals, vec![TokVal::Number(42.0)]);
    }

    #[test]
    fn keywords()
    {
        let (_, vals) = run(b"true false null");
        assert_eq!(vals, vec![
            TokVal::Bool(true),
            TokVal::Bool(false),
            TokVal::Null
        ]);
    }

    #[test]
    fn object()
    {
        let (_, vals) = run(br#"{"a":1}"#);
        assert_eq!(vals, vec![
            TokVal::ObjOpen,
            TokVal::Str("a".into()),
            TokVal::Colon,
            TokVal::Number(1.0),
            TokVal::ObjClose,
        ]);
    }

    #[test]
    fn lex_accumulates()
    {
        let mut buf = buf_from_slice(br#"[1,2]"#);
        let mut lex = Lex::new();
        let _: Vec<_> =
            Tokenizer::new(Utf8Iter::new(&mut buf), &mut lex).collect();
        assert_eq!(lex.tokens().len(), 5); // [ 1 , 2 ]
    }

    #[test]
    fn lex_streaming_does_not_accumulate()
    {
        let mut buf = buf_from_slice(br#"[1,2]"#);
        let mut lex = Lex::streaming();
        let _: Vec<_> =
            Tokenizer::new(Utf8Iter::new(&mut buf), &mut lex).collect();
        assert_eq!(lex.tokens().len(), 0);
    }
}
