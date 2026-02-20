use std::fs::File;
use std::io::{self, Read};

// ============================================================
//  UTF‑8 STREAMING CODEPOINT ITERATOR
// ============================================================

pub struct Utf8Codepoints<R: Read>
{
    reader: R,
    buf: [u8; 1],
    codepoint: u32,
    needed: usize,
}

impl<R: Read> Utf8Codepoints<R>
{
    pub fn new(reader: R) -> Self
    {
        Self { reader, buf: [0], codepoint: 0, needed: 0 }
    }
}

impl<R: Read> Iterator for Utf8Codepoints<R>
{
    type Item = io::Result<char>;

    fn next(&mut self) -> Option<Self::Item>
    {
        loop
        {
            let n = match self.reader.read(&mut self.buf)
            {
                Ok(0) => return None,
                Ok(num) => num,
                Err(e) => return Some(Err(e)),
            };

            let b = self.buf[0];

            if self.needed == 0
            {
                if b < 0x80
                {
                    return Some(Ok(b as char));
                }
                else if b & 0b1110_0000 == 0b1100_0000
                {
                    self.codepoint = (b & 0b0001_1111) as u32;
                    self.needed = 1;
                }
                else if b & 0b1111_0000 == 0b1110_0000
                {
                    self.codepoint = (b & 0b0000_1111) as u32;
                    self.needed = 2;
                }
                else if b & 0b1111_1000 == 0b1111_0000
                {
                    self.codepoint = (b & 0b0000_0111) as u32;
                    self.needed = 3;
                }
                else
                {
                    continue;
                }
            }
            else
            {
                if b & 0b1100_0000 != 0b1000_0000
                {
                    self.needed = 0;
                    continue;
                }

                self.codepoint =
                    (self.codepoint << 6) | (b & 0b0011_1111) as u32;
                self.needed -= 1;

                if self.needed == 0
                    && let Some(ch) = char::from_u32(self.codepoint)
                {
                    return Some(Ok(ch));
                }
            }
        }
    }
}

// Entry points
pub fn codepoints_from_reader<R: Read>(reader: R) -> Utf8Codepoints<R>
{
    Utf8Codepoints::new(reader)
}

pub fn codepoints_from_file(path: &str) -> io::Result<Utf8Codepoints<File>>
{
    Ok(Utf8Codepoints::new(File::open(path)?))
}

pub fn codepoints_from_str(s: &str) -> Utf8Codepoints<io::Cursor<&[u8]>>
{
    Utf8Codepoints::new(io::Cursor::new(s.as_bytes()))
}
