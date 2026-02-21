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
