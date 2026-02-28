mod kvbuf;

// * DONE: this design will work for pq's use case:
// ? Remove token type u8 and classify from source?
// ? classify_row works for all types except strings:
// ? Store string types with leading `"` only?
// ? Json Keys are always strings so do *not* store leading `"`
// ? This would require: deref txt handle, then read first byte from src for ty
// ? Math:
// ? All rows are key + val. All keys are strings. All vals are stored as
// ? strings and can also be strings. All value string types are stored with
// ? leading `"` to allow them to be identified apart from the others since the
// ? others are all stored as strings anyway. Key strs do not need `"`.
// ? The usage of the string interner need not be tied to the interner impl. The
// ? leading quote invariant must be documented, especially since the key does
// ? not use this format (the str interner doesn't care either way).
// ? 5000 tokenzied rows = 1000 strings = 5000 keys and 5000 vals. This means
// ? 5000 bytes for storing token type, OR max 5000 bytes if all vals are txts.
// ? What is the compute cost for this extra indirection? Token type = 1 byte
// ? for all rows. Leading quote = 1 byte for all rows with string values. If
// ? all values are strs, it's the same cost as storing row type. 🫳🏻🎤

// ? Do not store self.id since all tranversals use loops anyway?
// ? Do not store document root since it's a guaranteed parent anyway

/// Invariant: This cannot classify strings unless the `"` are stored for each.
pub fn row_type(row: Row) -> RowType
{
    let first_byte = row.val.as_bytes()[0];
    match first_byte
    {
        // ! INVARIANT: only txt row vals start w/`"`, not any other val or keys
        b'"' => RowType::Str,
        b'{' | b'}' => RowType::Obj,
        b'[' | b']' => RowType::Arr,
        b'-' | b'0' ..= b'9' => RowType::Num,
        b't' => RowType::Bit,
        b'f' => RowType::Bit,
        b'n' => RowType::Nil,
        _ => RowType::Nil,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[rustfmt::skip]
pub enum RowType {
    Obj, Arr, Str, Num, Bit, Nil,
}

#[derive(Debug, Clone)]
pub struct Row
{
    pub id: u32,
    pub par: u32,
    /// Document root in the table. Multiple roots are JSON Lines documents.
    pub root: u32,
    pub key: String,
    pub val: String,
    pub ty: RowType,
}
