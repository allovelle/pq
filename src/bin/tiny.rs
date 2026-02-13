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

mod format
{
    // TODO: If atomic (not structural), return only width of self and indent
    // TODO: If structural, return vec of logical indents (nests) and max
    // TODO: minimum width of each nest level so an outer formatter can decide
    // Layout::Structure(vec![(indent_level, min_width), ...])
    // Layout::Atomic(min_width)
    use super::parser::{Row, RowType};

    fn compute_width(row: &Row, table: &[Row]) -> usize
    {
        match row.ty
        {
            RowType::Obj | RowType::Arr => 2, // for {} or []
            RowType::Str => row.val.len() + 2, // for quotes
            RowType::Num | RowType::Bit | RowType::Null => row.val.len(),
        }
    }

    /// Rows already store values as strings so their literal width is just the
    /// length of the string plus any necessary punctuation.
    fn inline_width(row: &Row, table: &[Row]) -> usize
    {
        let key_val_sep = 2; // for ": "
        let quotes = 2; // for quotes around strings
        let key_width = row.key.len() + quotes + key_val_sep;

        match row.ty
        {
            RowType::Str => return key_width + row.val.len() + quotes,
            RowType::Num | RowType::Bit | RowType::Null =>
            {
                return key_width + row.val.len();
            }
            _ => (), // Obj & Arr is handled below
        }

        // Structural nodes sum the widths of their subnodes plus punctuation

        let braces = 4; // for { _ } or [ _ ]
        let comma = 2; // for ", "

        let mut total_width = if row.id == 0
        {
            0 // root has no key and no braces
        }
        else
        {
            braces + key_width
        };

        let mut nodes = row.subnodes(table).peekable();

        while let Some(subnode) = nodes.next()
        {
            total_width += inline_width(subnode, table);

            if nodes.peek().is_some()
            {
                total_width += comma;
            }
        }

        total_width
    }

    fn outline_width(row: &Row, table: &[Row]) -> usize
    {
        match row.ty
        {
            RowType::Obj | RowType::Arr =>
            {
                // For containers, we consider the width of the opening and closing brackets
                2 // for {} or []
            }
            RowType::Str | RowType::Num | RowType::Bit | RowType::Null =>
            {
                inline_width(row, table)
            }
        }
    }
}

mod parser
{
    use crate::lexer::{
        TokVal, TokenKind, classify_token, json_tokens_from_reader, token_value,
    };
    use std::fs::File;
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
        let reader = std::io::Cursor::new(src.as_bytes());
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
        let file = File::open(path)?;
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
        reader: R,
    ) -> io::Result<Vec<Row>>
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

            // TODO: Should not add root node, do not assume obj vs arr
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
                        String::new(),
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
                        String::new(),
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
    for r in rows
    {
        println!("{:?}", r);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
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
        let subnodes = parse_from_str(code).unwrap();
        println!("Subnodes: {:#?}", subnodes);
        assert_eq!(subnodes.len(), 8);
        assert_eq!(subnodes[0].val, "[");
        assert_eq!(subnodes[1].val, "1");
        assert_eq!(subnodes[2].val, "2");
        assert_eq!(subnodes[3].ty, RowType::Arr);
        assert_eq!(subnodes[3].key, "4"); // Array indices have numeric keys
        assert_eq!(subnodes[3].val, "[");
        assert_eq!(subnodes[4].val, "3");
        assert_eq!(subnodes[5].val, "4");
        assert_eq!(subnodes[6].val, "5");
        assert_eq!(subnodes[7].val, "6");

        let code = r#"[1, 2, [3, 4], 5, 6"#;
        assert!(parse_from_str(code).is_err());
    }
}
