use crate::txt::iter::{ConstUtf8Iter, Utf8Iter, Utf8PartIter};

#[inline]
const fn utf8_expected_len(byte0: u8) -> Option<usize>
{
    if byte0 <= 0x7F
    {
        Some(1)
    }
    else if byte0 & 0b1110_0000 == 0b1100_0000
    {
        Some(2)
    }
    else if byte0 & 0b1111_0000 == 0b1110_0000
    {
        Some(3)
    }
    else if byte0 & 0b1111_1000 == 0b1111_0000
    {
        Some(4)
    }
    else
    {
        None
    }
}

#[inline]
const fn is_continuation(byte: u8) -> bool
{
    byte & 0b1100_0000 == 0b1000_0000
}

/// Returns the number of bytes in the codepoint starting at `udx`.
/// Returns `None` if:
/// 1. `udx` is out of range
/// 2. `udx` is not on a codepoint start
/// 3. bytes are incomplete/invalid UTF-8 for that codepoint
pub fn utf8_codepoint_len(buffer: &[u8], udx: usize) -> Option<usize>
{
    if udx >= buffer.len()
    {
        return None;
    }

    let byte0 = buffer[udx];
    let len = utf8_expected_len(byte0)?;

    if udx + len > buffer.len()
    {
        return None;
    }

    let mut i = 1usize;
    while i < len
    {
        if !is_continuation(buffer[udx + i])
        {
            return None;
        }
        i += 1;
    }

    Some(len)
}

/// Returns the UTF-8 char starting at `udx`.
pub fn utf8_char_on(buffer: &[u8], udx: usize) -> Option<char>
{
    let len = utf8_codepoint_len(buffer, udx)?;
    let slice = &buffer[udx .. udx + len];
    match std::str::from_utf8(slice)
    {
        Ok(s) => s.chars().next(),
        Err(_) => None,
    }
}

/// Returns char at `udx`, or an error with the furthest valid index.
pub fn utf8_codepoint_at(buffer: &[u8], udx: usize) -> Result<char, usize>
{
    utf8_char_on(buffer, udx).ok_or(udx)
}

/// Returns the index immediately after the codepoint at `udx`.
pub fn utf8_byte_udx_after(buffer: &[u8], udx: usize) -> Option<usize>
{
    let len = utf8_codepoint_len(buffer, udx)?;
    Some(udx + len)
}

pub fn utf8_iter_chars(txt: &str) -> Utf8Iter<'_>
{
    Utf8Iter::new(txt.as_bytes())
}

pub fn utf8_iter_chars_const(txt: &str) -> ConstUtf8Iter<'_>
{
    ConstUtf8Iter::new(txt.as_bytes())
}

/// Iterates valid UTF-8 chars until a partial/invalid sequence is hit.
pub fn utf8_iter_part_chars(txt: &str) -> Utf8PartIter<'_>
{
    Utf8PartIter::new(txt.as_bytes())
}
