/// utf8.rs — Accumulating UTF-8 input buffer + codepoint iterator.
///
/// `Utf8Buf` owns the raw bytes as they arrive.  It can be used in two modes:
///
///   - **Accumulating** (default): every byte is appended to an internal Vec.
///     The buffer is always a valid `&[u8]` slice for token deref lookback.
///
///   - **Streaming** (pass-through): bytes are forwarded without being stored.
///     Use this when you don't need lookback and want to minimise allocation.
///     Set via `Utf8Buf::streaming()`.
///
/// `Utf8Iter` borrows a `Utf8Buf` and yields `(byte_offset, char)` pairs.

// ── Utf8Buf ───────────────────────────────────────────────────────────────────

pub struct Utf8Buf
{
    inner: Box<dyn Iterator<Item = u8>>,
    buf: Vec<u8>,
    accumulate: bool,
}

impl Utf8Buf
{
    /// Accumulating mode — every consumed byte is appended to the internal buffer.
    pub fn new(inner: Box<dyn Iterator<Item = u8>>) -> Self
    {
        Self { inner, buf: Vec::new(), accumulate: true }
    }

    /// Streaming mode — bytes are forwarded without being stored.
    pub fn streaming(inner: Box<dyn Iterator<Item = u8>>) -> Self
    {
        Self { inner, buf: Vec::new(), accumulate: false }
    }

    /// Borrow the accumulated bytes for token deref / lookback.
    /// In streaming mode this will be empty.
    pub fn as_bytes(&self) -> &[u8]
    {
        &self.buf
    }

    /// Pull the next raw byte, accumulating if configured.
    fn next_byte(&mut self) -> Option<u8>
    {
        let b = self.inner.next()?;
        if self.accumulate
        {
            self.buf.push(b);
        }
        Some(b)
    }
}

// ── Utf8Iter ──────────────────────────────────────────────────────────────────
// Borrows Utf8Buf mutably and yields (byte_offset, char) pairs.
// The offset is relative to the start of the Utf8Buf's byte stream.

pub struct Utf8Iter<'a>
{
    src: &'a mut Utf8Buf,
    offset: usize,
    partial: bool,
}

impl<'a> Utf8Iter<'a>
{
    pub fn new(src: &'a mut Utf8Buf) -> Self
    {
        Self { src, offset: 0, partial: false }
    }

    pub fn partial(&self) -> bool
    {
        self.partial
    }
}

impl<'a> Iterator for Utf8Iter<'a>
{
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item>
    {
        self.partial = false;
        let b0 = self.src.next_byte()?;
        let start = self.offset;
        self.offset += 1;

        if b0 & 0x80 == 0
        {
            return Some((start, b0 as char));
        }

        let (width, mut cp) = if b0 & 0xF8 == 0xF0
        {
            (4, (b0 & 0x07) as u32)
        }
        else if b0 & 0xF0 == 0xE0
        {
            (3, (b0 & 0x0F) as u32)
        }
        else if b0 & 0xE0 == 0xC0
        {
            (2, (b0 & 0x1F) as u32)
        }
        else
        {
            return Some((start, char::REPLACEMENT_CHARACTER));
        };

        for _ in 1 .. width
        {
            match self.src.next_byte()
            {
                Some(b) if b & 0xC0 == 0x80 =>
                {
                    cp = (cp << 6) | (b & 0x3F) as u32;
                    self.offset += 1;
                }
                Some(_) =>
                {
                    self.offset += 1;
                    return Some((start, char::REPLACEMENT_CHARACTER));
                }
                None =>
                {
                    self.partial = true;
                    return None;
                }
            }
        }

        Some((start, char::from_u32(cp).unwrap_or(char::REPLACEMENT_CHARACTER)))
    }
}

// ── convenience ───────────────────────────────────────────────────────────────

pub fn buf_from_slice(bytes: &[u8]) -> Utf8Buf
{
    Utf8Buf::new(Box::new(bytes.to_vec().into_iter()))
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn collect(src: &[u8]) -> Vec<(usize, char)>
    {
        let mut buf = buf_from_slice(src);
        Utf8Iter::new(&mut buf).collect()
    }

    #[test]
    fn ascii()
    {
        assert_eq!(collect(b"abc"), [(0, 'a'), (1, 'b'), (2, 'c')]);
    }

    #[test]
    fn multibyte()
    {
        assert_eq!(collect("éz".as_bytes()), [(0, 'é'), (2, 'z')]);
    }

    #[test]
    fn accumulates()
    {
        let mut buf = buf_from_slice(b"hi");
        let _: Vec<_> = Utf8Iter::new(&mut buf).collect();
        assert_eq!(buf.as_bytes(), b"hi");
    }

    #[test]
    fn streaming_does_not_accumulate()
    {
        let mut buf = Utf8Buf::streaming(Box::new(b"hi".iter().copied()));
        let _: Vec<_> = Utf8Iter::new(&mut buf).collect();
        assert_eq!(buf.as_bytes(), b"");
    }
}
