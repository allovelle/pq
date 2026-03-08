/// utf8.rs — Codepoint-streaming iterator over a byte source.
///
/// `Utf8Iter` takes anything that yields `u8` and produces `(byte_offset, char)`
/// pairs.  It handles multi-byte codepoints internally, accumulating continuation
/// bytes before emitting a char.
///
/// `partial()` returns true when the stream ended mid-codepoint (truncated input).

pub struct Utf8Iter<I: Iterator<Item = u8>>
{
    inner: I,
    offset: usize,
    partial: bool,
}

impl<I: Iterator<Item = u8>> Utf8Iter<I>
{
    pub fn new(inner: I) -> Self
    {
        Self { inner, offset: 0, partial: false }
    }

    /// True if the stream ended in the middle of a multi-byte codepoint.
    pub fn partial(&self) -> bool
    {
        self.partial
    }
}

impl<I: Iterator<Item = u8>> Iterator for Utf8Iter<I>
{
    /// (byte_offset_of_codepoint_start, char)
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item>
    {
        self.partial = false;

        let b0 = self.inner.next()?;
        let start = self.offset;
        self.offset += 1;

        // ASCII fast-path
        if b0 & 0x80 == 0
        {
            return Some((start, b0 as char));
        }

        // Determine expected byte count from the leading byte.
        let (width, mut codepoint) = if b0 & 0xF8 == 0xF0
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
            // Unexpected continuation byte or invalid — emit replacement.
            return Some((start, char::REPLACEMENT_CHARACTER));
        };

        // Consume `width - 1` continuation bytes.
        for _ in 1 .. width
        {
            match self.inner.next()
            {
                Some(b) if b & 0xC0 == 0x80 =>
                {
                    codepoint = (codepoint << 6) | (b & 0x3F) as u32;
                    self.offset += 1;
                }
                Some(_) =>
                {
                    // Not a continuation byte — invalid sequence.
                    self.offset += 1;
                    return Some((start, char::REPLACEMENT_CHARACTER));
                }
                None =>
                {
                    // Stream ended mid-codepoint.
                    self.partial = true;
                    return None;
                }
            }
        }

        let ch =
            char::from_u32(codepoint).unwrap_or(char::REPLACEMENT_CHARACTER);
        Some((start, ch))
    }
}

/// Wrap a byte slice into a `Utf8Iter`.
pub fn from_slice(bytes: &[u8]) -> Utf8Iter<impl Iterator<Item = u8> + '_>
{
    Utf8Iter::new(bytes.iter().copied())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn ascii()
    {
        let v: Vec<_> = from_slice(b"abc").collect();
        assert_eq!(v, [(0, 'a'), (1, 'b'), (2, 'c')]);
    }

    #[test]
    fn multibyte()
    {
        let v: Vec<_> = from_slice("éz".as_bytes()).collect();
        assert_eq!(v, [(0, 'é'), (2, 'z')]);
    }

    #[test]
    fn partial_truncated()
    {
        let mut it = from_slice(&[0xC3]);
        assert!(it.next().is_none());
        assert!(it.partial());
    }

    #[test]
    fn four_byte_codepoint()
    {
        let v: Vec<_> = from_slice("😀".as_bytes()).collect();
        assert_eq!(v, [(0, '😀')]);
    }
}
