//! # Parser — flat row-stream
//!
//! Transforms the [`TokSpan`] stream from the lexer into a flat stream of
//! [`Row`] values.
//!
//! ## Row anatomy
//!
//! ```text
//! struct Row {
//!     id:  u32,      // monotonically increasing node id
//!     par: u32,      // parent node id
//!     key: Text,     // object key (empty for array/root elements)
//!     val: Text,     // scalar value or span of composite opening token
//!     ty:  RowTy,    // value type tag
//! }
//! ```
//!
//! ## Root detection (JSONL)
//!
//! A node whose `id == par` is a **root document**.  Consecutive JSON values
//! in a JSONL stream each become their own root.
//!
//! ## Streaming usage
//!
//! ```text
//! let mut p = Parser::new(src_bytes);
//! while let Some(row) = p.next()? {
//!     // process row
//! }
//! ```
//!
//! No full-document buffering occurs.  The only state kept is an explicit
//! parent-id stack for tracking nesting depth.

use crate::err::ParseError;
use crate::lex::{Lex, LexIter, TokKind, TokSpan};
use crate::utf8::Text;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// Type tag for a [`Row`] value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTy
{
    Str,
    Num,
    Bool,
    Null,
    /// Opening of an object `{`.  Children follow until a matching `RootEnd`.
    ObjOpen,
    /// Closing of an object `}`.
    ObjClose,
    /// Opening of an array `[`.
    ArrOpen,
    /// Closing of an array `]`.
    ArrClose,
}

/// A single node in the flat row stream.
///
/// The invariant `par < id` holds for all non-root rows.
/// For root rows `id == par` (they are self-referential).
#[derive(Debug, Clone)]
pub struct Row
{
    /// Monotonically increasing node identity.
    pub id: u32,
    /// Parent node id.  Equals `id` for JSONL roots.
    pub par: u32,
    /// Object key bytes (empty [`Text`] for non-object children).
    pub key: Text,
    /// Scalar value bytes, or the opening token bytes for composites.
    pub val: Text,
    /// Type tag.
    pub ty: RowTy,
}

impl Row
{
    /// Returns `true` if this row is a JSONL root document.
    #[inline]
    pub fn is_root(&self) -> bool
    {
        self.id == self.par
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Streaming parser over raw JSON/JSONL bytes.
///
/// Internally holds a [`LexIter`] and a parent-id stack.  Emits one [`Row`]
/// per call to [`Parser::next`].  Never buffers the whole document.
pub struct Parser<'src>
{
    src: &'src [u8],
    tokens: LexIter<'src>,
    /// Lookahead: one token peeked from the iterator.
    peeked: Option<TokSpan>,
    /// Stack of (parent_id, open_bracket) pairs for nesting.
    /// `open_bracket` is `{` or `[`.
    stack: Vec<(u32, u8)>,
    /// Next available node id.
    next_id: u32,
    /// Current object key waiting to be paired with its value token.
    pending_key: Option<Text>,
    /// Rows that have been produced but not yet consumed (e.g. ObjOpen/ArrOpen
    /// is emitted before its children, so we push children to a small queue).
    queue: std::collections::VecDeque<Row>,
}

impl<'src> Parser<'src>
{
    pub fn new(src: &'src [u8]) -> Self
    {
        Self {
            src,
            tokens: Lex::iter(src),
            peeked: None,
            stack: Vec::new(),
            next_id: 0,
            pending_key: None,
            queue: std::collections::VecDeque::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Public interface
    // -----------------------------------------------------------------------

    /// Yield the next [`Row`], or `None` at end of input.
    pub fn next(&mut self) -> Result<Option<Row>, ParseError>
    {
        // Drain queued rows first.
        if let Some(row) = self.queue.pop_front()
        {
            return Ok(Some(row));
        }
        self.advance()
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn peek_tok(&mut self) -> Result<Option<&TokSpan>, ParseError>
    {
        if self.peeked.is_none()
        {
            self.peeked = self.tokens.next().transpose()?;
        }
        Ok(self.peeked.as_ref())
    }

    fn consume_tok(&mut self) -> Result<Option<TokSpan>, ParseError>
    {
        if let Some(t) = self.peeked.take()
        {
            return Ok(Some(t));
        }
        Ok(self.tokens.next().transpose()?)
    }

    fn skip_ws_structurals(&mut self) -> Result<Option<TokSpan>, ParseError>
    {
        // The lexer already skips whitespace; we just need to skip `,` and `:`.
        loop
        {
            match self.consume_tok()?
            {
                None => return Ok(None),
                Some(t)
                    if matches!(t.kind, TokKind::Comma | TokKind::Colon) =>
                {
                    continue;
                }
                Some(t) => return Ok(Some(t)),
            }
        }
    }

    fn alloc_id(&mut self) -> u32
    {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn current_parent(&self) -> u32
    {
        self.stack.last().map(|(id, _)| *id).unwrap_or(u32::MAX)
    }

    fn span_text(span: &TokSpan) -> Text
    {
        Text::new(span.start, span.end)
    }

    fn empty_text() -> Text
    {
        Text::new(0, 0)
    }

    /// Core state machine: produce the next row (or None).
    fn advance(&mut self) -> Result<Option<Row>, ParseError>
    {
        let tok = match self.skip_ws_structurals()?
        {
            None => return Ok(None),
            Some(t) => t,
        };

        match tok.kind
        {
            // ------------------------------------------------------------------
            // Closing brackets — pop stack
            // ------------------------------------------------------------------
            TokKind::RBrace =>
            {
                let (par_id, open) = self.stack.pop().ok_or_else(|| {
                    ParseError::UnexpectedToken {
                        offset: tok.start as usize,
                        got: "}".into(),
                        expected: "matching {",
                    }
                })?;
                if open != b'{'
                {
                    return Err(ParseError::MismatchedBracket {
                        bracket: '{',
                        open_offset: 0,
                        close_offset: tok.start as usize,
                    });
                }
                let id = self.alloc_id();
                return Ok(Some(Row {
                    id,
                    par: par_id,
                    key: Self::empty_text(),
                    val: Self::span_text(&tok),
                    ty: RowTy::ObjClose,
                }));
            }

            TokKind::RBracket =>
            {
                let (par_id, open) = self.stack.pop().ok_or_else(|| {
                    ParseError::UnexpectedToken {
                        offset: tok.start as usize,
                        got: "]".into(),
                        expected: "matching [",
                    }
                })?;
                if open != b'['
                {
                    return Err(ParseError::MismatchedBracket {
                        bracket: '[',
                        open_offset: 0,
                        close_offset: tok.start as usize,
                    });
                }
                let id = self.alloc_id();
                return Ok(Some(Row {
                    id,
                    par: par_id,
                    key: Self::empty_text(),
                    val: Self::span_text(&tok),
                    ty: RowTy::ArrClose,
                }));
            }

            // ------------------------------------------------------------------
            // Object opening
            // ------------------------------------------------------------------
            TokKind::LBrace =>
            {
                let id = self.alloc_id();
                // If stack is empty this is a new JSONL root → par = id.
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                self.stack.push((id, b'{'));
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::ObjOpen,
                }));
            }

            // ------------------------------------------------------------------
            // Array opening
            // ------------------------------------------------------------------
            TokKind::LBracket =>
            {
                let id = self.alloc_id();
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                self.stack.push((id, b'['));
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::ArrOpen,
                }));
            }

            // ------------------------------------------------------------------
            // String: could be an object key or a value
            // ------------------------------------------------------------------
            TokKind::Str =>
            {
                // Peek: if next real token is `:` we are an object key.
                // (skip_ws_structurals already consumes `:`, so we peek raw)
                let is_key = {
                    // Look at raw next token before consuming.
                    let next = self.peek_tok()?;
                    matches!(next.map(|t| t.kind), Some(TokKind::Colon))
                };

                if is_key
                {
                    // Consume the colon.
                    self.pending_key = Some(Self::span_text(&tok));
                    let _ = self.consume_tok()?; // ':'
                    // Recurse to get the value token.
                    return self.advance();
                }

                // It's a scalar value.
                let id = self.alloc_id();
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::Str,
                }));
            }

            // ------------------------------------------------------------------
            // Scalars
            // ------------------------------------------------------------------
            TokKind::Num =>
            {
                let id = self.alloc_id();
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::Num,
                }));
            }

            TokKind::True | TokKind::False =>
            {
                let id = self.alloc_id();
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::Bool,
                }));
            }

            TokKind::Null =>
            {
                let id = self.alloc_id();
                let par = if self.stack.is_empty()
                {
                    id
                }
                else
                {
                    self.current_parent()
                };
                let key =
                    self.pending_key.take().unwrap_or_else(Self::empty_text);
                return Ok(Some(Row {
                    id,
                    par,
                    key,
                    val: Self::span_text(&tok),
                    ty: RowTy::Null,
                }));
            }

            // `:` and `,` were already consumed by skip_ws_structurals — if
            // we somehow still get them something is wrong.
            TokKind::Colon | TokKind::Comma =>
            {
                return Err(ParseError::UnexpectedToken {
                    offset: tok.start as usize,
                    got: format!("{:?}", tok.kind),
                    expected: "value",
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{
    use super::*;

    fn parse_all(src: &[u8]) -> Vec<Row>
    {
        let mut p = Parser::new(src);
        let mut rows = Vec::new();
        loop
        {
            match p.next().unwrap()
            {
                None => break,
                Some(r) => rows.push(r),
            }
        }
        rows
    }

    #[test]
    fn scalar_is_root()
    {
        let rows = parse_all(b"42");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_root());
        assert_eq!(rows[0].ty, RowTy::Num);
    }

    #[test]
    fn jsonl_two_roots_have_independent_ids()
    {
        let rows = parse_all(b"1\n2\n");
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_root());
        assert!(rows[1].is_root());
        // Roots should have distinct ids.
        assert_ne!(rows[0].id, rows[1].id);
    }

    #[test]
    fn object_parent_ids()
    {
        // {"a":1}
        let rows = parse_all(br#"{"a":1}"#);
        // ObjOpen (root), Num child, ObjClose
        assert_eq!(rows.len(), 3);
        let obj = &rows[0];
        let val = &rows[1];
        let close = &rows[2];
        assert!(obj.is_root(), "ObjOpen should be root");
        assert_eq!(val.par, obj.id, "child par should be object id");
        assert_eq!(close.par, obj.id, "close par should be object id");
    }

    #[test]
    fn object_key_stored()
    {
        let rows = parse_all(br#"{"hello":42}"#);
        let val_row = rows.iter().find(|r| r.ty == RowTy::Num).unwrap();
        // key bytes 1..6 should be `"hello"` (with quotes)
        let key = val_row.key;
        assert!(key.byte_len() > 0, "key should be non-empty");
    }

    #[test]
    fn array_children_parent()
    {
        let rows = parse_all(b"[1,2,3]");
        // ArrOpen, Num, Num, Num, ArrClose
        assert_eq!(rows.len(), 5);
        let arr = &rows[0];
        for i in 1 .. 4
        {
            assert_eq!(rows[i].par, arr.id);
        }
    }

    #[test]
    fn nested_object()
    {
        let rows = parse_all(br#"{"a":{"b":1}}"#);
        // ObjOpen(root), ObjOpen(child), Num, ObjClose, ObjClose
        assert_eq!(rows.len(), 5);
        assert!(rows[0].is_root());
        assert!(!rows[1].is_root());
        assert_eq!(rows[1].par, rows[0].id);
    }

    #[test]
    fn parent_id_lt_child_id()
    {
        let rows = parse_all(br#"{"a":[1,{"b":2}]}"#);
        for row in &rows
        {
            if !row.is_root()
            {
                assert!(
                    row.par < row.id,
                    "invariant violated: par={} id={}",
                    row.par,
                    row.id
                );
            }
        }
    }
}
