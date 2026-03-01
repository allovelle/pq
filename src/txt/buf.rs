//! Append-only UTF-8 byte buffer.
//! Supports partial UTF-8 appends and exposes only offset/span handles.

use std::ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo};

use crate::txt::iter::{Utf8Iter, Utf8PartIter};
use crate::txt::utf8::{utf8_byte_udx_after, utf8_char_on, utf8_codepoint_len};
use crate::txt::view::CodepointView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan
{
    pub start: usize,
    pub end: usize,
}

impl TextSpan
{
    pub const fn new(start: usize, end: usize) -> Self
    {
        Self { start, end }
    }

    pub const fn len(self) -> usize
    {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool
    {
        self.start == self.end
    }
}

/// Append-only UTF-8 byte buffer.
///
/// - Stores all appended bytes (including partial trailing UTF-8 fragments).
/// - Maintains `valid_end`: largest prefix that is valid UTF-8.
/// - All safe string operations are bounded by `valid_end`.
#[derive(Debug, Clone, Default)]
pub struct Utf8Buffer
{
    bytes: Vec<u8>,
    valid_end: usize,
}

impl Utf8Buffer
{
    pub fn new(capacity: usize) -> Self
    {
        Self { bytes: Vec::with_capacity(capacity), valid_end: 0 }
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.bytes.len()
    }

    #[inline]
    pub fn valid_len(&self) -> usize
    {
        self.valid_end
    }

    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.bytes.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize
    {
        self.bytes.capacity()
    }

    #[inline]
    pub fn remaining_capacity(&self) -> usize
    {
        self.bytes.capacity().saturating_sub(self.bytes.len())
    }

    #[inline]
    pub fn clear(&mut self)
    {
        self.bytes.clear();
        self.valid_end = 0;
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8]
    {
        &self.bytes
    }

    #[inline]
    pub fn as_valid_bytes(&self) -> &[u8]
    {
        &self.bytes[.. self.valid_end]
    }

    /// Valid UTF-8 prefix as a string.
    #[inline]
    pub fn as_str(&self) -> &str
    {
        // SAFETY: valid_end is maintained from UTF-8 validation.
        unsafe { std::str::from_utf8_unchecked(&self.bytes[.. self.valid_end]) }
    }

    /// Full bytes as UTF-8 if complete/valid.
    pub fn as_full_str(&self) -> Option<&str>
    {
        std::str::from_utf8(&self.bytes).ok()
    }

    /// Appends bytes and returns the inserted span.
    ///
    /// This never panics due to partial UTF-8.
    pub fn append(&mut self, bytes: &[u8]) -> TextSpan
    {
        let start = self.bytes.len();
        self.bytes.extend_from_slice(bytes);
        let end = self.bytes.len();
        self.refresh_valid_end();
        TextSpan::new(start, end)
    }

    /// Appends a UTF-8 string and returns the inserted span.
    pub fn append_str(&mut self, s: &str) -> TextSpan
    {
        self.append(s.as_bytes())
    }

    /// Appends a single UTF-8 codepoint and returns the inserted span.
    pub fn append_char(&mut self, ch: char) -> TextSpan
    {
        let mut tmp = [0u8; 4];
        let encoded = ch.encode_utf8(&mut tmp);
        self.append(encoded.as_bytes())
    }

    /// Checks whether `idx` is a char boundary in the valid UTF-8 prefix.
    pub fn is_char_boundary(&self, idx: usize) -> bool
    {
        idx <= self.valid_end && self.as_str().is_char_boundary(idx)
    }

    /// Returns a raw byte slice for a span.
    pub fn slice_bytes(&self, span: TextSpan) -> Option<&[u8]>
    {
        if span.start <= span.end && span.end <= self.bytes.len()
        {
            Some(&self.bytes[span.start .. span.end])
        }
        else
        {
            None
        }
    }

    /// Returns a UTF-8 string slice for a span if boundaries are valid.
    pub fn slice_str(&self, span: TextSpan) -> Option<&str>
    {
        if span.end > self.valid_end
            || !self.is_char_boundary(span.start)
            || !self.is_char_boundary(span.end)
        {
            return None;
        }

        Some(&self.as_str()[span.start .. span.end])
    }

    /// Returns UTF-8 codepoint at a known-safe boundary.
    pub fn char_at(&self, idx: usize) -> Option<char>
    {
        if idx >= self.valid_end
        {
            return None;
        }
        utf8_char_on(self.as_valid_bytes(), idx)
    }

    pub fn codepoint_len_at(&self, idx: usize) -> Option<usize>
    {
        if idx >= self.valid_end
        {
            return None;
        }
        utf8_codepoint_len(self.as_valid_bytes(), idx)
    }

    pub fn next_boundary_after(&self, idx: usize) -> Option<usize>
    {
        if idx >= self.valid_end
        {
            return None;
        }
        utf8_byte_udx_after(self.as_valid_bytes(), idx)
    }

    /// Valid UTF-8 iterator from start of valid prefix.
    pub fn iter_chars(&self) -> Utf8Iter<'_>
    {
        Utf8Iter::new(self.as_valid_bytes())
    }

    /// Valid UTF-8 iterator from a byte offset.
    pub fn iter_chars_from(&self, start: usize) -> Option<Utf8Iter<'_>>
    {
        if !self.is_char_boundary(start)
        {
            return None;
        }
        Some(Utf8Iter::new(&self.as_valid_bytes()[start ..]))
    }

    /// Iterates until first invalid/partial sequence over all bytes.
    pub fn iter_part_chars(&self) -> Utf8PartIter<'_>
    {
        Utf8PartIter::new(&self.bytes)
    }

    /// Returns each valid codepoint as `U+XXXX` text.
    pub fn codepoint_debug_unicode(&self) -> Vec<String>
    {
        self.iter_chars().map(|ch| ch.fmt_unicode()).collect()
    }

    pub fn cursor(&self) -> Utf8Cursor<'_>
    {
        Utf8Cursor::new(self)
    }

    pub fn cursor_at(&self, start: usize) -> Option<Utf8Cursor<'_>>
    {
        Utf8Cursor::at(self, start)
    }

    fn refresh_valid_end(&mut self)
    {
        self.valid_end = match std::str::from_utf8(&self.bytes)
        {
            Ok(_) => self.bytes.len(),
            Err(err) => err.valid_up_to(),
        };
    }
}

pub struct Utf8Cursor<'a>
{
    buf: &'a Utf8Buffer,
    pos: usize,
}

impl<'a> Utf8Cursor<'a>
{
    pub fn new(buf: &'a Utf8Buffer) -> Self
    {
        Self { buf, pos: 0 }
    }

    pub fn at(buf: &'a Utf8Buffer, start: usize) -> Option<Self>
    {
        if !buf.is_char_boundary(start)
        {
            return None;
        }
        Some(Self { buf, pos: start })
    }

    pub fn pos(&self) -> usize
    {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) -> bool
    {
        if self.buf.is_char_boundary(pos)
        {
            self.pos = pos;
            true
        }
        else
        {
            false
        }
    }

    pub fn peek(&self) -> Option<char>
    {
        self.buf.char_at(self.pos)
    }

    pub fn next_char(&mut self) -> Option<(usize, char)>
    {
        let start = self.pos;
        let ch = self.buf.char_at(start)?;
        self.pos += ch.len_utf8();
        Some((start, ch))
    }

    pub fn span_to_pos(&self, start: usize) -> Option<TextSpan>
    {
        if self.buf.is_char_boundary(start) && start <= self.pos
        {
            Some(TextSpan::new(start, self.pos))
        }
        else
        {
            None
        }
    }
}

impl Index<usize> for Utf8Buffer
{
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output
    {
        &self.bytes[index]
    }
}

impl Index<Range<usize>> for Utf8Buffer
{
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output
    {
        &self.bytes[index]
    }
}

impl Index<RangeTo<usize>> for Utf8Buffer
{
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output
    {
        &self.bytes[index]
    }
}

impl Index<RangeFrom<usize>> for Utf8Buffer
{
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output
    {
        &self.bytes[index]
    }
}

impl Index<RangeInclusive<usize>> for Utf8Buffer
{
    type Output = [u8];

    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output
    {
        &self.bytes[index]
    }
}

impl Index<RangeFull> for Utf8Buffer
{
    type Output = [u8];

    fn index(&self, index: RangeFull) -> &Self::Output
    {
        &self.bytes[index]
    }
}

#[cfg(test)]
mod tests
{
    use super::{TextSpan, Utf8Buffer};

    #[test]
    fn append_partial_then_complete_utf8()
    {
        let mut buf = Utf8Buffer::new(8);
        let span_a = buf.append("A".as_bytes());
        assert_eq!(span_a, TextSpan::new(0, 1));
        assert_eq!(buf.as_str(), "A");
        assert_eq!(buf.valid_len(), 1);

        let span_partial = buf.append(&[0xE2, 0x82]); // partial for €
        assert_eq!(span_partial, TextSpan::new(1, 3));
        assert_eq!(buf.as_str(), "A");
        assert_eq!(buf.valid_len(), 1);

        let span_end = buf.append(&[0xAC]); // completes €
        assert_eq!(span_end, TextSpan::new(3, 4));
        assert_eq!(buf.as_str(), "A€");
        assert_eq!(buf.valid_len(), 4);
    }

    #[test]
    fn cursor_walks_char_boundaries()
    {
        let mut buf = Utf8Buffer::default();
        buf.append_str("a€z");

        let mut c = buf.cursor();
        assert_eq!(c.next_char(), Some((0, 'a')));
        assert_eq!(c.next_char(), Some((1, '€')));
        assert_eq!(c.next_char(), Some((4, 'z')));
        assert_eq!(c.next_char(), None);
    }

    #[test]
    fn safe_string_slice_by_span()
    {
        let mut buf = Utf8Buffer::default();
        buf.append_str("hello");
        let span = TextSpan::new(1, 4);
        assert_eq!(buf.slice_str(span), Some("ell"));
        assert_eq!(buf.slice_bytes(span), Some("ell".as_bytes()));
    }

    #[test]
    fn raw_indexing_available()
    {
        let mut buf = Utf8Buffer::default();
        buf.append_str("abc");
        assert_eq!(buf[0], b'a');
        assert_eq!(&buf[1 .. 3], b"bc");
    }
}
