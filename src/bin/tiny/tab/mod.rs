mod kvbuf;

use std::io;

use crate::lexer::{json_tokens_from_reader, TokenKind};
use kvbuf::KeyValBuf;
pub use kvbuf::TextOffset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[rustfmt::skip]
pub enum RowType {
    Obj, Arr, Str, Num, Bit, Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row
{
    pub par: u32,
    /// Document root in the table. Multiple roots are JSON Lines documents.
    pub root: u32,
    pub key: TextOffset,
    pub val: TextOffset,
}

/// Invariant: string values keep the opening quote in source. Keys are always
/// the inner string (without quotes) when interned, and source string token
/// offsets when parsed from object keys.
pub fn row_type(source: &str, row: Row) -> RowType
{
    let first_byte = source.as_bytes()[row.val as usize];
    match first_byte
    {
        b'"' => RowType::Str,
        b'{' | b'}' => RowType::Obj,
        b'[' | b']' => RowType::Arr,
        b'-' | b'0' ..= b'9' => RowType::Num,
        b't' | b'f' => RowType::Bit,
        b'n' => RowType::Nil,
        _ => RowType::Nil,
    }
}

#[derive(Debug, Clone, Copy)]
struct ContainerInfo
{
    row_id: u32,
    child_count: u32,
}

/// Append-only JSON table backed by source offsets.
pub struct JsonTab
{
    rows: Vec<Row>,
    kv: KeyValBuf,
}

impl JsonTab
{
    pub fn parse_from_str(source: &str) -> io::Result<Self>
    {
        let mut parser = Parser::new(source);
        parser.parse()?;
        Ok(Self { rows: parser.rows, kv: parser.kv })
    }

    #[inline]
    pub fn rows(&self) -> &[Row]
    {
        &self.rows
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.rows.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.rows.is_empty()
    }

    #[inline]
    pub fn source(&self) -> &str
    {
        self.kv.source()
    }

    #[inline]
    pub fn row(&self, id: u32) -> Option<Row>
    {
        self.rows.get(id as usize).copied()
    }

    #[inline]
    pub fn row_type(&self, id: u32) -> Option<RowType>
    {
        self.row(id).map(|row| row_type(self.kv.source(), row))
    }

    #[inline]
    pub fn key(&self, id: u32) -> Option<&str>
    {
        self.row(id).map(|row| self.kv.text_value(row.key))
    }

    #[inline]
    pub fn val(&self, id: u32) -> Option<&str>
    {
        self.row(id).map(|row| self.kv.text_value(row.val))
    }

    #[inline]
    pub fn raw_val(&self, id: u32) -> Option<&str>
    {
        self.row(id).map(|row| self.kv.token_slice(row.val))
    }

    pub fn first_child(&self, id: u32) -> Option<u32>
    {
        let child = id + 1;
        let row = self.rows.get(child as usize)?;
        (row.par == id).then_some(child)
    }

    pub fn next_sibling(&self, id: u32) -> Option<u32>
    {
        let row = *self.rows.get(id as usize)?;
        let next = id + 1;
        let next_row = self.rows.get(next as usize)?;
        (next_row.par == row.par).then_some(next)
    }

    pub fn parent(&self, id: u32) -> Option<u32>
    {
        let row = *self.rows.get(id as usize)?;
        (row.par != id).then_some(row.par)
    }

    /// Mount a copy of an entire subtree onto the end of the table.
    pub fn graft(&mut self, base: u32) -> Option<u32>
    {
        let base_idx = base as usize;
        if base_idx >= self.rows.len()
        {
            return None;
        }

        let mut end = base_idx + 1;
        while end < self.rows.len()
            && self.rows[end].par >= base
            && self.rows[end].par != end as u32
        {
            end += 1;
        }

        let new_base = self.rows.len() as u32;
        let offset = new_base - base;

        self.rows.reserve(end - base_idx);
        for i in base_idx .. end
        {
            let src = self.rows[i];
            let new_id = i as u32 + offset;
            let new_par = if i == base_idx { new_id } else { src.par + offset };
            let new_root = src.root + offset;

            self.rows.push(Row {
                par: new_par,
                root: new_root,
                key: src.key,
                val: src.val,
            });
        }

        Some(new_base)
    }
}

struct Parser<'src>
{
    src: &'src str,
    tokens: Vec<TextOffset>,
    token_idx: usize,
    rows: Vec<Row>,
    current_parent: u32,
    current_root: u32,
    stack: Vec<ContainerInfo>,
    kv: KeyValBuf,
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
            current_root: 0,
            stack: Vec::new(),
            kv: KeyValBuf::from_source(src),
        }
    }

    fn parse(&mut self) -> io::Result<()>
    {
        let iter =
            json_tokens_from_reader(std::io::Cursor::new(self.src.as_bytes()));
        for result in iter
        {
            self.tokens.push(result? as TextOffset);
        }

        if self.tokens.is_empty()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "empty input",
            ));
        }

        self.parse_value(None)?;
        Ok(())
    }

    fn peek(&self) -> Option<(TextOffset, TokenKind)>
    {
        let at = *self.tokens.get(self.token_idx)?;
        Some((at, self.kv.classify(at)))
    }

    fn next(&mut self) -> Option<(TextOffset, TokenKind)>
    {
        let next = self.peek()?;
        self.token_idx += 1;
        Some(next)
    }

    fn expect(&mut self, expected: TokenKind) -> io::Result<TextOffset>
    {
        match self.next()
        {
            Some((at, kind)) if kind == expected => Ok(at),
            Some((at, kind)) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected token at {at}: expected {expected:?}, got {kind:?}"),
            )),
            None => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("expected {expected:?}"),
            )),
        }
    }

    fn accept(&mut self, expected: TokenKind) -> bool
    {
        if let Some((_, kind)) = self.peek()
        {
            if kind == expected
            {
                self.token_idx += 1;
                return true;
            }
        }
        false
    }

    fn add_row(&mut self, key: TextOffset, val: TextOffset) -> u32
    {
        let id = self.rows.len() as u32;
        self.rows.push(Row {
            par: self.current_parent,
            root: self.current_root,
            key,
            val,
        });
        id
    }

    fn parse_value(&mut self, key: Option<TextOffset>) -> io::Result<()>
    {
        let (at, kind) = self.peek().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected JSON value")
        })?;

        match kind
        {
            TokenKind::LBrace => self.parse_object(key),
            TokenKind::LBracket => self.parse_array(key),
            TokenKind::String
            | TokenKind::Number
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null =>
            {
                self.next();
                let key = key.unwrap_or_else(|| self.kv.intern(""));
                self.add_row(key, at);
                if let Some(info) = self.stack.last_mut()
                {
                    info.child_count += 1;
                }
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected token kind: {kind:?}"),
            )),
        }
    }

    fn parse_object(&mut self, key: Option<TextOffset>) -> io::Result<()>
    {
        let open = self.expect(TokenKind::LBrace)?;
        let key = key.unwrap_or_else(|| self.kv.intern(""));
        let obj_id = self.add_row(key, open);

        if self.rows.len() == 1
        {
            self.current_root = obj_id;
            self.rows[obj_id as usize].root = obj_id;
            self.rows[obj_id as usize].par = obj_id;
        }

        let saved_parent = self.current_parent;
        self.current_parent = obj_id;
        self.stack.push(ContainerInfo { row_id: obj_id, child_count: 0 });

        if !self.accept(TokenKind::RBrace)
        {
            loop
            {
                let key_at = self.expect(TokenKind::String)?;
                self.expect(TokenKind::Colon)?;
                self.parse_value(Some(key_at))?;

                if self.accept(TokenKind::Comma)
                {
                    continue;
                }
                if self.accept(TokenKind::RBrace)
                {
                    break;
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected comma or object close",
                ));
            }
        }

        self.stack.pop();
        self.current_parent = saved_parent;
        if let Some(info) = self.stack.last_mut()
        {
            if info.row_id != obj_id
            {
                info.child_count += 1;
            }
        }
        Ok(())
    }

    fn parse_array(&mut self, key: Option<TextOffset>) -> io::Result<()>
    {
        let open = self.expect(TokenKind::LBracket)?;
        let key = key.unwrap_or_else(|| self.kv.intern(""));
        let arr_id = self.add_row(key, open);

        if self.rows.len() == 1
        {
            self.current_root = arr_id;
            self.rows[arr_id as usize].root = arr_id;
            self.rows[arr_id as usize].par = arr_id;
        }

        let saved_parent = self.current_parent;
        self.current_parent = arr_id;
        self.stack.push(ContainerInfo { row_id: arr_id, child_count: 0 });

        if !self.accept(TokenKind::RBracket)
        {
            loop
            {
                let idx =
                    self.stack.last().map(|info| info.child_count).unwrap_or(0);
                let key = self.kv.intern(&idx.to_string());
                self.parse_value(Some(key))?;

                if self.accept(TokenKind::Comma)
                {
                    continue;
                }
                if self.accept(TokenKind::RBracket)
                {
                    break;
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected comma or array close",
                ));
            }
        }

        self.stack.pop();
        self.current_parent = saved_parent;
        if let Some(info) = self.stack.last_mut()
        {
            if info.row_id != arr_id
            {
                info.child_count += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::{JsonTab, RowType};

    #[test]
    fn parses_and_resolves_offsets()
    {
        let tab = JsonTab::parse_from_str(r#"{"k":"v","arr":[1]}"#).unwrap();
        assert_eq!(tab.len(), 4);
        assert_eq!(tab.row_type(0), Some(RowType::Obj));
        assert_eq!(tab.key(1), Some("k"));
        assert_eq!(tab.val(1), Some("v"));
        assert_eq!(tab.key(2), Some("arr"));
        assert_eq!(tab.row_type(3), Some(RowType::Num));
        assert_eq!(tab.key(3), Some("0"));
        assert_eq!(tab.val(3), Some("1"));
    }
}
