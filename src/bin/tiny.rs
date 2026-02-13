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
        pub id: u32,     // index within parent container
        pub par: u32,    // parent row ID
        pub key: String, // empty for array elements
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
            /* table
            .iter()
            .filter(|r| r.id != 0 && self.id == r.par && r.id > self.id) */
            table.iter().filter(|r| r.id > self.id && self.id == r.par)
        }
    }

    /// Parser state for building the row table incrementally.
    pub struct Parser<'a, R: Read>
    {
        src: &'a str,
        tokens: crate::lexer::JsonTokens<R>,
        rows: Vec<Row>,
        stack: Vec<u32>,        // parent row IDs
        stack_ty: Vec<RowType>, // parent types
        next_row_id: u32,
        next_index_in_parent: Vec<u32>,
    }

    impl<'a, R: Read> Parser<'a, R>
    {
        pub fn new(src: &'a str, reader: R) -> Self
        {
            Self {
                src,
                tokens: json_tokens_from_reader(reader),
                rows: Vec::new(),
                stack: Vec::new(),
                stack_ty: Vec::new(),
                next_row_id: 0,
                next_index_in_parent: Vec::new(),
            }
        }

        fn push_container(&mut self, ty: RowType, key: String)
        {
            let par =
                self.stack.last().copied().unwrap_or(0 /* u32::max */);
            let id = self.next_row_id;
            self.next_row_id += 1;

            // let idx = if let Some(last) = self.next_index_in_parent.last_mut()
            // {
            //     let v = *last;
            //     *last += 1;
            //     v
            // }
            // else
            // {
            //     0
            // };
            let idx = self.rows.len() as u32;

            let val = match ty
            {
                RowType::Obj => "{".to_string(),
                RowType::Arr => "[".to_string(),
                _ => String::new(),
            };

            self.rows.push(Row { id: idx, par, key, val, ty });

            self.stack.push(id);
            self.stack_ty.push(ty); // NEW
            self.next_index_in_parent.push(0);
        }

        fn pop_container(&mut self)
        {
            self.stack.pop();
            self.stack_ty.pop();
            self.next_index_in_parent.pop();
        }

        fn add_value(&mut self, key: String, val: TokVal)
        {
            let par =
                self.stack.last().copied().unwrap_or(0 /* u32::max */);
            let id = self.next_row_id;
            self.next_row_id += 1;

            // let idx = if let Some(last) = self.next_index_in_parent.last_mut()
            // {
            //     let v = *last;
            //     *last += 1;
            //     v
            // }
            // else
            // {
            //     0
            // };
            let idx = self.rows.len() as u32;

            // O(1) parent type lookup
            let parent_ty = self.stack_ty.last().copied();

            // Arrays override the key with the index
            let final_key = match parent_ty
            {
                Some(RowType::Arr) => idx.to_string(),
                _ => key,
            };

            let (ty, val_str) = match val
            {
                TokVal::Txt(s) => (RowType::Str, s.to_string()),
                TokVal::Num(s) => (RowType::Num, s.to_string()),
                TokVal::Bit(b) =>
                {
                    (RowType::Bit, if b { "true" } else { "false" }.to_string())
                }
                TokVal::Nil => (RowType::Null, "null".to_string()),
                _ => (RowType::Null, String::new()),
            };

            self.rows.push(Row {
                id: idx,
                par,
                key: final_key,
                val: val_str,
                ty,
            });
        }

        pub fn parse(mut self) -> io::Result<Vec<Row>>
        {
            let mut pending_key: Option<String> = None;

            while let Some(tok) = self.tokens.next()
            {
                let start = tok?;
                let kind = classify_token(self.src, start);

                match kind
                {
                    TokenKind::LBrace =>
                    {
                        let key = pending_key.take().unwrap_or_default();
                        self.push_container(RowType::Obj, key);
                    }
                    TokenKind::LBracket =>
                    {
                        let key = pending_key.take().unwrap_or_default();
                        self.push_container(RowType::Arr, key);
                    }
                    TokenKind::RBrace | TokenKind::RBracket =>
                    {
                        self.pop_container();
                    }
                    TokenKind::String =>
                    {
                        let val = token_value(self.src, start);
                        match val
                        {
                            TokVal::Txt(s) =>
                            {
                                if pending_key.is_none()
                                {
                                    pending_key = Some(s.to_string());
                                }
                                else
                                {
                                    let key = pending_key.take().unwrap();
                                    self.add_value(key, TokVal::Txt(s));
                                }
                            }
                            _ =>
                            {}
                        }
                    }
                    TokenKind::Number =>
                    {
                        let key = pending_key.take().unwrap_or_default();
                        let val = token_value(self.src, start);
                        self.add_value(key, val);
                    }
                    TokenKind::True | TokenKind::False | TokenKind::Null =>
                    {
                        let key = pending_key.take().unwrap_or_default();
                        let val = token_value(self.src, start);
                        self.add_value(key, val);
                    }
                    TokenKind::Colon | TokenKind::Comma =>
                    {}
                    TokenKind::Unknown =>
                    {}
                }
            }

            Ok(self.rows)
        }
    }

    pub fn parse_from_str(src: &str) -> io::Result<Vec<Row>>
    {
        let reader = std::io::Cursor::new(src.as_bytes());
        Parser::new(src, reader).parse()
    }

    pub fn parse_from_file(path: &str) -> io::Result<Vec<Row>>
    {
        let src = std::fs::read_to_string(path)?;
        let file = File::open(path)?;
        Parser::new(&src, file).parse()
    }

    pub fn parse_from_reader<R: Read>(
        src: &str,
        reader: R,
    ) -> io::Result<Vec<Row>>
    {
        Parser::new(src, reader).parse()
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
    #[test]
    fn test_subnodes()
    {
        let code = r#"[1, 2, [3, 4], 5, 6"#;
        assert!(parse_from_str(code).is_err());
    }
}
