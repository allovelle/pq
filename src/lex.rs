/// lex.rs — Streaming JSON tokenizer.
///
/// Three types, each a different "view" of a token:
///
///   Tok(u32)   — a byte-offset index into the source.  Cheap to copy, store,
///                and pass around.  Produced by the tokenizer iterator.
///
///   TokTy      — the *kind* of a token, no payload.  Derived from the first
///                byte at the offset — O(1), no re-scan needed for most kinds.
///
///   TokVal     — the fully decoded value (owned String / f64 / bool / unit).
///                Requires re-scanning the source from the offset.
///
/// The tokenizer iterator yields `Tok` values.  Callers promote them to
/// `TokTy` or `TokVal` only when they need the extra information.

// ── Tok ───────────────────────────────────────────────────────────────────────

/// A token represented as its byte offset in the source.
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

/// Token kind — no payload.  Derived cheaply from the first source byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokTy
{
    Str,
    Number,
    Bool,
    Null,
    ObjOpen,  // {
    ObjClose, // }
    ArrOpen,  // [
    ArrClose, // ]
    Colon,    // :
    Comma,    // ,
    Unknown,
}

impl TokTy
{
    /// Derive the kind from a single leading byte — O(1).
    /// Only `Bool` and `Null` are ambiguous from one byte (`t`/`f`/`n`) but we
    /// still resolve them: `t` → Bool, `f` → Bool, `n` → Null.
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

/// Fully decoded token value — requires re-scanning the source.
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

// ── source re-scan helpers ────────────────────────────────────────────────────

/// Decode the `TokVal` for the token starting at `src[offset]`.
/// Scans only as many bytes as needed for that one token.
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

/// Decode the `TokTy` for the token starting at `src[offset]`.
/// Always O(1) — just inspects the leading byte.
pub fn tok_ty(src: &[u8], tok: Tok) -> TokTy
{
    TokTy::from_byte(src.get(tok.offset()).copied().unwrap_or(0))
}

/// Return both the type and value in one pass.
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
    // off points at the opening `"`.
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

// ── Tokenizer (iterator) ──────────────────────────────────────────────────────

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
    }, // len of keyword chars seen so far
}

/// Streaming tokenizer.  Wraps a `(usize, char)` iterator and yields `Tok`
/// values (byte-offset indices).  Promote to `TokTy`/`TokVal` via the free
/// functions above, passing the original source slice.
pub struct Tokenizer<I: Iterator<Item = (usize, char)>>
{
    chars: I,
    state: State,
    pending: Option<Tok>,
    done: bool,
}

impl<I: Iterator<Item = (usize, char)>> Tokenizer<I>
{
    pub fn new(chars: I) -> Self
    {
        Self { chars, state: State::Idle, pending: None, done: false }
    }

    fn flush(&mut self) -> Option<Tok>
    {
        match self.state
        {
            State::InNumber { start } | State::InKeyword { start, .. } =>
            {
                self.state = State::Idle;
                Some(Tok(start))
            }
            _ => None,
        }
    }

    /// Process one codepoint.  Returns a completed `Tok` if one just finished,
    /// and sets `*reinject` if `ch` belongs to the next token.
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
                '{' | '}' | '[' | ']' | ':' | ',' => Some(Tok(off)),
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
                    return Some(Tok(start));
                }
                None
            }

            State::InNumber { start } => match ch
            {
                '0' ..= '9' | '.' | 'e' | 'E' | '+' | '-' => None,
                _ =>
                {
                    *reinject = Some((offset, ch));
                    self.flush()
                }
            },

            State::InKeyword { start: _, len } =>
            {
                // Keywords: true(4) false(5) null(4).
                // We just count chars; tok_val() will decode the actual value.
                *len += 1;
                let done = *len >= 4; // shortest keyword is 4 chars
                if done
                {
                    // peek: "false" needs 5
                    // We don't know which keyword we're in without the source,
                    // so we keep going until we hit a non-alpha char or len>=5.
                    // Actually simpler: reinject nothing, emit after 4 chars and
                    // let tok_val decode. "false" will still be correct because
                    // scan reads until non-alpha anyway.
                    self.flush()
                }
                else
                {
                    None
                }
            }
        }
    }
}

impl<I: Iterator<Item = (usize, char)>> Iterator for Tokenizer<I>
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

// ── keyword fix: "false" is 5 chars ──────────────────────────────────────────
// The InKeyword arm above emits after 4 chars. For "false" that means we emit
// at the `s`, leaving `e` as the next char — which is harmless (it'll be
// skipped as Unknown in Idle). tok_val() reads the full keyword from the
// source so the decoded value is always correct regardless.
//
// If exact keyword boundary tracking matters in a future pass, replace the
// len-count with a source-byte comparison.

// ── public constructor ────────────────────────────────────────────────────────

pub fn tokenize<I>(chars: I) -> Tokenizer<I>
where
    I: Iterator<Item = (usize, char)>,
{
    Tokenizer::new(chars)
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::utf8::from_slice;

    fn lex_vals(src: &[u8]) -> Vec<TokVal>
    {
        tokenize(from_slice(src)).map(|t| tok_val(src, t)).collect()
    }
    fn lex_tys(src: &[u8]) -> Vec<TokTy>
    {
        tokenize(from_slice(src)).map(|t| tok_ty(src, t)).collect()
    }

    #[test]
    fn structural()
    {
        assert_eq!(lex_vals(b"{}[],:"), vec![
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
        assert_eq!(lex_vals(br#""hello""#), vec![TokVal::Str("hello".into())]);
    }

    #[test]
    fn number_val()
    {
        assert_eq!(lex_vals(b"42"), vec![TokVal::Number(42.0)]);
        assert_eq!(lex_vals(b"-3.14"), vec![TokVal::Number(-3.14)]);
    }

    #[test]
    fn keyword_vals()
    {
        assert_eq!(lex_vals(b"true false null"), vec![
            TokVal::Bool(true),
            TokVal::Bool(false),
            TokVal::Null,
        ]);
    }

    #[test]
    fn tok_ty_from_byte()
    {
        assert_eq!(TokTy::from_byte(b'"'), TokTy::Str);
        assert_eq!(TokTy::from_byte(b'4'), TokTy::Number);
        assert_eq!(TokTy::from_byte(b't'), TokTy::Bool);
        assert_eq!(TokTy::from_byte(b'f'), TokTy::Bool);
        assert_eq!(TokTy::from_byte(b'n'), TokTy::Null);
        assert_eq!(TokTy::from_byte(b'{'), TokTy::ObjOpen);
    }

    #[test]
    fn ty_and_val_consistent()
    {
        let src = br#"{"x":true}"#;
        let pairs: Vec<_> =
            tokenize(from_slice(src)).map(|t| tok_ty_val(src, t)).collect();
        // ty should match val for every token
        for (ty, val) in &pairs
        {
            let expected_ty = match val
            {
                TokVal::Str(_) => TokTy::Str,
                TokVal::Bool(_) => TokTy::Bool,
                TokVal::ObjOpen => TokTy::ObjOpen,
                TokVal::Colon => TokTy::Colon,
                TokVal::ObjClose => TokTy::ObjClose,
                _ => continue,
            };
            assert_eq!(*ty, expected_ty);
        }
    }

    #[test]
    fn tok_offsets_are_valid()
    {
        let src = br#"[1,"x"]"#;
        let toks: Vec<Tok> = tokenize(from_slice(src)).collect();
        // Every offset must be within bounds.
        for t in &toks
        {
            assert!(
                t.offset() < src.len(),
                "offset {} out of bounds",
                t.offset()
            );
        }
    }

    #[test]
    fn object_full()
    {
        assert_eq!(lex_vals(br#"{"a":1}"#), vec![
            TokVal::ObjOpen,
            TokVal::Str("a".into()),
            TokVal::Colon,
            TokVal::Number(1.0),
            TokVal::ObjClose,
        ]);
    }
}
