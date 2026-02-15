use crate::lexer::{
    TokVal, TokenKind, classify_token, json_tokens_from_reader, token_value,
};
use std::io;
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
        offset: usize, expected: String, got: TokenKind
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
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    })
}

pub fn parse_from_file(path: &str) -> io::Result<Vec<Row>>
{
    let src = std::fs::read_to_string(path)?;
    Parser::new(&src).parse().map_err(|e| match e
    {
        ParseError::Io(io_err) => io_err,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
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
        let token_iter =
            json_tokens_from_reader(std::io::Cursor::new(self.src.as_bytes()));
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

    fn expect_token(&mut self, expected: TokenKind)
    -> Result<usize, ParseError>
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
                        message: "Expected string key in object".to_string(),
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
                                expected: "comma or closing brace".to_string(),
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
                let arr_idx = if let Some(info) = self.container_stack.last()
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

    fn parse_array_element(&mut self, index: u32) -> Result<(), ParseError>
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

    fn extract_value_string(&self, offset: usize, kind: TokenKind) -> String
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
