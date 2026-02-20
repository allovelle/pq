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

use crate::codepoints::Utf8Codepoints;

// TODO: Streaming IO (string buffer) -> Utf8 Codepoint Iter (character stream)
// TODO:    -> Tokens (arr/obj/com/col) -> Token Values (txt, num) -> Rows
// TODO:    -> (id, par, key, val, ty)
