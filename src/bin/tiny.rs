use crate::lexer::json_tokens_from_str;

mod streaming
{
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
                    Ok(n) => n,
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
                    {
                        if let Some(ch) = char::from_u32(self.codepoint)
                        {
                            return Some(Ok(ch));
                        }
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

    pub fn codepoints_from_file(path: &str)
    -> io::Result<Utf8Codepoints<File>>
    {
        Ok(Utf8Codepoints::new(File::open(path)?))
    }

    pub fn codepoints_from_str(s: &str) -> Utf8Codepoints<io::Cursor<&[u8]>>
    {
        Utf8Codepoints::new(io::Cursor::new(s.as_bytes()))
    }
}

mod lexer
{
    // ============================================================
    //  JSON TOKEN STREAMING STATE MACHINE (LEXER ONLY)
    // ============================================================

    use std::{
        fs::File,
        io::{self, Read},
    };

    use crate::streaming::Utf8Codepoints;

    #[derive(Debug, Clone, PartialEq)]
    pub enum JsonToken
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
        Number(String),
        String(String),
    }

    pub struct JsonTokens<R: Read>
    {
        chars: Utf8Codepoints<R>,
        buf: Option<char>,
    }

    impl<R: Read> JsonTokens<R>
    {
        pub fn new(reader: R) -> Self
        {
            Self { chars: Utf8Codepoints::new(reader), buf: None }
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
        type Item = io::Result<JsonToken>;

        fn next(&mut self) -> Option<Self::Item>
        {
            while let Some(ch) = self.next_char()
            {
                let ch = match ch
                {
                    Ok(c) => c,
                    Err(e) => return Some(Err(e)),
                };

                match ch
                {
                    '{' => return Some(Ok(JsonToken::LBrace)),
                    '}' => return Some(Ok(JsonToken::RBrace)),
                    '[' => return Some(Ok(JsonToken::LBracket)),
                    ']' => return Some(Ok(JsonToken::RBracket)),
                    ':' => return Some(Ok(JsonToken::Colon)),
                    ',' => return Some(Ok(JsonToken::Comma)),

                    // Skip whitespace
                    c if c.is_whitespace() => continue,

                    // String
                    '"' =>
                    {
                        let mut s = String::new();
                        while let Some(Ok(ch)) = self.next_char()
                        {
                            let c = ch;
                            if c == '"'
                            {
                                break;
                            }
                            s.push(c);
                        }
                        return Some(Ok(JsonToken::String(s)));
                    }

                    // Number (very loose for now)
                    c if c.is_ascii_digit() || c == '-' =>
                    {
                        let mut s = String::new();
                        s.push(c);
                        while let Some(Ok(ch)) = self.next_char()
                        {
                            let c = ch;
                            if c.is_ascii_digit()
                                || c == '.'
                                || c == 'e'
                                || c == 'E'
                                || c == '+'
                                || c == '-'
                            {
                                s.push(c);
                            }
                            else
                            {
                                self.unread(c);
                                break;
                            }
                        }
                        return Some(Ok(JsonToken::Number(s)));
                    }

                    // true / false / null
                    't' =>
                    {
                        for expected in ['r', 'u', 'e']
                        {
                            let c = self.next_char()?;
                            if !matches!(c, Ok(expected))
                            {
                                return Some(Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "invalid token",
                                )));
                            }
                        }
                        return Some(Ok(JsonToken::True));
                    }
                    'f' =>
                    {
                        for expected in ['a', 'l', 's', 'e']
                        {
                            let c = self.next_char()?;
                            if !matches!(c, Ok(expected))
                            {
                                return Some(Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "invalid token",
                                )));
                            }
                        }
                        return Some(Ok(JsonToken::False));
                    }
                    'n' =>
                    {
                        for expected in ['u', 'l', 'l']
                        {
                            let c = self.next_char()?;
                            if !matches!(c, Ok(expected))
                            {
                                return Some(Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "invalid token",
                                )));
                            }
                        }
                        return Some(Ok(JsonToken::Null));
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

    // Entry points for JSON tokens
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
}

fn main()
{
    let code = r#"{"key": "value", "number": 123, "bool": true, "null": null}"#;
    println!("Input JSON: {}", code);
    for token in json_tokens_from_str(code)
    {
        match token
        {
            Ok(t) => println!("{:?}", t),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
