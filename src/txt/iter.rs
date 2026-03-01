pub use iter_char::Utf8Iter;
pub use iter_const::ConstUtf8Iter;
pub use iter_part::Utf8PartIter;

use crate::txt::utf8::utf8_char_on;

mod iter_char
{
    #[derive(Debug, Clone)]
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

        pub fn with_pos(bytes: &'a [u8], pos: usize) -> Self
        {
            Self { bytes, pos }
        }

        pub fn pos(&self) -> usize
        {
            self.pos
        }
    }

    impl<'a> Iterator for Utf8Iter<'a>
    {
        type Item = char;

        fn next(&mut self) -> Option<Self::Item>
        {
            let ch = super::utf8_char_on(self.bytes, self.pos)?;
            self.pos += ch.len_utf8();
            Some(ch)
        }
    }
}

mod iter_const
{
    #[derive(Debug, Clone, Copy)]
    pub struct ConstUtf8Iter<'a>
    {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> ConstUtf8Iter<'a>
    {
        pub fn new(bytes: &'a [u8]) -> Self
        {
            Self { bytes, pos: 0 }
        }

        pub fn next(&mut self) -> Option<char>
        {
            if let Some(ch) = super::utf8_char_on(self.bytes, self.pos)
            {
                self.pos += ch.len_utf8();
                Some(ch)
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
    /// Iterates valid UTF-8 chars and stops at the first invalid/partial
    /// sequence.
    #[derive(Debug, Clone)]
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

        pub fn pos(&self) -> usize
        {
            self.pos
        }

        pub fn remainder(&self) -> &'a [u8]
        {
            &self.bytes[self.pos ..]
        }
    }

    impl<'a> Iterator for Utf8PartIter<'a>
    {
        type Item = char;

        fn next(&mut self) -> Option<Self::Item>
        {
            let ch = super::utf8_char_on(self.bytes, self.pos)?;
            self.pos += ch.len_utf8();
            Some(ch)
        }
    }
}
