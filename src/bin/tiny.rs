use crate::lexer::{classify_token, json_tokens_from_str, token_value};

mod codepoints
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
            Self {
                chars: Utf8Codepoints::new(reader),
                buf: None,
                byte_offset: 0,
            }
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
}

mod parser
{
    use crate::lexer::{
        TokVal, TokenKind, classify_token, json_tokens_from_reader, token_value,
    };
    use std::io::{self, Read};
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum ParseError
    {
        #[error("IO error: {0}")]
        Io(#[from] io::Error),

        #[error(
            "Unexpected token at offset {offset}: expected {expected}, got {got:?}"
        )]
        UnexpectedToken
        {
            offset: usize,
            expected: String,
            got: TokenKind,
        },

        #[error("Unexpected end of input: expected {expected}")]
        UnexpectedEof
        {
            expected: String
        },

        #[error("Missing closing bracket for container at row {row_id}")]
        UnclosedContainer
        {
            row_id: u32
        },

        #[error("Invalid JSON structure at offset {offset}: {message}")]
        InvalidStructure
        {
            offset: usize, message: String
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum RowType
    {
        Obj,
        Arr,
        Str,
        Num,
        Bit,
        Null,
    }

    /// Invariant: id is the index within its container.
    #[derive(Debug, Clone)]
    pub struct Row
    {
        pub id: u32, // index within parent container (also row index in table)
        pub par: u32, // parent row ID
        pub key: String, // empty for array elements (will use string index)
        pub val: String, // empty for objects/arrays
        pub ty: RowType,
    }

    impl Row
    {
        pub fn subnodes<'row>(
            &'row self,
            table: &'row [Row],
        ) -> impl Iterator<Item = &'row Row>
        {
            table.iter().filter(|r| r.id > self.id && self.id == r.par)
        }
    }

    pub fn parse_from_str(src: &str) -> io::Result<Vec<Row>>
    {
        Parser::new(src).parse().map_err(|e| match e
        {
            ParseError::Io(io_err) => io_err,
            other =>
            {
                io::Error::new(io::ErrorKind::InvalidData, other.to_string())
            }
        })
    }

    pub fn parse_from_file(path: &str) -> io::Result<Vec<Row>>
    {
        let src = std::fs::read_to_string(path)?;
        Parser::new(&src).parse().map_err(|e| match e
        {
            ParseError::Io(io_err) => io_err,
            other =>
            {
                io::Error::new(io::ErrorKind::InvalidData, other.to_string())
            }
        })
    }

    pub fn parse_from_reader<R: Read>(
        src: &str,
        _reader: R,
    ) -> io::Result<Vec<Row>>
    {
        // Note: we only use src since we re-tokenize from it
        Parser::new(src).parse().map_err(|e| match e
        {
            ParseError::Io(io_err) => io_err,
            other =>
            {
                io::Error::new(io::ErrorKind::InvalidData, other.to_string())
            }
        })
    }

    struct Parser<'src>
    {
        src: &'src str,
        tokens: Vec<usize>,
        token_idx: usize,
        rows: Vec<Row>,
        current_parent: u32,
        next_row_id: u32,
        container_stack: Vec<ContainerInfo>,
    }

    #[derive(Debug, Clone)]
    struct ContainerInfo
    {
        row_id: u32,
        ty: ContainerType,
        child_count: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum ContainerType
    {
        Object,
        Array,
    }

    impl<'src> Parser<'src>
    {
        fn new(src: &'src str) -> Self
        {
            Self {
                src,
                tokens: Vec::new(),
                token_idx: 0,
                rows: Vec::new(),
                current_parent: 0,
                next_row_id: 0,
                container_stack: Vec::new(),
            }
        }

        pub fn parse(mut self) -> Result<Vec<Row>, ParseError>
        {
            // Collect all tokens first
            let token_iter = json_tokens_from_reader(std::io::Cursor::new(
                self.src.as_bytes(),
            ));
            for token_result in token_iter
            {
                self.tokens.push(token_result?);
            }

            // Add root node (id=0, parent=0)
            // self.add_row(0, String::new(), String::new(), RowType::Obj);
            // self.add_row(0, String::new(), '{'.to_string(), RowType::Obj);
            // self.current_parent = 0;
            // self.container_stack.push(ContainerInfo {
            //     row_id: 0,
            //     ty: ContainerType::Object,
            //     child_count: 0,
            // });

            // Parse the top-level value
            if self.tokens.is_empty()
            {
                return Err(ParseError::UnexpectedEof {
                    expected: "JSON value".to_string(),
                });
            }

            self.parse_value()?;

            // Verify all containers are closed
            self.verify_all_closed()?;

            Ok(self.rows)
        }

        fn peek_token(&self) -> Option<(usize, TokenKind)>
        {
            if self.token_idx < self.tokens.len()
            {
                let offset = self.tokens[self.token_idx];
                Some((offset, classify_token(self.src, offset)))
            }
            else
            {
                None
            }
        }

        fn next_token(&mut self) -> Option<(usize, TokenKind)>
        {
            let result = self.peek_token();
            if result.is_some()
            {
                self.token_idx += 1;
            }
            result
        }

        fn expect_token(
            &mut self,
            expected: TokenKind,
        ) -> Result<usize, ParseError>
        {
            match self.next_token()
            {
                Some((offset, kind)) if kind == expected => Ok(offset),
                Some((offset, kind)) => Err(ParseError::UnexpectedToken {
                    offset,
                    expected: format!("{:?}", expected),
                    got: kind,
                }),
                None => Err(ParseError::UnexpectedEof {
                    expected: format!("{:?}", expected),
                }),
            }
        }

        fn accept_token(&mut self, kind: TokenKind) -> bool
        {
            if let Some((_, tk)) = self.peek_token()
            {
                if tk == kind
                {
                    self.token_idx += 1;
                    return true;
                }
            }
            false
        }

        fn add_row(
            &mut self,
            parent_id: u32,
            key: String,
            val: String,
            ty: RowType,
        ) -> u32
        {
            let row_id = self.next_row_id;
            self.next_row_id += 1;

            self.rows.push(Row { id: row_id, par: parent_id, key, val, ty });

            row_id
        }

        fn parse_value(&mut self) -> Result<(), ParseError>
        {
            match self.peek_token()
            {
                Some((offset, TokenKind::LBrace)) =>
                {
                    self.parse_object()?;
                }
                Some((offset, TokenKind::LBracket)) =>
                {
                    self.parse_array()?;
                }
                Some((offset, kind)) =>
                {
                    // Primitive value
                    self.parse_primitive(offset, kind)?;
                }
                None =>
                {
                    return Err(ParseError::UnexpectedEof {
                        expected: "JSON value".to_string(),
                    });
                }
            }

            Ok(())
        }

        fn parse_object(&mut self) -> Result<(), ParseError>
        {
            let offset = self.expect_token(TokenKind::LBrace)?;

            // Get the key for this object from the current container
            let key = self.get_current_key();

            // Add the object row
            let obj_row_id = self.add_row(
                self.current_parent,
                key,
                // String::new(),
                '{'.to_string(),
                RowType::Obj,
            );

            // Push container onto stack
            self.container_stack.push(ContainerInfo {
                row_id: obj_row_id,
                ty: ContainerType::Object,
                child_count: 0,
            });

            let old_parent = self.current_parent;
            self.current_parent = obj_row_id;

            // Parse object members
            if !self.accept_token(TokenKind::RBrace)
            {
                loop
                {
                    // Expect key (string)
                    let key_offset = self.expect_token(TokenKind::String)?;
                    let key_val = token_value(self.src, key_offset);
                    let key_str = if let TokVal::Txt(s) = key_val
                    {
                        s.to_string()
                    }
                    else
                    {
                        return Err(ParseError::InvalidStructure {
                            offset: key_offset,
                            message: "Expected string key in object"
                                .to_string(),
                        });
                    };

                    // Store the key for the next value
                    if let Some(info) = self.container_stack.last_mut()
                    {
                        info.child_count += 1;
                    }

                    // Expect colon
                    self.expect_token(TokenKind::Colon)?;

                    // Store key temporarily
                    let saved_key = key_str;

                    // Parse value - we need to set the key before parsing
                    // For objects, the key is explicitly provided
                    self.parse_object_member_value(saved_key)?;

                    // Check for comma or end
                    if self.accept_token(TokenKind::Comma)
                    {
                        // Continue to next member
                        continue;
                    }
                    else if self.accept_token(TokenKind::RBrace)
                    {
                        break;
                    }
                    else
                    {
                        return Err(ParseError::UnexpectedEof {
                            expected: "comma or closing brace".to_string(),
                        });
                    }
                }
            }

            // Pop container from stack
            self.container_stack.pop();
            self.current_parent = old_parent;

            // Increment parent's child count
            if let Some(info) = self.container_stack.last_mut()
            {
                info.child_count += 1;
            }

            Ok(())
        }

        fn parse_object_member_value(
            &mut self,
            key: String,
        ) -> Result<(), ParseError>
        {
            match self.peek_token()
            {
                Some((offset, TokenKind::LBrace)) =>
                {
                    // Nested object - key is stored before we recurse
                    let saved_parent = self.current_parent;

                    // Temporarily pop to add the object with correct key
                    let offset = self.expect_token(TokenKind::LBrace)?;

                    let obj_row_id = self.add_row(
                        self.current_parent,
                        key,
                        // String::new(),
                        '{'.to_string(),
                        RowType::Obj,
                    );

                    self.container_stack.push(ContainerInfo {
                        row_id: obj_row_id,
                        ty: ContainerType::Object,
                        child_count: 0,
                    });

                    self.current_parent = obj_row_id;

                    // Parse object contents
                    if !self.accept_token(TokenKind::RBrace)
                    {
                        loop
                        {
                            let key_offset =
                                self.expect_token(TokenKind::String)?;
                            let key_val = token_value(self.src, key_offset);
                            let nested_key = if let TokVal::Txt(s) = key_val
                            {
                                s.to_string()
                            }
                            else
                            {
                                return Err(ParseError::InvalidStructure {
                                    offset: key_offset,
                                    message: "Expected string key".to_string(),
                                });
                            };

                            self.expect_token(TokenKind::Colon)?;
                            self.parse_object_member_value(nested_key)?;

                            if self.accept_token(TokenKind::Comma)
                            {
                                continue;
                            }
                            else if self.accept_token(TokenKind::RBrace)
                            {
                                break;
                            }
                            else
                            {
                                return Err(ParseError::UnexpectedEof {
                                    expected: "comma or closing brace"
                                        .to_string(),
                                });
                            }
                        }
                    }

                    self.container_stack.pop();
                    self.current_parent = saved_parent;
                }
                Some((offset, TokenKind::LBracket)) =>
                {
                    // Nested array
                    let saved_parent = self.current_parent;

                    let offset = self.expect_token(TokenKind::LBracket)?;

                    let arr_row_id = self.add_row(
                        self.current_parent,
                        key,
                        // String::new(),
                        '['.to_string(),
                        RowType::Arr,
                    );

                    self.container_stack.push(ContainerInfo {
                        row_id: arr_row_id,
                        ty: ContainerType::Array,
                        child_count: 0,
                    });

                    self.current_parent = arr_row_id;

                    if !self.accept_token(TokenKind::RBracket)
                    {
                        loop
                        {
                            let arr_idx = if let Some(info) =
                                self.container_stack.last()
                            {
                                info.child_count
                            }
                            else
                            {
                                0
                            };

                            self.parse_array_element(arr_idx)?;

                            if let Some(info) = self.container_stack.last_mut()
                            {
                                info.child_count += 1;
                            }

                            if self.accept_token(TokenKind::Comma)
                            {
                                continue;
                            }
                            else if self.accept_token(TokenKind::RBracket)
                            {
                                break;
                            }
                            else
                            {
                                return Err(ParseError::UnexpectedEof {
                                    expected: "comma or closing bracket"
                                        .to_string(),
                                });
                            }
                        }
                    }

                    self.container_stack.pop();
                    self.current_parent = saved_parent;
                }
                Some((offset, kind)) =>
                {
                    // Primitive value
                    let val_str = self.extract_value_string(offset, kind);
                    let row_type = self.kind_to_row_type(kind);

                    self.add_row(self.current_parent, key, val_str, row_type);

                    self.next_token(); // Consume the token
                }
                None =>
                {
                    return Err(ParseError::UnexpectedEof {
                        expected: "value".to_string(),
                    });
                }
            }

            Ok(())
        }

        fn parse_array(&mut self) -> Result<(), ParseError>
        {
            let offset = self.expect_token(TokenKind::LBracket)?;

            // Get the key for this array from the current container
            let key = self.get_current_key();

            // Add the array row
            let arr_row_id = self.add_row(
                self.current_parent,
                key,
                // String::new(),
                '['.to_string(),
                RowType::Arr,
            );

            // Push container onto stack
            self.container_stack.push(ContainerInfo {
                row_id: arr_row_id,
                ty: ContainerType::Array,
                child_count: 0,
            });

            let old_parent = self.current_parent;
            self.current_parent = arr_row_id;

            // Parse array elements
            if !self.accept_token(TokenKind::RBracket)
            {
                loop
                {
                    let arr_idx = if let Some(info) =
                        self.container_stack.last()
                    {
                        info.child_count
                    }
                    else
                    {
                        0
                    };

                    self.parse_array_element(arr_idx)?;

                    if let Some(info) = self.container_stack.last_mut()
                    {
                        info.child_count += 1;
                    }

                    // Check for comma or end
                    if self.accept_token(TokenKind::Comma)
                    {
                        continue;
                    }
                    else if self.accept_token(TokenKind::RBracket)
                    {
                        break;
                    }
                    else
                    {
                        return Err(ParseError::UnexpectedEof {
                            expected: "comma or closing bracket".to_string(),
                        });
                    }
                }
            }

            // Pop container from stack
            self.container_stack.pop();
            self.current_parent = old_parent;

            // Increment parent's child count
            if let Some(info) = self.container_stack.last_mut()
            {
                info.child_count += 1;
            }

            Ok(())
        }

        fn parse_array_element(&mut self, index: u32)
        -> Result<(), ParseError>
        {
            let key = index.to_string();

            match self.peek_token()
            {
                Some((offset, TokenKind::LBrace)) =>
                {
                    self.parse_object()?;
                    // Update the key of the last added row (the object)
                    if let Some(row) = self.rows.last_mut()
                    {
                        row.key = key;
                    }
                }
                Some((offset, TokenKind::LBracket)) =>
                {
                    self.parse_array()?;
                    // Update the key of the last added row (the array)
                    if let Some(row) = self.rows.last_mut()
                    {
                        row.key = key;
                    }
                }
                Some((offset, kind)) =>
                {
                    self.parse_primitive(offset, kind)?;
                    // Update the key of the last added row
                    if let Some(row) = self.rows.last_mut()
                    {
                        row.key = key;
                    }
                }
                None =>
                {
                    return Err(ParseError::UnexpectedEof {
                        expected: "array element".to_string(),
                    });
                }
            }

            Ok(())
        }

        fn parse_primitive(
            &mut self,
            offset: usize,
            kind: TokenKind,
        ) -> Result<(), ParseError>
        {
            let key = self.get_current_key();
            let val_str = self.extract_value_string(offset, kind);
            let row_type = self.kind_to_row_type(kind);

            self.add_row(self.current_parent, key, val_str, row_type);
            self.next_token(); // Consume the token

            // Increment parent's child count
            if let Some(info) = self.container_stack.last_mut()
            {
                info.child_count += 1;
            }

            Ok(())
        }

        fn get_current_key(&self) -> String
        {
            if let Some(info) = self.container_stack.last()
            {
                match info.ty
                {
                    ContainerType::Array => info.child_count.to_string(),
                    ContainerType::Object => String::new(), // Will be set by caller
                }
            }
            else
            {
                String::new()
            }
        }

        fn extract_value_string(&self, offset: usize, kind: TokenKind)
        -> String
        {
            let tok_val = token_value(self.src, offset);
            match tok_val
            {
                TokVal::Txt(s) => s.to_string(),
                TokVal::Num(s) => s.to_string(),
                TokVal::Bit(b) => b.to_string(),
                TokVal::Nil => "null".to_string(),
                _ => String::new(),
            }
        }

        fn kind_to_row_type(&self, kind: TokenKind) -> RowType
        {
            match kind
            {
                TokenKind::String => RowType::Str,
                TokenKind::Number => RowType::Num,
                TokenKind::True | TokenKind::False => RowType::Bit,
                TokenKind::Null => RowType::Null,
                _ => RowType::Null,
            }
        }

        fn verify_all_closed(&self) -> Result<(), ParseError>
        {
            // If we have any containers left on the stack (except root), they're unclosed
            if self.container_stack.len() > 1
            {
                if let Some(unclosed) = self.container_stack.last()
                {
                    return Err(ParseError::UnclosedContainer {
                        row_id: unclosed.row_id,
                    });
                }
            }

            Ok(())
        }
    }
}

mod formatter
{
    use crate::parser::{Row, RowType};
    use crossterm::style::Stylize;
    use std::io::{self, IsTerminal};

    /// Helper to convert bool to usize for indexing
    #[inline]
    fn udx(b: bool) -> usize
    {
        b as usize
    }

    /// Theme configuration for JSON syntax highlighting
    #[derive(Clone)]
    pub struct Theme
    {
        pub style_key: fn(&str) -> String,
        pub style_quote_key: fn(&str) -> String,
        pub style_open: fn(&str) -> String,
        pub style_end: fn(&str) -> String,
        pub style_quote_val: fn(&str) -> String,
        pub style_txt: fn(&str) -> String,
        pub style_nil: fn(&str) -> String,
        pub style_num: fn(&str) -> String,
        pub style_bit: fn(&str) -> String,
        pub style_punctuation: fn(&str) -> String,
    }

    impl Theme
    {
        /// Colored theme using crossterm
        pub fn colored() -> Self
        {
            Self {
                style_key: |s| s.green().to_string(),
                style_quote_key: |s| s.dark_green().to_string(),
                style_open: |s| s.red().to_string(),
                style_end: |s| s.red().to_string(),
                style_quote_val: |s| s.dark_red().to_string(),
                style_txt: |s| s.red().to_string(),
                style_nil: |s| s.red().to_string(),
                style_num: |s| s.cyan().to_string(),
                style_bit: |s| s.yellow().to_string(),
                style_punctuation: |s| s.white().to_string(),
            }
        }

        /// Plain theme (no colors)
        pub fn plain() -> Self
        {
            Self {
                style_key: |s| s.to_string(),
                style_quote_key: |s| s.to_string(),
                style_open: |s| s.to_string(),
                style_end: |s| s.to_string(),
                style_quote_val: |s| s.to_string(),
                style_txt: |s| s.to_string(),
                style_nil: |s| s.to_string(),
                style_num: |s| s.to_string(),
                style_bit: |s| s.to_string(),
                style_punctuation: |s| s.to_string(),
            }
        }
    }

    /// Format configuration
    pub struct FormatConfig
    {
        pub indent_size: usize,
        pub max_line_length: usize,
        pub theme: Theme,
    }

    impl Default for FormatConfig
    {
        fn default() -> Self
        {
            let use_colors = io::stdout().is_terminal();
            Self {
                indent_size: 4,
                max_line_length: 80,
                theme: if use_colors
                {
                    Theme::colored()
                }
                else
                {
                    Theme::plain()
                },
            }
        }
    }

    impl FormatConfig
    {
        pub fn new() -> Self
        {
            Self::default()
        }

        pub fn with_indent(mut self, size: usize) -> Self
        {
            self.indent_size = size;
            self
        }

        pub fn with_max_line_length(mut self, length: usize) -> Self
        {
            self.max_line_length = length;
            self
        }

        pub fn with_colors(mut self, use_colors: bool) -> Self
        {
            self.theme =
                if use_colors { Theme::colored() } else { Theme::plain() };
            self
        }

        pub fn with_theme(mut self, theme: Theme) -> Self
        {
            self.theme = theme;
            self
        }
    }

    /// Format the entire table into lines of JSON
    pub fn format_table(table: &[Row], config: &FormatConfig) -> Vec<String>
    {
        if table.is_empty()
        {
            return vec![];
        }

        let mut lines = Vec::new();
        let mut accumulate_indent = 0;

        for row in table
        {
            let parent = table.get(row.par as usize).unwrap_or(row);
            let next = table.get(row.id as usize + 1).unwrap_or(row);

            // Determine row properties
            let is_parent = row.id == next.par;
            let empty = matches!(row.ty, RowType::Obj | RowType::Arr)
                && row.id != next.par;
            let last = next.id == row.id || next.par < row.par;

            // Format the main row line
            let indent = " ".repeat(accumulate_indent * config.indent_size);

            // Build key part
            let key = if parent.ty != RowType::Arr && row.id != 0
            {
                format!(
                    "{}{}{}",
                    (config.theme.style_quote_key)("\""),
                    (config.theme.style_key)(&row.key),
                    (config.theme.style_quote_key)("\"")
                )
            }
            else
            {
                String::new()
            };

            let colon = if row.id != 0 && !key.is_empty()
            {
                (config.theme.style_punctuation)(": ")
            }
            else
            {
                String::new()
            };

            // Build value part
            let val = match row.ty
            {
                RowType::Arr if empty =>
                {
                    format!(
                        "{}{}",
                        (config.theme.style_open)("["),
                        (config.theme.style_end)("]")
                    )
                }
                RowType::Arr => (config.theme.style_open)("["),
                RowType::Obj if empty =>
                {
                    format!(
                        "{}{}",
                        (config.theme.style_open)("{"),
                        (config.theme.style_end)("}")
                    )
                }
                RowType::Obj => (config.theme.style_open)("{"),
                RowType::Null => (config.theme.style_nil)(&row.val),
                RowType::Bit => (config.theme.style_bit)(&row.val),
                RowType::Str =>
                {
                    format!(
                        "{}{}{}",
                        (config.theme.style_quote_val)("\""),
                        (config.theme.style_txt)(&row.val),
                        (config.theme.style_quote_val)("\"")
                    )
                }
                RowType::Num => (config.theme.style_num)(&row.val),
            };

            let comma = if (empty || !is_parent) && !last
            {
                (config.theme.style_punctuation)(",")
            }
            else
            {
                String::new()
            };

            let line = format!("{}{}{}{}{}", indent, key, colon, val, comma);
            lines.push(line);

            // Update indent for next iteration
            if is_parent && !empty
            {
                accumulate_indent += 1;
            }
            else if last
            {
                accumulate_indent -= 1;
            }

            // Emit closing brackets by walking up the tree
            let mut node = row;
            let mut increment_dedent = accumulate_indent;

            while node.par > next.par || next.id == node.id
            {
                let parent_node = table.get(node.par as usize).unwrap_or(node);

                // Determine if we need a closing bracket
                let brace = if node.par > next.par || node.id == next.id
                {
                    if parent_node.ty == RowType::Obj
                    {
                        (config.theme.style_end)("}")
                    }
                    else
                    {
                        (config.theme.style_end)("]")
                    }
                }
                else
                {
                    String::new()
                };

                // Next node is sibling of current node's parent
                let comma = if next.par == parent_node.par
                {
                    (config.theme.style_punctuation)(",")
                }
                else
                {
                    String::new()
                };

                if !brace.is_empty()
                {
                    let indent =
                        " ".repeat(increment_dedent * config.indent_size);
                    let line = format!("{}{}{}", indent, brace, comma);
                    lines.push(line);
                }

                node = parent_node;
                increment_dedent = increment_dedent.saturating_sub(1);
            }

            // Adjust accumulate_indent if needed
            if accumulate_indent > increment_dedent + 1
            {
                accumulate_indent -= 1;
            }

            // Handle the final root closing bracket
            if let Some(root) = table.first()
            {
                let last_before_root = next.id == row.id;
                if last_before_root
                {
                    let indent =
                        " ".repeat(increment_dedent * config.indent_size);
                    let end = if root.ty == RowType::Obj
                    {
                        (config.theme.style_end)("}")
                    }
                    else
                    {
                        (config.theme.style_end)("]")
                    };
                    let line = format!("{}{}", indent, end);
                    lines.push(line);
                }
            }
        }

        lines
    }

    /// Check if a row and its children can fit on one line (within max_length)
    fn can_inline(table: &[Row], row_idx: usize, max_length: usize) -> bool
    {
        let row = &table[row_idx];

        // Only consider inlining objects and arrays
        if !matches!(row.ty, RowType::Obj | RowType::Arr)
        {
            return false;
        }

        // Calculate the projected length if inlined
        let projected = calculate_inline_length(table, row_idx);
        projected <= max_length
    }

    /// Calculate the length if this row and its children were inlined
    fn calculate_inline_length(table: &[Row], row_idx: usize) -> usize
    {
        let row = &table[row_idx];
        let mut length = 0;

        // Opening bracket
        length += 1;

        // Get all direct children
        let children: Vec<_> =
            table.iter().enumerate().filter(|(_, r)| r.par == row.id).collect();

        for (idx, (child_idx, child)) in children.iter().enumerate()
        {
            // Key (if object)
            if row.ty == RowType::Obj
            {
                length += child.key.len() + 4; // "key":
            }

            // Value
            match child.ty
            {
                RowType::Str => length += child.val.len() + 2, // "value"
                RowType::Num | RowType::Bit | RowType::Null =>
                {
                    length += child.val.len()
                }
                RowType::Obj | RowType::Arr =>
                {
                    // Recursively check nested structures
                    length += calculate_inline_length(table, *child_idx);
                }
            }

            // Comma and space between elements
            if idx < children.len() - 1
            {
                length += 2; // ", "
            }
        }

        // Closing bracket
        length += 1;

        length
    }

    /// Format table with inlining for small structures (respects max_line_length)
    pub fn format_table_compact(
        table: &[Row],
        config: &FormatConfig,
    ) -> Vec<String>
    {
        // TODO: Implement smart inlining based on max_line_length
        // For now, delegate to standard formatter
        format_table(table, config)
    }

    /// Print formatted JSON to stdout
    pub fn print_formatted(table: &[Row], config: &FormatConfig)
    {
        for line in format_table(table, config)
        {
            println!("{}", line);
        }
    }

    /// Format and return as a single string
    pub fn format_to_string(table: &[Row], config: &FormatConfig) -> String
    {
        format_table(table, config).join("\n")
    }
}

fn main()
{
    let code = r#"{
        "k": "v", "num": 123, "bit": true, "nil": null,
        "arr": [1, 2, 3],
        "obj": { "a": "b", "c": "d" }
    }"#;

    println!("Input JSON: {}", code);
    for token in json_tokens_from_str(code)
    {
        match token
        {
            Ok(at) =>
            {
                let ty = classify_token(code, at);
                let val = token_value(code, at);
                println!("{:<3?} {:<6} {:?}", at, format!("{ty:?}"), val);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!();

    println!("Input JSON: {}", code);
    let rows = parser::parse_from_str(code).unwrap();
    for r in rows.iter()
    {
        println!("{:?}", r);
    }

    formatter::print_formatted(&rows, &formatter::FormatConfig::new());
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::formatter::FormatConfig;
    use crate::formatter::format_table;
    use crate::lexer::*;
    use crate::parser::*;

    /// Tests [json_tokens_from_str], [classify_token], and [token_value]
    /// together to verify individual tokens are in the right order and
    /// correctly classified while also proving actual token values.
    /// Does not invoke the parser so invalid source is allowed.
    #[test]
    fn test_token_index_and_order()
    {
        let code = r#"[1, 2, [3, 4], 5, 6]"#;
        for token in json_tokens_from_str(code)
        {
            match token
            {
                Ok(at) =>
                {
                    let ty = classify_token(code, at);
                    let val = token_value(code, at);
                    println!("{:<3} {:<15} {:?}", at, format!("{ty:?}"), val);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        let expected = [
            (0, TokenKind::LBracket, TokVal::NewArr),
            (1, TokenKind::Number, TokVal::Num("1")),
            (2, TokenKind::Comma, TokVal::Com),
            (4, TokenKind::Number, TokVal::Num("2")),
            (5, TokenKind::Comma, TokVal::Com),
            (7, TokenKind::LBracket, TokVal::NewArr),
            (8, TokenKind::Number, TokVal::Num("3")),
            (9, TokenKind::Comma, TokVal::Com),
            (11, TokenKind::Number, TokVal::Num("4")),
            (12, TokenKind::RBracket, TokVal::EndArr),
            (13, TokenKind::Comma, TokVal::Com),
            (15, TokenKind::Number, TokVal::Num("5")),
            (16, TokenKind::Comma, TokVal::Com),
            (18, TokenKind::Number, TokVal::Num("6")),
            (19, TokenKind::RBracket, TokVal::EndArr),
        ];
        let token_stream =
            json_tokens_from_str(code).filter_map(|t| t.ok()).map(|at| {
                let ty = classify_token(code, at);
                let val = token_value(code, at);
                (at, ty, val)
            });
        for (resulted, expected) in token_stream.zip(expected.iter())
        {
            assert_eq!(resulted, *expected);
        }
    }

    /// This test contains invalid JSON but only tests tokenization which allows
    /// scrambled tokens. The parser must enforce structure.
    /// Does not invoke the parser so invalid source is allowed.
    #[test]
    fn test_valid_tokens_from_invalid_source()
    {
        let code = r#"[1, 2, [3, 4], 5, 6"#;
        let tokens: Vec<usize> =
            json_tokens_from_str(code).map(|row| row.unwrap()).collect();
        let msg = "Missing end arr is an error for the parser, not the lexer";
        assert_eq!(tokens.len(), 14, "{}", msg);
    }

    /// Verifies that structured values correctly return their direct subnodes.
    /// Invokes the parser so invalid source should be rejected.
    #[test]
    fn test_subnodes()
    {
        let code = r#"[1, 2, [3, 4], 5, 6]"#;
        let table = parser::parse_from_str(code).unwrap();
        assert_eq!(table[0].ty, RowType::Arr);
        assert_eq!(table[3].ty, RowType::Arr);
        let subnodes = table[3].subnodes(&table).collect::<Vec<_>>();
        assert_eq!(subnodes.len(), 2);
        assert_eq!(subnodes[0].val, "3");
        assert_eq!(subnodes[1].val, "4");
    }

    #[test]
    fn test_parser_json_structure()
    {
        let code = r#"[1, 2, [3, 4], 5, 6]"#;
        let table = parse_from_str(code).unwrap();

        assert_eq!(table.len(), 8);
        assert_eq!(table[0].val, "[");
        assert_eq!(table[1].val, "1");
        assert_eq!(table[2].val, "2");
        assert_eq!(table[3].ty, RowType::Arr);
        assert_eq!(table[3].key, "4"); // Array indices have numeric keys
        assert_eq!(table[3].val, "[");
        assert_eq!(table[4].val, "3");
        assert_eq!(table[5].val, "4");
        assert_eq!(table[6].val, "5");
        assert_eq!(table[7].val, "6");

        let code = r#"[1, 2, [3, 4], 5, 6"#;
        assert!(parse_from_str(code).is_err());

        // let code = r#"{ "k": "v", "num": 123, "bit": true, "nil": null, "arr": [1, 2, 3], "obj": { "a": "b", "c": "d" }"#;
        let code = r#"{
            "k": "v", "num": 123, "bit": true, "nil": null,
            "arr": [1, 2, 3],
            "obj": { "a": "b", "c": ["d"] }
        }"#;
        let table = parse_from_str(code).unwrap();

        assert_eq!(table.len(), 13);
        assert_eq!(
            (table[0].ty, table[0].key.as_str(), table[0].val.as_str()),
            (RowType::Obj, "", "{")
        );
        assert_eq!(
            (table[1].ty, table[1].key.as_str(), table[1].val.as_str()),
            (RowType::Str, "k", "v")
        );
        assert_eq!(
            (table[2].ty, table[2].key.as_str(), table[2].val.as_str()),
            (RowType::Num, "num", "123")
        );
        assert_eq!(
            (table[3].ty, table[3].key.as_str(), table[3].val.as_str()),
            (RowType::Bit, "bit", "true")
        );
        assert_eq!(
            (table[4].ty, table[4].key.as_str(), table[4].val.as_str()),
            (RowType::Null, "nil", "null")
        );
        assert_eq!(
            (table[5].ty, table[5].key.as_str(), table[5].val.as_str()),
            (RowType::Arr, "arr", "[")
        );
        assert_eq!(
            (table[6].ty, table[6].key.as_str(), table[6].val.as_str()),
            (RowType::Num, "0", "1")
        );
        assert_eq!(
            (table[7].ty, table[7].key.as_str(), table[7].val.as_str()),
            (RowType::Num, "2", "2")
        );
        assert_eq!(
            (table[8].ty, table[8].key.as_str(), table[8].val.as_str()),
            (RowType::Num, "4", "3")
        );
        assert_eq!(
            (table[9].ty, table[9].key.as_str(), table[9].val.as_str()),
            (RowType::Obj, "obj", "{")
        );
        assert_eq!(
            (table[10].ty, table[10].key.as_str(), table[10].val.as_str()),
            (RowType::Str, "a", "b")
        );
        assert_eq!(
            (table[11].ty, table[11].key.as_str(), table[11].val.as_str()),
            (RowType::Arr, "c", "[")
        );
        assert_eq!(
            (table[12].ty, table[12].key.as_str(), table[12].val.as_str()),
            (RowType::Str, "0", "d")
        );

        // Row { id: 0, par: 0, key: "", val: "{", ty: Obj }
        // Row { id: 1, par: 0, key: "k", val: "v", ty: Str }
        // Row { id: 2, par: 0, key: "num", val: "123", ty: Num }
        // Row { id: 3, par: 0, key: "bit", val: "true", ty: Bit }
        // Row { id: 4, par: 0, key: "nil", val: "null", ty: Null }
        // Row { id: 5, par: 0, key: "arr", val: "[", ty: Arr }
        // Row { id: 6, par: 5, key: "0", val: "1", ty: Num }
        // Row { id: 7, par: 5, key: "2", val: "2", ty: Num }
        // Row { id: 8, par: 5, key: "4", val: "3", ty: Num }
        // Row { id: 9, par: 0, key: "obj", val: "{", ty: Obj }
        // Row { id: 10, par: 9, key: "a", val: "b", ty: Str }
        // Row { id: 11, par: 9, key: "c", val: "[", ty: Arr }
        // Row { id: 12, par: 11, key: "0", val: "d", ty: Str }
        for row in &table
        {
            println!("{:?}", row);
        }
    }

    #[test]
    fn test_format_simple_array()
    {
        let json = r#"[1, 2, 3]"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        // Should produce formatted output
        assert!(!lines.is_empty());
        println!("Formatted array:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_nested_object()
    {
        let json = r#"{"name": "test", "nested": {"key": "value"}}"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        assert!(!lines.is_empty());
        println!("Formatted object:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_complex()
    {
        let json = r#"[1, 2, [3, 4], 5, 6]"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        assert!(!lines.is_empty());
        println!("Formatted complex:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_with_colors()
    {
        let json = r#"{"key": "value", "num": 42}"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(true);
        let lines = format_table(&table, &config);

        println!("Formatted with colors:");
        for line in &lines
        {
            println!("{}", line);
        }
    }
}
