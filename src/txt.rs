use std::fmt;

/// Returns the byte index of the codepoint *after* the one provided.
pub const fn utf8_byte_udx_after(buffer: &[u8], udx: usize) -> Option<usize>
{
    if let Some(ch) = utf8_char_on(buffer, udx)
    {
        let stride = ch.len_utf8();
        let start = udx + stride - 1;
        return Some(udx + start);
    }
    None
}

/// Returns the number of bytes of the first char in the buffer. Retrns none if
/// char is invalid, and 1-4 for valid codepoints. If udx is none, index of 0 is
/// assumed.
pub const fn utf8_codepoint_len(buffer: &[u8]) -> Option<usize>
{
    const UTF8_MASKS: [[u8; 4]; 4] = [
        [0b1000_0000, 0, 0, 0b1111_1111],
        [0b1110_0000, 0b1100_0000, 1, 0b0001_1111],
        [0b1111_0000, 0b1110_0000, 2, 0b0000_1111],
        [0b1111_1000, 0b1111_0000, 3, 0b0000_0111],
    ];

    let udx = 0;
    let byte0 = if udx < buffer.len() { buffer[udx] } else { 0 };

    let mut udx_checker = 0;
    while udx_checker < UTF8_MASKS.len()
    {
        let [mask, valid_mask, additional_len, strip] = UTF8_MASKS[udx_checker];
        udx_checker += 1;

        if udx + additional_len as usize >= buffer.len()
        {
            return None;
        }

        let len = buffer.len();
        let offsets =
            [len.checked_sub(1), len.checked_sub(2), len.checked_sub(3)];
        let offsets2 = [0; 3];

        let byte1 = match buffer.len().checked_sub(1)
        {
            Some(_) => 0,
            Some(offset) if udx < offset => buffer[udx + 1],
            None => 0,
        };

        if byte0 & mask == valid_mask
        {
            let mut bytes = [byte0, 0, 0, 0];
            let mut i = 1;
            while let Some(boundary) = buffer.len().checked_sub(i)
                && i < bytes.len()
                && udx < boundary
            {
                bytes[i] = buffer[udx + i];
                i += 1;
            }

            // let bytes = [byte0, byte1, byte2, byte3];
            let strip_masks = [strip, 0b0011_1111, 0b0011_1111, 0b0011_1111];
            let mut codepoint: u32 = 0;

            let mut udx_i = 0usize;
            while udx_i < additional_len as usize + 1
            {
                let shift = 6 * (additional_len - udx_i as u8);
                let stripped_byte = bytes[udx_i] & strip_masks[udx_i];
                codepoint |= (stripped_byte as u32) << (shift as u32);
                udx_i += 1;
            }

            // TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above
            // TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above// TODO: return how many bytes the char is, even if it is fragmented
            // TODO: that means remove some checks from above
            let char = std::char::from_u32(codepoint);
            return char.map(char::len_utf8);
        }
    }

    None
}

/// Returns a [char] for a specific byte index. If the byte index is not on a
/// valid codepoint boundary (specifically the start byte of that codepoint),
/// [Option::None] is returned.
pub const fn utf8_char_on(buffer: &[u8], udx: usize) -> Option<char>
{
    const UTF8_MASKS: [[u8; 4]; 4] = [
        [0b1000_0000, 0, 0, 0b1111_1111],
        [0b1110_0000, 0b1100_0000, 1, 0b0001_1111],
        [0b1111_0000, 0b1110_0000, 2, 0b0000_1111],
        [0b1111_1000, 0b1111_0000, 3, 0b0000_0111],
    ];

    let byte0 = if udx < buffer.len() { buffer[udx] } else { 0 };

    let mut udx_checker = 0;
    while udx_checker < UTF8_MASKS.len()
    {
        let [mask, valid_mask, additional_len, strip] = UTF8_MASKS[udx_checker];
        udx_checker += 1;

        if udx + additional_len as usize >= buffer.len()
        {
            return None;
        }

        let len = buffer.len();
        let offsets =
            [len.checked_sub(1), len.checked_sub(2), len.checked_sub(3)];
        let offsets2 = [0; 3];

        let byte1 = match buffer.len().checked_sub(1)
        {
            Some(_) => 0,
            Some(offset) if udx < offset => buffer[udx + 1],
            None => 0,
        };

        if byte0 & mask == valid_mask
        {
            // let byte1 =
            //     if udx < buffer.len() - 1 { buffer[udx + 1] } else { 0 };
            // let byte2 =
            //     if udx < buffer.len() - 2 { buffer[udx + 2] } else { 0 };
            // let byte3 =
            //     if udx < buffer.len() - 3 { buffer[udx + 3] } else { 0 };

            // let mut bytes = [byte0, 0, 0, 0];
            // let mut i = 1;
            // while i < bytes.len()
            // {
            //     if let Some(boundary) = buffer.len().checked_sub(i)
            //         && i < bytes.len()
            //         && udx < boundary
            //     {
            //         bytes[i] = buffer[udx + i]
            //     }
            //     else
            //     {
            //         bytes[i] = 0;
            //     }
            //     i += 1;
            // }

            let mut bytes = [byte0, 0, 0, 0];
            let mut i = 1;
            while let Some(boundary) = buffer.len().checked_sub(i)
                && i < bytes.len()
                && udx < boundary
            {
                bytes[i] = buffer[udx + i];
                i += 1;
            }

            // let bytes = [byte0, byte1, byte2, byte3];
            let strip_masks = [strip, 0b0011_1111, 0b0011_1111, 0b0011_1111];
            let mut codepoint: u32 = 0;

            let mut udx_i = 0usize;
            while udx_i < additional_len as usize + 1
            {
                let shift = 6 * (additional_len - udx_i as u8);
                let stripped_byte = bytes[udx_i] & strip_masks[udx_i];
                codepoint |= (stripped_byte as u32) << (shift as u32);
                udx_i += 1;
            }

            return std::char::from_u32(codepoint);
        }
    }

    None
}

/// Iterator for allowing looping over a string by char.
#[derive(Debug)]
pub struct Utf8Iter<'a>
{
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Utf8Iter<'a>
{
    pub fn new(bytes: &'a [u8]) -> Self
    {
        Self { bytes, pos: 0 }
    }
}

impl<'a> Iterator for Utf8Iter<'a>
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item>
    {
        let codepoint = utf8_char_on(self.bytes, self.pos)?;
        self.pos += codepoint.len_utf8();
        Some(codepoint)
    }
}

pub fn utf8_iter_chars<'buf>(txt: &'buf str) -> Utf8Iter<'buf>
{
    Utf8Iter::new(txt.as_bytes())
}

#[derive(Debug)]
pub struct ConstUtf8Iter<'a>
{
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ConstUtf8Iter<'a>
{
    pub const fn new(bytes: &'a [u8]) -> Self
    {
        Self { bytes, pos: 0 }
    }

    pub const fn next(&mut self) -> Option<char>
    {
        if let Some(codepoint) = utf8_char_on(self.bytes, self.pos)
        {
            self.pos += codepoint.len_utf8();
            Some(codepoint)
        }
        else
        {
            None
        }
    }
}

pub const fn utf8_iter_chars_const<'buf>(txt: &'buf str)
-> ConstUtf8Iter<'buf>
{
    ConstUtf8Iter::new(txt.as_bytes())
}

impl<T: fmt::Debug> ToDebug for T {}
pub trait ToDebug: fmt::Debug
{
    /// Equivalent to `format!("{:?}", thing);`
    fn to_debug(&self) -> String
    {
        format!("{self:?}")
    }

    /// Equivalent to `format!("{:#?}", thing);`
    fn to_long_debug(&self) -> String
    {
        format!("{self:#?}")
    }

    /// A debug view of a debug view (includes the outer quotes)
    fn to_debug_literal(&self) -> String
    {
        format!("{:?}", format!("{}", self.to_debug()))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_left(&self, space: usize) -> String
    {
        format!("{:<space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_right(&self, space: usize) -> String
    {
        format!("{:>space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_center(&self, space: usize) -> String
    {
        format!("{:^space$}", format!("{self:?}"))
    }
}

// TODO: This is a pretty formatter for 1-4 byte codepoints to use the U+... fmt
/// **Format ASCII & multi-byte codepoints as either their escape-code format
/// `\u{AB12}` or their Unicode codepoint `U+AB12`.**
pub trait CodepointView
{
    fn fmt_escape(self) -> String;
    fn fmt_unicode(self) -> String;
}

impl CodepointView for char
{
    fn fmt_escape(self) -> String
    {
        format!("\\u{:04X}", self as u32)
    }

    fn fmt_unicode(self) -> String
    {
        format!("U+{:04X}", self as u32)
    }
}

/// Partial iterator for allowing looping over UTF-8 codepoint fragments.
/// Terminates at end of buffer or when a partial fragment is hit. Returns the
/// size of the partial character. 0-4 bytes, 0 for end of stream, 1-4 for valid
/// codepoints that were sent fragmented.
#[derive(Debug)]
pub struct Utf8PartIter<'a>
{
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Utf8PartIter<'a>
{
    pub fn new(bytes: &'a [u8]) -> Self
    {
        Self { bytes, pos: 0 }
    }
}

impl<'a> Iterator for Utf8PartIter<'a>
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item>
    {
        let codepoint = utf8_char_on(self.bytes, self.pos)?;
        self.pos += codepoint.len_utf8();
        Some(codepoint)
    }
}

pub fn utf8_iter_chars<'buf>(txt: &'buf str) -> Utf8PartIter<'buf>
{
    Utf8PartIter::new(txt.as_bytes())
}
