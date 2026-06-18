pub use iter_char::Utf8Iter;
pub use iter_const::ConstUtf8Iter;
pub use iter_part::Utf8PartIter;

use crate::txt::utf8::utf8_char_on;

mod iter_char
{
    /// Iterator for allowing looping over a string by char.
    #[derive(Debug)]
    pub struct Utf8Iter<'a>
    {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> Utf8Iter<'a>
    {
        pub fn new(bytes: &'a [u8]) -> Self { Self { bytes, pos: 0 } }
    }

    impl<'a> Iterator for Utf8Iter<'a>
    {
        type Item = char;

        fn next(&mut self) -> Option<Self::Item>
        {
            let codepoint = super::utf8_char_on(self.bytes, self.pos)?;
            self.pos += codepoint.len_utf8();
            Some(codepoint)
        }
    }
}

mod iter_const
{
    #[derive(Debug)]
    pub struct ConstUtf8Iter<'a>
    {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> ConstUtf8Iter<'a>
    {
        pub const fn new(bytes: &'a [u8]) -> Self { Self { bytes, pos: 0 } }

        pub const fn next(&mut self) -> Option<char>
        {
            if let Some(codepoint) = super::utf8_char_on(self.bytes, self.pos)
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
}

mod iter_part
{
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
        pub fn new(bytes: &'a [u8]) -> Self { Self { bytes, pos: 0 } }
    }

    impl<'a> Iterator for Utf8PartIter<'a>
    {
        type Item = char;

        fn next(&mut self) -> Option<Self::Item>
        {
            let codepoint = super::utf8_char_on(self.bytes, self.pos)?;
            self.pos += codepoint.len_utf8();
            Some(codepoint)
        }
    }
}
