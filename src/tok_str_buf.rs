// ! TODO: Token handles are source byte offsets resolved by Utf8Iter.
// ! TODO: No need for off+len because toks are recomputed every dereference.
// ! TODO: There's no need for null termination because it's not a
// ! TODO: STRING Interner, it's a TOKEN VALUE Interner
// ! TODO: ^^^ been searching a long time for a satisfying design foro this

// TODO: Append strings to buf[u8]. Hand out u32 offset token handles. Deref
// TODO: token handles into token value (which denotes type automatically free)

use crate::{TokVal, TokenKind};

type TokId = u32;

fn append_string(_buf: &mut Vec<u8>, _text: &[u8]) {}

fn handle_type(_buf: &mut Vec<u8>, _handle: TokId) -> TokenKind
{
    TokenKind::Null
}

fn ref_handle(_buf: &mut Vec<u8>, _offset: usize) -> TokId
{
    0
}

fn deref_handle(_buf: &mut Vec<u8>, _handle: TokId) -> TokVal<'_>
{
    TokVal::Txt("hello")
}

/*
Tokens are just indexes into an array of bytes. This allows simultaneous insert
and deref from the input buffer.

Utf8Iter is used to iter over characters safely directly from an offset.

Tokenization begins at arbitrary source point to return a token value.

During tokenization, only token handles are returned to keep things light. Most
tokens don't even have a value (new-obj, new-arr, bit, nil, com, col). Only txt
and num need them.

This is zero copy: log(n) token index creation.

src: &Vec<u8>
tok: 0
val: new-arr
*/

// TODO: Streaming IO (string buffer) -> Utf8 Codepoint Iter (character stream)
// TODO:    -> Tokens (arr/obj/com/col) -> Token Values (txt, num) -> Rows
// TODO:    -> (id, par, key, val, ty)

// # Tiny Version of the Pique JSON Formatter

// **Formatting Commands**:

// - Wrap
// - Spacer
// - Minify
// - Newline
// - Indent(udx)

// These are pushed on a stack and indents look at the stack to determine the start
// of their indentation for example. So the structure of the JSON determines the
// structure of the formatting commands.
