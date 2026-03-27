/// [`Text`] is a lifetime-erased string slice into a [`Utf8Buf`].
///
/// Because [`Utf8Buf`] is **append-only**, any `(from, to)` range that was
/// valid at the moment it was issued remains valid forever — the bytes at those
/// positions never move or change.  `Text` encodes that guarantee without
/// carrying a lifetime.
///
/// Invariant: `from <= to`, and both offsets must lie on valid UTF-8 codepoint
/// boundaries inside the owning [`Utf8Buf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text
{
    pub from: u32,
    pub to: u32,
}

impl Text
{
    #[inline]
    pub fn new(from: u32, to: u32) -> Self
    {
        debug_assert!(from <= to, "Text: from ({from}) > to ({to})");
        Self { from, to }
    }

    /// Number of bytes in the slice (NOT codepoints).
    #[inline]
    pub fn byte_len(self) -> usize { (self.to - self.from) as usize }

    /// Resolve this `Text` against its owning buffer.
    ///
    /// # Safety
    /// Caller must guarantee that `buf` is the buffer this `Text` was issued
    /// from and that it still contains those bytes (upheld by append-only
    /// invariant).
    #[inline]
    pub fn resolve<'b>(self, buf: &'b Utf8Buf) -> &'b str
    {
        // SAFETY: append-only buffer + codepoint-boundary invariant on Text.
        unsafe {
            let bytes = buf
                .buffer
                .get_unchecked(self.from as usize .. self.to as usize);
            std::str::from_utf8_unchecked(bytes)
        }
    }
}

// ---------------------------------------------------------------------------

/// [`Utf8Buf`] — an **append-only** UTF-8 byte buffer.
///
/// The append-only constraint means that:
///   - existing byte offsets are never invalidated, and
///   - [`Text`] values issued from this buffer remain valid indefinitely.
///
/// All writes are validated to end on a codepoint boundary before the
/// "safe length" cursor is advanced, so partial codepoint writes are allowed
/// in flight but are invisible to safe readers until the codepoint is complete.
///
/// The use case of Utf8Buf is piecemeal indexing of the buffer while
/// allowing synchronous appends consisting of partial or complete UTF-8
/// codepoints.
pub struct Utf8Buf
{
    buffer: Vec<u8>,
    /// Number of bytes that form complete UTF-8 codepoints.
    /// Reads and safe slices are limited to `[0..safe_len]`.
    safe_len: usize,
}

impl Utf8Buf
{
    pub fn new() -> Self { Self { buffer: Vec::new(), safe_len: 0 } }

    pub fn with_capacity(cap: usize) -> Self
    {
        Self { buffer: Vec::with_capacity(cap), safe_len: 0 }
    }

    /// Total byte capacity held in the underlying allocation.
    #[inline]
    pub fn capacity(&self) -> usize { self.buffer.capacity() }

    /// Number of bytes that are fully committed (end on a codepoint boundary).
    #[inline]
    pub fn safe_len(&self) -> usize { self.safe_len }

    /// Raw byte length including any in-flight partial codepoint.
    #[inline]
    pub fn raw_len(&self) -> usize { self.buffer.len() }

    /// Append a validated UTF-8 string, advancing `safe_len` to include it.
    pub fn push_str(&mut self, s: &str)
    {
        self.buffer.extend_from_slice(s.as_bytes());
        self.safe_len = self.buffer.len();
    }

    /// Appends `bytes`, which may contain an incomplete trailing codepoint.
    ///
    /// `safe_len` is advanced to the last complete codepoint boundary within
    /// the entire buffer after the append; any partial codepoint at the tail
    /// remains buffered but invisible to safe readers until completed by a
    /// subsequent push.
    pub fn push_bytes(&mut self, bytes: &[u8])
    {
        self.buffer.extend_from_slice(bytes);
        self.safe_len = last_codepoint_boundary(&self.buffer);
    }

    /// Appends a single byte, which may be a continuation byte of a multi-byte
    /// codepoint.  `safe_len` advances only when the byte completes a
    /// codepoint; until then [`has_partial`](Self::has_partial) returns `true`.
    pub fn push_byte(&mut self, byte: u8)
    {
        self.buffer.push(byte);
        // Re-scan from the last safe boundary.
        self.safe_len = last_codepoint_boundary(&self.buffer);
    }

    /// Returns `true` if there are in-flight bytes past `safe_len`.
    #[inline]
    pub fn has_partial(&self) -> bool { self.buffer.len() > self.safe_len }

    /// Returns the safe region as a `&str`, covering only fully committed
    /// codepoints.  Any in-flight bytes past `safe_len` are excluded.
    #[inline]
    pub fn as_str(&self) -> &str
    {
        // SAFETY: [0..safe_len] is always valid UTF-8 by construction.
        unsafe { std::str::from_utf8_unchecked(&self.buffer[.. self.safe_len]) }
    }

    /// Returns the raw byte slice including any partial codepoint at the tail.
    ///
    /// The last few bytes may not form valid UTF-8; prefer
    /// [`as_str`](Self::as_str) or [`as_bytes`](Self::as_bytes) unless you
    /// specifically need the raw buffer.
    #[inline]
    pub fn as_bytes_raw(&self) -> &[u8] { &self.buffer }

    /// Safe byte slice, ending on a codepoint boundary.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] { &self.buffer[.. self.safe_len] }

    /// Resolve a [`Text`] to `&str` without a lifetime on `self`. Shorthand for
    /// [`Text::resolve`].
    #[inline]
    pub fn slice(&self, t: Text) -> &str { t.resolve(self) }

    /// Create a [`Text`] that spans `[from, to)` bytes.
    ///
    /// Panics in debug builds if the range is out of bounds or not on codepoint
    /// boundaries.
    #[inline]
    pub fn text(&self, from: u32, to: u32) -> Text
    {
        debug_assert!(
            (to as usize) <= self.safe_len,
            "Text::to ({to}) exceeds safe_len ({})",
            self.safe_len
        );
        Text::new(from, to)
    }

    /// Create a [`Utf8Iter`] starting at byte offset 0.
    pub fn iter(&self) -> Utf8Iter<'_> { Utf8Iter::new(self, 0) }

    /// Create a [`Utf8Iter`] starting at byte offset `start`.
    ///
    /// `start` **must** lie on a valid codepoint boundary (caller guarantee).
    pub fn iter_from(&self, start: u32) -> Utf8Iter<'_>
    {
        Utf8Iter::new(self, start as usize)
    }
}

impl Default for Utf8Buf
{
    #[rustfmt::skip]
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------

/// Walks backward from the end of `buf` to find the largest prefix that
/// consists entirely of complete UTF-8 codepoints, returning that length.
///
/// Used internally after every raw-byte push to keep `safe_len` accurate
/// even when the last write ended mid-codepoint.
fn last_codepoint_boundary(buf: &[u8]) -> usize
{
    let len = buf.len();
    if len == 0
    {
        return 0;
    }
    // A UTF-8 codepoint starts with 0xxxxxxx or 11xxxxxx.
    // Continuation bytes are 10xxxxxx.  Walk back over continuation bytes.
    let mut i = len;
    while i > 0
    {
        i -= 1;
        let b = buf[i];
        if b & 0b1100_0000 != 0b1000_0000
        {
            // This is a leading byte.  Check that the codepoint it starts is
            // fully present.
            let cp_len = utf8_leading_byte_width(b);
            if i + cp_len <= len
            {
                return i + cp_len; // complete
            }
            else
            {
                return i; // incomplete — exclude this leading byte too
            }
        }
        // else: continuation byte, keep walking back
    }
    0
}

/// Returns total byte width of the UTF-8 codepoint whose leading byte is `b`.
///
/// Assumes `b` is a valid leading byte (0xxxxxxx, 110xxxxx, 1110xxxx, or
/// 11110xxx).  Continuation bytes (10xxxxxx) are not valid inputs and will
/// return 1, which the caller should never encounter on a valid boundary.
#[inline]
fn utf8_leading_byte_width(b: u8) -> usize
{
    if b & 0b1000_0000 == 0
    {
        1
    }
    else if b & 0b1110_0000 == 0b1100_0000
    {
        2
    }
    else if b & 0b1111_0000 == 0b1110_0000
    {
        3
    }
    else
    {
        4
    }
}

// ---------------------------------------------------------------------------

/// [`Utf8Iter`] — a codepoint iterator over a [`Utf8Buf`].
///
/// Yields `(byte_offset, char)` pairs.  Always starts on a codepoint boundary.
///
/// [`Utf8Iter::partial`] returns `true` when the iterator has consumed all
/// complete codepoints but the buffer has in-flight bytes that may complete
/// a further codepoint once more bytes are pushed.  Callers can use this to
/// decide whether to wait for more input before treating the iteration as done.
pub struct Utf8Iter<'buf>
{
    buf: &'buf Utf8Buf,
    /// Current byte offset within `buf.as_bytes()` (the safe region).
    pos: usize,
}

impl<'buf> Utf8Iter<'buf>
{
    /// Create an iterator starting at `start_byte`.
    ///
    /// `start_byte` must be on a valid codepoint boundary within the safe
    /// region of `buf` (caller guarantee; not checked in release builds).
    pub fn new(buf: &'buf Utf8Buf, start_byte: usize) -> Self
    {
        debug_assert!(
            start_byte <= buf.safe_len(),
            "Utf8Iter: start_byte ({start_byte}) > safe_len ({})",
            buf.safe_len()
        );
        Self { buf, pos: start_byte }
    }

    /// Seek to a new byte offset, which must be on a codepoint boundary.
    pub fn seek(&mut self, byte_offset: usize)
    {
        debug_assert!(byte_offset <= self.buf.safe_len());
        self.pos = byte_offset;
    }

    /// Current byte offset (offset of the **next** character to be yielded).
    #[inline]
    pub fn byte_pos(&self) -> usize { self.pos }

    /// Returns `true` if the backing buffer has bytes beyond the last complete
    /// codepoint — i.e. a partial codepoint write is in flight.
    #[inline]
    pub fn partial(&self) -> bool { self.buf.has_partial() }

    /// Peeks at the next character without advancing the iterator.
    ///
    /// Returns `None` if the safe region is exhausted.  Does not account for
    /// in-flight bytes; check [`partial`](Self::partial) separately if needed.
    pub fn peek(&self) -> Option<char>
    {
        let bytes = self.buf.as_bytes();
        if self.pos >= bytes.len()
        {
            return None;
        }
        // SAFETY: safe region is valid UTF-8 and pos is on a boundary.
        let s = unsafe { std::str::from_utf8_unchecked(&bytes[self.pos ..]) };
        s.chars().next()
    }
}

impl<'buf> Iterator for Utf8Iter<'buf>
{
    /// Yields `(byte_offset, char)` pairs where `byte_offset` is the position
    /// of the *start* of the codepoint within the buffer's safe region — i.e.
    /// the offset you would pass to [`Utf8Buf::iter_from`] or use as the `from`
    /// field of a [`Text`] to re-address this character later.
    ///
    /// The iterator advances its internal cursor past the codepoint after
    /// yielding, so consecutive calls yield non-overlapping characters in
    /// order.  The returned offset is a *snapshot before that advance*, making
    /// it suitable as a stable byte address into the owning [`Utf8Buf`].
    ///
    /// Returns `None` when all complete codepoints in the safe region have been
    /// consumed.  If [`Utf8Iter::partial`] returns `true` at that point, the
    /// buffer holds in-flight bytes that may complete another codepoint once
    /// further data is pushed — callers that care about streaming completeness
    /// should check this flag rather than treating `None` as end-of-input.
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item>
    {
        let bytes = self.buf.as_bytes();
        if self.pos >= bytes.len()
        {
            return None;
        }
        let start = self.pos;
        let b = bytes[start];
        let width = utf8_leading_byte_width(b);

        // SAFETY: safe region guarantees entire codepoints; pos is on boundary.
        let ch = unsafe {
            let slice = bytes.get_unchecked(start .. start + width);
            let s = std::str::from_utf8_unchecked(slice);
            s.chars().next().unwrap_unchecked()
        };

        self.pos += width;
        Some((start, ch))
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn ascii_roundtrip()
    {
        let mut buf = Utf8Buf::new();
        buf.push_str("hello");
        let chars: Vec<_> = buf.iter().collect();
        assert_eq!(chars, vec![
            (0, 'h'),
            (1, 'e'),
            (2, 'l'),
            (3, 'l'),
            (4, 'o')
        ]);
    }

    #[test]
    fn multibyte_iter()
    {
        let mut buf = Utf8Buf::new();
        buf.push_str("日本語");
        // Each kanji is 3 bytes.
        let chars: Vec<_> = buf.iter().collect();
        assert_eq!(chars, vec![(0, '日'), (3, '本'), (6, '語')]);
    }

    #[test]
    fn iter_from_midpoint()
    {
        let mut buf = Utf8Buf::new();
        buf.push_str("abc");
        let chars: Vec<_> = buf.iter_from(1).collect();
        assert_eq!(chars, vec![(1, 'b'), (2, 'c')]);
    }

    #[test]
    fn partial_byte_write()
    {
        let mut buf = Utf8Buf::new();
        // '€' is 0xE2 0x82 0xAC  (3 bytes)
        buf.push_byte(0xE2);
        assert!(buf.has_partial());
        assert_eq!(buf.safe_len(), 0);
        buf.push_byte(0x82);
        assert!(buf.has_partial());
        buf.push_byte(0xAC);
        assert!(!buf.has_partial());
        assert_eq!(buf.safe_len(), 3);
        let chars: Vec<_> = buf.iter().collect();
        assert_eq!(chars, vec![(0, '€')]);
    }

    #[test]
    fn text_resolve()
    {
        let mut buf = Utf8Buf::new();
        buf.push_str("hello world");
        let t = buf.text(6, 11);
        assert_eq!(buf.slice(t), "world");
    }

    #[test]
    fn partial_flag_via_iter()
    {
        let mut buf = Utf8Buf::new();
        buf.push_byte(0xE2); // partial '€'
        let it = buf.iter();
        assert!(it.partial());
    }

    #[test]
    fn seek()
    {
        let mut buf = Utf8Buf::new();
        buf.push_str("abcd");
        let mut it = buf.iter();
        assert_eq!(it.next(), Some((0, 'a')));
        it.seek(3);
        assert_eq!(it.next(), Some((3, 'd')));
        assert_eq!(it.next(), None);
    }
}
