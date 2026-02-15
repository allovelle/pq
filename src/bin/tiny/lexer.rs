use std::fs::File;
use std::io::{self, Read};

use crate::codepoints::Utf8Codepoints;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind
{
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    True,
    False,
    Null,
    Number,
    String,
    Unknown,
}

pub struct JsonTokens<R: Read>
{
    chars: Utf8Codepoints<R>,
    buf: Option<char>,
    byte_offset: usize, // running byte offset
}

impl<R: Read> JsonTokens<R>
{
    pub fn new(reader: R) -> Self
    {
        Self { chars: Utf8Codepoints::new(reader), buf: None, byte_offset: 0 }
    }

    fn next_char(&mut self) -> Option<io::Result<char>>
    {
        if let Some(c) = self.buf.take()
        {
            return Some(Ok(c));
        }
        self.chars.next()
    }

    fn unread(&mut self, c: char)
    {
        self.buf = Some(c);
    }
}

impl<R: Read> Iterator for JsonTokens<R>
{
    type Item = io::Result<usize>;

    // ONLY BYTE OFFSET

    fn next(&mut self) -> Option<Self::Item>
    {
        while let Some(ch) = self.next_char()
        {
            let ch = match ch
            {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };

            let ch_len = ch.len_utf8();
            let start = self.byte_offset;
            self.byte_offset += ch_len;

            match ch
            {
                '{' | '}' | '[' | ']' | ':' | ',' =>
                {
                    return Some(Ok(start));
                }

                c if c.is_whitespace() => continue,

                '"' =>
                {
                    // string token
                    loop
                    {
                        match self.next_char()?
                        {
                            Ok(inner) =>
                            {
                                self.byte_offset += inner.len_utf8();
                                if inner == '"'
                                {
                                    break;
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    return Some(Ok(start));
                }

                c if c.is_ascii_digit() || c == '-' =>
                {
                    // number token
                    loop
                    {
                        match self.next_char()
                        {
                            Some(Ok(inner)) =>
                            {
                                if inner.is_ascii_digit()
                                    || inner == '.'
                                    || inner == 'e'
                                    || inner == 'E'
                                    || inner == '+'
                                    || inner == '-'
                                {
                                    self.byte_offset += inner.len_utf8();
                                }
                                else
                                {
                                    self.unread(inner);
                                    break;
                                }
                            }
                            Some(Err(e)) => return Some(Err(e)),
                            None => break,
                        }
                    }
                    return Some(Ok(start));
                }

                't' =>
                {
                    // true
                    for expected in ['r', 'u', 'e']
                    {
                        match self.next_char()?
                        {
                            Ok(c) =>
                            {
                                self.byte_offset += c.len_utf8();
                                if c != expected
                                {
                                    return Some(Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "invalid token",
                                    )));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    return Some(Ok(start));
                }

                'f' =>
                {
                    // false
                    for expected in ['a', 'l', 's', 'e']
                    {
                        match self.next_char()?
                        {
                            Ok(c) =>
                            {
                                self.byte_offset += c.len_utf8();
                                if c != expected
                                {
                                    return Some(Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "invalid token",
                                    )));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    return Some(Ok(start));
                }

                'n' =>
                {
                    // null
                    for expected in ['u', 'l', 'l']
                    {
                        match self.next_char()?
                        {
                            Ok(c) =>
                            {
                                self.byte_offset += c.len_utf8();
                                if c != expected
                                {
                                    return Some(Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "invalid token",
                                    )));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    return Some(Ok(start));
                }

                _ =>
                {
                    return Some(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unexpected character: {}", ch),
                    )));
                }
            }
        }

        None
    }
}

// Entry points
pub fn json_tokens_from_reader<R: Read>(reader: R) -> JsonTokens<R>
{
    JsonTokens::new(reader)
}

pub fn json_tokens_from_file(path: &str) -> io::Result<JsonTokens<File>>
{
    Ok(JsonTokens::new(File::open(path)?))
}

pub fn json_tokens_from_str(s: &str) -> JsonTokens<io::Cursor<&[u8]>>
{
    JsonTokens::new(io::Cursor::new(s.as_bytes()))
}

// ------------------------------------------------------------
// RETOKENIZER: classify token by re-reading source
// ------------------------------------------------------------
pub fn classify_token(src: &str, start: usize) -> TokenKind
{
    let b = src.as_bytes()[start];
    match b
    {
        b'{' => TokenKind::LBrace,
        b'}' => TokenKind::RBrace,
        b'[' => TokenKind::LBracket,
        b']' => TokenKind::RBracket,
        b':' => TokenKind::Colon,
        b',' => TokenKind::Comma,
        b'"' => TokenKind::String,
        b'-' | b'0' ..= b'9' => TokenKind::Number,
        b't' => TokenKind::True,
        b'f' => TokenKind::False,
        b'n' => TokenKind::Null,
        _ => TokenKind::Unknown,
    }
}

#[derive(Debug, PartialEq)]
pub enum TokVal<'a>
{
    NewObj,
    EndObj,
    NewArr,
    EndArr,
    Col,
    Com,
    Bit(bool),
    Nil,
    Num(&'a str),
    Txt(&'a str),
    Unknown,
}

pub fn token_value<'a>(src: &'a str, start: usize) -> TokVal<'a>
{
    match classify_token(src, start)
    {
        TokenKind::LBrace => TokVal::NewObj,
        TokenKind::RBrace => TokVal::EndObj,
        TokenKind::LBracket => TokVal::NewArr,
        TokenKind::RBracket => TokVal::EndArr,
        TokenKind::Colon => TokVal::Col,
        TokenKind::Comma => TokVal::Com,

        TokenKind::True => TokVal::Bit(true),
        TokenKind::False => TokVal::Bit(false),
        TokenKind::Null => TokVal::Nil,

        TokenKind::String => TokVal::Txt(slice_string(src, start)),
        TokenKind::Number => TokVal::Num(slice_number(src, start)),

        TokenKind::Unknown => TokVal::Unknown,
    }
}

pub fn slice_string(src: &str, start: usize) -> &str
{
    let s = &src[start ..];
    let bytes = s.as_bytes();

    let mut i = 1; // skip initial quote
    while i < bytes.len()
    {
        if bytes[i] == b'"'
        {
            break;
        }
        i += 1;
    }

    &s[1 .. i] // inner contents only
}

pub fn slice_number(src: &str, start: usize) -> &str
{
    let bytes = src.as_bytes();
    let mut end = start;

    while end < bytes.len()
    {
        match bytes[end]
        {
            b'0' ..= b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => end += 1,
            _ => break,
        }
    }

    &src[start .. end]
}
