use pq::txt::buf::Utf8Buffer;

use crate::lexer::{TokenKind, classify_token};

pub type TextOffset = u32;

// ! This design has a flaw:
// ! There are handles *after* the source len, which means it can't append more

// ! This mixes tokenization with string interning and that's something that's
// ! tracked usually by each subsystem based on their unique usage patterns

/// Append-only storage for token-backed text handles.
///
/// Handles below `source_len` are source byte offsets. Handles at/after
/// `source_len` are interned values terminated by `\0`.
pub struct KeyValBuf
{
    buf: Utf8Buffer,
    source_len: u32,
}

impl KeyValBuf
{
    pub fn from_source(source: &str) -> Self
    {
        let base = source.len();
        let cap = base.saturating_mul(8).max(4096);
        let buf = Utf8Buffer::new(cap);
        buf.append_str(source);
        Self { buf, source_len: base as u32 }
    }

    #[inline]
    pub fn source(&self) -> &str
    {
        &self.buf.as_str()[.. self.source_len as usize]
    }

    #[inline]
    pub fn source_len(&self) -> u32
    {
        self.source_len
    }

    /// Interns a synthetic string and returns its handle.
    pub fn intern(&self, s: &str) -> TextOffset
    {
        let at = self.buf.len() as TextOffset;
        self.buf.append_str(s);
        self.buf.append(&[0]);
        at
    }

    pub fn classify(&self, at: TextOffset) -> TokenKind
    {
        if at >= self.source_len
        {
            return TokenKind::String;
        }

        classify_token(self.source(), at as usize)
    }

    pub fn token_end(&self, at: TextOffset) -> TextOffset
    {
        if at >= self.source_len
        {
            let data = self.buf.as_str().as_bytes();
            let mut i = at as usize;
            while i < data.len() && data[i] != 0
            {
                i += 1;
            }
            return i as TextOffset;
        }

        let src = self.source().as_bytes();
        let start = at as usize;
        let mut end = start;

        match self.classify(at)
        {
            TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::Colon
            | TokenKind::Comma => end += 1,
            TokenKind::True => end += 4,
            TokenKind::False => end += 5,
            TokenKind::Null => end += 4,
            TokenKind::Number =>
            {
                while end < src.len()
                {
                    match src[end]
                    {
                        b'0' ..= b'9' | b'.' | b'e' | b'E' | b'+' | b'-' =>
                        {
                            end += 1
                        }
                        _ => break,
                    }
                }
            }
            TokenKind::String =>
            {
                end += 1;
                while end < src.len()
                {
                    if src[end] == b'"'
                    {
                        end += 1;
                        break;
                    }
                    end += 1;
                }
            }
            TokenKind::Unknown => end = start,
        }

        end as TextOffset
    }

    pub fn token_slice(&self, at: TextOffset) -> &str
    {
        let end = self.token_end(at);
        &self.buf.as_str()[at as usize .. end as usize]
    }

    pub fn text_value(&self, at: TextOffset) -> &str
    {
        if at >= self.source_len
        {
            return self.token_slice(at);
        }

        let slice = self.token_slice(at);
        if slice.starts_with('"') && slice.ends_with('"') && slice.len() >= 2
        {
            &slice[1 .. slice.len() - 1]
        }
        else
        {
            slice
        }
    }
}
