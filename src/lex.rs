/// lex.rs — Streaming JSON tokenizer.
///
/// `Tokenizer` wraps a `Utf8Iter` and implements `Iterator<Item = Token>`.
/// It reacts to each codepoint as it arrives from the upstream iterator —
/// no buffering of the full source, no structural pre-scan.
///
/// When the upstream yields `None` the tokenizer flushes any in-progress
/// token and then itself returns `None` on the next call.
///
/// # Token
///
/// Values that require accumulation (strings, numbers, keywords) are returned
/// as owned `String` / `f64` so the tokenizer owns its output independently
/// of the source lifetime.

#[derive(Debug, Clone, PartialEq)]
pub enum Token
{
    /// Interned string content (quotes stripped, no unescaping yet).
    Str(String),
    Number(f64),
    Bool(bool),
    Null,
    ObjOpen,  // {
    ObjClose, // }
    ArrOpen,  // [
    ArrClose, // ]
    Colon,    // :
    Comma,    // ,
}

impl Token
{
    pub fn display(&self) -> String
    {
        match self
        {
            Token::Str(s) => format!("Str({:?})", s),
            Token::Number(n) => format!("Number({})", n),
            Token::Bool(b) => format!("Bool({})", b),
            Token::Null => "Null".into(),
            Token::ObjOpen => "ObjOpen  {".into(),
            Token::ObjClose => "ObjClose }".into(),
            Token::ArrOpen => "ArrOpen  [".into(),
            Token::ArrClose => "ArrClose ]".into(),
            Token::Colon => "Colon    :".into(),
            Token::Comma => "Comma    ,".into(),
        }
    }
}

// ── internal tokenizer state ──────────────────────────────────────────────────

#[derive(Debug)]
enum State
{
    /// Between tokens — eating whitespace.
    Idle,
    /// Inside a `"..."` string.  Accumulates content (after opening `"`).
    InString
    {
        buf: String, escaped: bool
    },
    /// Inside a number literal.
    InNumber
    {
        buf: String, start: usize
    },
    /// Inside an identifier keyword (true / false / null).
    InKeyword
    {
        buf: String, start: usize
    },
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

pub struct Tokenizer<I: Iterator<Item = (usize, char)>>
{
    chars: I,
    state: State,
    /// Tokens ready to be yielded (at most 1 buffered at a time).
    pending: Option<Token>,
    done: bool,
}

impl<I: Iterator<Item = (usize, char)>> Tokenizer<I>
{
    pub fn new(chars: I) -> Self
    {
        Self { chars, state: State::Idle, pending: None, done: false }
    }

    // ── helpers ───────────────────────────────────────────────────────────

    /// Finalise an in-progress number or keyword token.
    /// Returns the completed token, leaving state = Idle.
    fn flush_accumulator(&mut self) -> Option<Token>
    {
        let old = std::mem::replace(&mut self.state, State::Idle);
        match old
        {
            State::InNumber { buf, .. } =>
            {
                let n = buf.parse::<f64>().unwrap_or(f64::NAN);
                Some(Token::Number(n))
            }
            State::InKeyword { buf, .. } => match buf.as_str()
            {
                "true" => Some(Token::Bool(true)),
                "false" => Some(Token::Bool(false)),
                "null" => Some(Token::Null),
                _ => None, // malformed — silently drop
            },
            _ => None,
        }
    }

    /// Process one `(offset, char)` pair from the upstream iterator.
    /// Returns a completed token if this codepoint terminates one, or `None`
    /// if we're still accumulating.
    ///
    /// For single-character structural tokens this is always `Some(tok)`.
    /// For multi-character tokens it returns `Some` only when the token ends.
    ///
    /// `reinject` is set to `Some(ch)` when `ch` belongs to the *next* token
    /// (e.g. the character that ended a number by not being a digit).
    fn step(
        &mut self,
        offset: usize,
        ch: char,
        reinject: &mut Option<(usize, char)>,
    ) -> Option<Token>
    {
        match &mut self.state
        {
            // ── Idle ──────────────────────────────────────────────────────
            State::Idle => match ch
            {
                // Whitespace — stay idle.
                ' ' | '\t' | '\r' | '\n' => None,

                // Single-character structural tokens.
                '{' => Some(Token::ObjOpen),
                '}' => Some(Token::ObjClose),
                '[' => Some(Token::ArrOpen),
                ']' => Some(Token::ArrClose),
                ':' => Some(Token::Colon),
                ',' => Some(Token::Comma),

                // String open.
                '"' =>
                {
                    self.state =
                        State::InString { buf: String::new(), escaped: false };
                    None
                }

                // Number start.
                '-' | '0' ..= '9' =>
                {
                    let mut buf = String::new();
                    buf.push(ch);
                    self.state = State::InNumber { buf, start: offset };
                    None
                }

                // Keyword start.
                't' | 'f' | 'n' =>
                {
                    let mut buf = String::new();
                    buf.push(ch);
                    self.state = State::InKeyword { buf, start: offset };
                    None
                }

                // Anything else — skip (could log a warning).
                _ => None,
            },

            // ── InString ─────────────────────────────────────────────────
            State::InString { buf, escaped } =>
            {
                if *escaped
                {
                    buf.push(ch);
                    *escaped = false;
                    None
                }
                else if ch == '\\'
                {
                    *escaped = true;
                    None
                }
                else if ch == '"'
                {
                    // End of string.
                    let s = std::mem::take(buf);
                    self.state = State::Idle;
                    Some(Token::Str(s))
                }
                else
                {
                    buf.push(ch);
                    None
                }
            }

            // ── InNumber ─────────────────────────────────────────────────
            State::InNumber { buf, .. } =>
            {
                match ch
                {
                    '0' ..= '9' | '.' | 'e' | 'E' | '+' | '-' =>
                    {
                        buf.push(ch);
                        None
                    }
                    _ =>
                    {
                        // This character ends the number; reinject it.
                        *reinject = Some((offset, ch));
                        self.flush_accumulator()
                    }
                }
            }

            // ── InKeyword ────────────────────────────────────────────────
            State::InKeyword { buf, .. } =>
            {
                if ch.is_ascii_alphabetic()
                {
                    buf.push(ch);
                    // Emit as soon as we have enough bytes.
                    let done =
                        matches!(buf.as_str(), "true" | "false" | "null");
                    if done { self.flush_accumulator() } else { None }
                }
                else
                {
                    *reinject = Some((offset, ch));
                    self.flush_accumulator()
                }
            }
        }
    }
}

// ── Iterator impl ─────────────────────────────────────────────────────────────

impl<I: Iterator<Item = (usize, char)>> Iterator for Tokenizer<I>
{
    type Item = Token;

    fn next(&mut self) -> Option<Token>
    {
        // Return any token that was completed and buffered in a previous call.
        if let Some(t) = self.pending.take()
        {
            return Some(t);
        }
        if self.done
        {
            return None;
        }

        // We may have a character reinjected from the previous step
        // (e.g. the `{` that terminated a number).  Track it here.
        let mut reinjected: Option<(usize, char)> = None;

        loop
        {
            let (offset, ch) = if let Some(pair) = reinjected.take()
            {
                pair
            }
            else
            {
                match self.chars.next()
                {
                    Some(pair) => pair,
                    None =>
                    {
                        // EOF — flush any in-progress accumulator.
                        self.done = true;
                        return self.flush_accumulator();
                    }
                }
            };

            let mut reinject: Option<(usize, char)> = None;
            if let Some(tok) = self.step(offset, ch, &mut reinject)
            {
                // If there's a reinjected character, buffer it for the
                // next call by processing it immediately into `pending`.
                if let Some((ro, rc)) = reinject
                {
                    let mut ri2: Option<(usize, char)> = None;
                    self.pending = self.step(ro, rc, &mut ri2);
                    // ri2 is rare (two consecutive single-char boundaries);
                    // we ignore it for now since structural tokens never reinject.
                }
                return Some(tok);
            }

            // If step reinjected a character, loop around with it.
            reinjected = reinject;
        }
    }
}

// ── public constructor ────────────────────────────────────────────────────────

/// Wrap a `(usize, char)` iterator (e.g. from `utf8::Utf8Iter`) into a
/// `Tokenizer`.
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

    fn lex(src: &[u8]) -> Vec<Token>
    {
        tokenize(from_slice(src)).collect()
    }

    #[test]
    fn structural_chars()
    {
        assert_eq!(lex(b"{}[],:"), vec![
            Token::ObjOpen,
            Token::ObjClose,
            Token::ArrOpen,
            Token::ArrClose,
            Token::Comma,
            Token::Colon,
        ]);
    }

    #[test]
    fn string_token()
    {
        assert_eq!(lex(br#""hello""#), vec![Token::Str("hello".into())]);
    }

    #[test]
    fn number_int()
    {
        assert_eq!(lex(b"42"), vec![Token::Number(42.0)]);
    }

    #[test]
    fn number_float()
    {
        assert_eq!(lex(b"-3.14"), vec![Token::Number(-3.14)]);
    }

    #[test]
    fn keywords()
    {
        assert_eq!(lex(b"true false null"), vec![
            Token::Bool(true),
            Token::Bool(false),
            Token::Null,
        ]);
    }

    #[test]
    fn object()
    {
        let tokens = lex(br#"{"a":1}"#);
        assert_eq!(tokens, vec![
            Token::ObjOpen,
            Token::Str("a".into()),
            Token::Colon,
            Token::Number(1.0),
            Token::ObjClose,
        ]);
    }

    #[test]
    fn array_of_mixed()
    {
        let tokens = lex(br#"[1, "x", true, null]"#);
        assert_eq!(tokens, vec![
            Token::ArrOpen,
            Token::Number(1.0),
            Token::Comma,
            Token::Str("x".into()),
            Token::Comma,
            Token::Bool(true),
            Token::Comma,
            Token::Null,
            Token::ArrClose,
        ]);
    }

    #[test]
    fn jsonl_two_objects()
    {
        let tokens = lex(b"{\"a\":1}\n{\"b\":2}");
        // Should produce tokens for both objects without any separator token.
        assert!(tokens.contains(&Token::Str("a".into())));
        assert!(tokens.contains(&Token::Str("b".into())));
    }
}
