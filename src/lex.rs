//! # Lexer
//!
//! Two-phase design:
//!
//! 1. **Structural index** — a fast pass over raw bytes that marks
//!    *pseudo-structural* positions: any byte that begins a JSON value
//!    (`"`, `-`, `0-9`, `t`, `f`, `n`, `{`, `[`) that is preceded by a
//!    whitespace or structural character.  This pass never allocates tokens.
//!
//! 2. **Token materialisation** — a second pass that starts at a structural
//!    byte offset and fully parses one [`TokKind`] worth of bytes, returning
//!    a [`TokSpan`].
//!
//! ## Two usage modes
//!
//! ```text
//! // Mode 1 — streaming iterator, no storage
//! for span in Lex::iter(src) { ... }
//!
//! // Mode 2 — random-access store
//! let store = Lex::lex(src);
//! let byte_offset: u32  = store.tok(logical_idx);   // logical → byte
//! let span: TokSpan     = store.at(byte_offset);    // byte → TokSpan
//! // Composition: Lex::at(Lex::tok(0)) = first TokSpan
//! ```

use std::collections::HashMap;

use crate::err::LexError;
use crate::utf8::{Tok, Utf8Buf};

// ---------------------------------------------------------------------------
// Token kinds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind
{
    /// `"..."` — the span includes the quote delimiters.
    Str,
    /// Integer or float.
    Num,
    /// `true`
    True,
    /// `false`
    False,
    /// `null`
    Null,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `:`
    Colon,
    /// `,`
    Comma,
}

/// A fully parsed token: kind + byte range in the source [`Utf8Buf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokSpan
{
    pub kind: TokKind,
    /// Start byte (inclusive).
    pub start: u32,
    /// End byte (exclusive).
    pub end: u32,
}

impl TokSpan
{
    /// Resolve the raw source bytes for this token.
    pub fn src<'b>(&self, buf: &'b Utf8Buf) -> &'b str
    {
        // SAFETY: lexer only emits spans with valid UTF-8 boundaries.
        unsafe {
            std::str::from_utf8_unchecked(
                &buf.as_bytes_raw()[self.start as usize .. self.end as usize],
            )
        }
    }

    /// The [`Tok`] (start-byte pointer) for this span.
    pub fn tok(&self) -> Tok
    {
        Tok(self.start)
    }
}

// ---------------------------------------------------------------------------
// Structural indexing helpers
// ---------------------------------------------------------------------------

/// Returns `true` if this byte can begin a JSON value.
#[inline]
fn is_value_start(b: u8) -> bool
{
    matches!(b, b'"' | b'-' | b'0' ..= b'9' | b't' | b'f' | b'n' | b'{' | b'[')
}

/// Returns `true` if this byte is JSON whitespace.
#[inline]
fn is_ws(b: u8) -> bool
{
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// Returns `true` if this byte is a JSON structural character.
#[inline]
fn is_structural(b: u8) -> bool
{
    matches!(b, b'{' | b'}' | b'[' | b']' | b':' | b',')
}

/// Collect all pseudo-structural byte offsets from `src`.
///
/// The scan is **string-aware**: whenever a `"` is encountered outside of a
/// string context, the scanner records that position and then fast-forwards
/// past the entire string body (honouring `\` escapes) to the closing `"`.
/// This prevents bytes *inside* string values (e.g. `-` in `"Milky Way - Sol"`)
/// from being misidentified as value-start positions.
///
/// A byte at position `i` is pseudo-structural when one of:
///
/// 1. It is `"` and outside a string — the whole string starting here is one
///    token; its interior bytes are skipped.
/// 2. It is a non-string value-start (`-`, `0-9`, `t`, `f`, `n`, `{`, `[`)
///    and the previous byte is whitespace or a structural character.
/// 3. It is a structural punctuation character (`{`, `}`, `[`, `]`, `:`, `,`).
pub fn structural_indices(src: &[u8]) -> Vec<u32>
{
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < src.len()
    {
        let b = src[i];
        if b == b'"'
        {
            // Record the opening quote as a structural position, then skip
            // past the entire string so its interior bytes are invisible.
            out.push(i as u32);
            i += 1; // step past opening `"`
            while i < src.len()
            {
                match src[i]
                {
                    b'"' =>
                    {
                        i += 1;
                        break;
                    } // closing quote — done
                    b'\\' =>
                    {
                        i += 2;
                    } // escape sequence — skip both bytes
                    _ =>
                    {
                        i += 1;
                    }
                }
            }
        }
        else if is_structural(b)
        {
            out.push(i as u32);
            i += 1;
        }
        else
        {
            // Non-string value-start only if preceded by ws or structural.
            let prev_ok = i == 0 || {
                let p = src[i - 1];
                is_ws(p) || is_structural(p)
            };
            if is_value_start(b) && prev_ok
            {
                out.push(i as u32);
            }
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Token materialisation
// ---------------------------------------------------------------------------

/// Parse exactly one token starting at `offset` in `src`.
///
/// Returns `(TokSpan, bytes_consumed)` or a [`LexError`].
pub fn lex_one(src: &[u8], offset: usize) -> Result<TokSpan, LexError>
{
    if offset >= src.len()
    {
        return Err(LexError::UnexpectedEof { offset, context: "token" });
    }
    let b = src[offset];
    match b
    {
        b'"' => lex_string(src, offset),
        b'-' | b'0' ..= b'9' => lex_number(src, offset),
        b't' => lex_literal(src, offset, b"true", TokKind::True),
        b'f' => lex_literal(src, offset, b"false", TokKind::False),
        b'n' => lex_literal(src, offset, b"null", TokKind::Null),
        b'{' => Ok(punct(TokKind::LBrace, offset)),
        b'}' => Ok(punct(TokKind::RBrace, offset)),
        b'[' => Ok(punct(TokKind::LBracket, offset)),
        b']' => Ok(punct(TokKind::RBracket, offset)),
        b':' => Ok(punct(TokKind::Colon, offset)),
        b',' => Ok(punct(TokKind::Comma, offset)),
        _ => Err(LexError::InvalidLiteral {
            offset,
            got: format!("{:?}", b as char),
        }),
    }
}

#[inline]
fn punct(kind: TokKind, offset: usize) -> TokSpan
{
    TokSpan { kind, start: offset as u32, end: (offset + 1) as u32 }
}

fn lex_string(src: &[u8], start: usize) -> Result<TokSpan, LexError>
{
    debug_assert_eq!(src[start], b'"');
    let mut i = start + 1;
    loop
    {
        match src.get(i)
        {
            None =>
            {
                return Err(LexError::UnterminatedString { offset: start });
            }
            Some(&b'\\') =>
            {
                // Consume escape sequence.
                i += 1;
                match src.get(i)
                {
                    None =>
                    {
                        return Err(LexError::UnterminatedString {
                            offset: start,
                        });
                    }
                    Some(&b'u') =>
                    {
                        // \uXXXX — skip 4 hex digits.
                        i += 1;
                        for _ in 0 .. 4
                        {
                            match src.get(i)
                            {
                                Some(&c) if c.is_ascii_hexdigit() => i += 1,
                                _ =>
                                {
                                    return Err(LexError::InvalidEscape {
                                        offset: i,
                                        ch: 'u',
                                    });
                                }
                            }
                        }
                    }
                    Some(&esc) =>
                    {
                        if matches!(
                            esc,
                            b'"' | b'\\'
                                | b'/'
                                | b'b'
                                | b'f'
                                | b'n'
                                | b'r'
                                | b't'
                        )
                        {
                            i += 1;
                        }
                        else
                        {
                            return Err(LexError::InvalidEscape {
                                offset: i,
                                ch: esc as char,
                            });
                        }
                    }
                }
            }
            Some(&b'"') =>
            {
                i += 1;
                return Ok(TokSpan {
                    kind: TokKind::Str,
                    start: start as u32,
                    end: i as u32,
                });
            }
            Some(_) =>
            {
                i += 1;
            }
        }
    }
}

fn lex_number(src: &[u8], start: usize) -> Result<TokSpan, LexError>
{
    let mut i = start;
    // Optional leading minus.
    if src.get(i) == Some(&b'-')
    {
        i += 1;
    }
    // Integer part.
    if src.get(i) == Some(&b'0')
    {
        i += 1;
    }
    else if matches!(src.get(i), Some(b'1' ..= b'9'))
    {
        while matches!(src.get(i), Some(b'0' ..= b'9'))
        {
            i += 1;
        }
    }
    else
    {
        return Err(LexError::InvalidNumber { offset: start });
    }
    // Optional fractional part.
    if src.get(i) == Some(&b'.')
    {
        i += 1;
        if !matches!(src.get(i), Some(b'0' ..= b'9'))
        {
            return Err(LexError::InvalidNumber { offset: start });
        }
        while matches!(src.get(i), Some(b'0' ..= b'9'))
        {
            i += 1;
        }
    }
    // Optional exponent.
    if matches!(src.get(i), Some(b'e') | Some(b'E'))
    {
        i += 1;
        if matches!(src.get(i), Some(b'+') | Some(b'-'))
        {
            i += 1;
        }
        if !matches!(src.get(i), Some(b'0' ..= b'9'))
        {
            return Err(LexError::InvalidNumber { offset: start });
        }
        while matches!(src.get(i), Some(b'0' ..= b'9'))
        {
            i += 1;
        }
    }
    Ok(TokSpan { kind: TokKind::Num, start: start as u32, end: i as u32 })
}

fn lex_literal(
    src: &[u8],
    start: usize,
    lit: &[u8],
    kind: TokKind,
) -> Result<TokSpan, LexError>
{
    let end = start + lit.len();
    if src.get(start .. end) == Some(lit)
    {
        Ok(TokSpan { kind, start: start as u32, end: end as u32 })
    }
    else
    {
        Err(LexError::InvalidLiteral {
            offset: start,
            got: String::from_utf8_lossy(&src[start .. end.min(src.len())])
                .into_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Mode 1 — streaming iterator
// ---------------------------------------------------------------------------

/// Streaming token iterator.  No tokens are stored.
///
/// Created with [`Lex::iter`].
pub struct LexIter<'src>
{
    src: &'src [u8],
    /// Index into the structural index vector.
    idx: usize,
    /// Precomputed structural offsets.
    offsets: Vec<u32>,
}

impl<'src> LexIter<'src>
{
    fn new(src: &'src [u8]) -> Self
    {
        Self { offsets: structural_indices(src), src, idx: 0 }
    }
}

impl<'src> Iterator for LexIter<'src>
{
    type Item = Result<TokSpan, LexError>;

    fn next(&mut self) -> Option<Self::Item>
    {
        loop
        {
            let off = *self.offsets.get(self.idx)? as usize;
            self.idx += 1;
            // Skip pure whitespace positions (structural_indices already
            // filters, but guard anyway).
            if off < self.src.len() && !is_ws(self.src[off])
            {
                return Some(lex_one(self.src, off));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mode 2 — random-access store
// ---------------------------------------------------------------------------

/// Token store built from a full parse of the source.
///
/// Maps:
/// - logical index → byte offset  (`tok_map`)
/// - byte offset   → [`TokSpan`]  (`span_map`)
pub struct LexStore
{
    /// logical_index → byte_offset
    tok_map: HashMap<u32, u32>,
    /// byte_offset   → TokSpan
    span_map: HashMap<u32, TokSpan>,
}

impl LexStore
{
    fn build(src: &[u8]) -> Result<Self, LexError>
    {
        let offsets = structural_indices(src);
        let mut tok_map = HashMap::with_capacity(offsets.len());
        let mut span_map = HashMap::with_capacity(offsets.len());
        let mut logical = 0u32;

        for off in offsets
        {
            let uoff = off as usize;
            if uoff < src.len() && !is_ws(src[uoff])
            {
                let span = lex_one(src, uoff)?;
                tok_map.insert(logical, off);
                span_map.insert(off, span);
                logical += 1;
            }
        }

        Ok(Self { tok_map, span_map })
    }

    /// Translate a logical (sequential) token index to a byte offset.
    ///
    /// Returns `None` if the index is out of range.
    pub fn tok(&self, logical_index: u32) -> Option<u32>
    {
        self.tok_map.get(&logical_index).copied()
    }

    /// Retrieve the [`TokSpan`] at the given byte offset.
    ///
    /// Returns `None` if `byte_offset` is not a token start.
    pub fn at(&self, byte_offset: u32) -> Option<&TokSpan>
    {
        self.span_map.get(&byte_offset)
    }

    /// Convenience: logical index → [`TokSpan`].
    ///
    /// Equivalent to `store.at(store.tok(n)?)`.
    pub fn span(&self, logical_index: u32) -> Option<&TokSpan>
    {
        let off = self.tok(logical_index)?;
        self.at(off)
    }

    /// Total number of tokens stored.
    pub fn len(&self) -> usize
    {
        self.tok_map.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.tok_map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Public entry-point
// ---------------------------------------------------------------------------

/// Lexer entry point — two static constructors, two usage modes.
pub struct Lex;

impl Lex
{
    /// **Mode 1** — streaming iterator.  Produces [`TokSpan`]s one at a time;
    /// nothing is stored.
    ///
    /// ```text
    /// for span in Lex::iter(src) { ... }
    /// ```
    pub fn iter(src: &[u8]) -> LexIter<'_>
    {
        LexIter::new(src)
    }

    /// **Mode 2** — full parse, random-access store.
    ///
    /// ```text
    /// let store = Lex::lex(src)?;
    /// let byte_off = store.tok(0)?;          // logical → byte
    /// let span     = store.at(byte_off)?;    // byte → TokSpan
    /// // or shorthand:
    /// let span     = store.span(0)?;
    /// ```
    pub fn lex(src: &[u8]) -> Result<LexStore, LexError>
    {
        LexStore::build(src)
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{
    use super::*;

    const SRC: &[u8] = br#"{"key": 42, "arr": [true, false, null]}"#;

    #[test]
    fn structural_index_not_empty()
    {
        let idx = structural_indices(SRC);
        assert!(!idx.is_empty());
    }

    #[test]
    fn streaming_iter_counts_tokens()
    {
        let spans: Result<Vec<_>, _> = Lex::iter(SRC).collect();
        let spans = spans.unwrap();
        // { "key" : 42 , "arr" : [ true , false , null ] }
        assert_eq!(spans.len(), 13);
    }

    #[test]
    fn store_logical_access()
    {
        let store = Lex::lex(SRC).unwrap();
        // Token 0 should be `{`
        let span = store.span(0).unwrap();
        assert_eq!(span.kind, TokKind::LBrace);
        assert_eq!(&SRC[span.start as usize .. span.end as usize], b"{");
    }

    #[test]
    fn store_byte_access()
    {
        let store = Lex::lex(SRC).unwrap();
        // The `{` is at byte 0.
        let span = store.at(0).unwrap();
        assert_eq!(span.kind, TokKind::LBrace);
    }

    #[test]
    fn store_composed_access()
    {
        let store = Lex::lex(SRC).unwrap();
        // store.at(store.tok(0)) == store.span(0)
        let byte_off = store.tok(0).unwrap();
        let via_at = store.at(byte_off).unwrap();
        let via_span = store.span(0).unwrap();
        assert_eq!(via_at, via_span);
    }

    #[test]
    fn string_token()
    {
        let src = br#""hello world""#;
        let store = Lex::lex(src).unwrap();
        let span = store.span(0).unwrap();
        assert_eq!(span.kind, TokKind::Str);
        assert_eq!(
            &src[span.start as usize .. span.end as usize],
            br#""hello world""#
        );
    }

    #[test]
    fn number_token()
    {
        let src = b"-3.14e+2";
        let store = Lex::lex(src).unwrap();
        let span = store.span(0).unwrap();
        assert_eq!(span.kind, TokKind::Num);
    }

    #[test]
    fn literal_tokens()
    {
        let src = b"true false null";
        let spans: Vec<_> = Lex::iter(src).map(|r| r.unwrap()).collect();
        assert_eq!(spans[0].kind, TokKind::True);
        assert_eq!(spans[1].kind, TokKind::False);
        assert_eq!(spans[2].kind, TokKind::Null);
    }

    #[test]
    fn jsonl_two_roots()
    {
        let src = b"1\n2\n";
        let spans: Vec<_> = Lex::iter(src).map(|r| r.unwrap()).collect();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].kind, TokKind::Num);
        assert_eq!(spans[1].kind, TokKind::Num);
    }
}
