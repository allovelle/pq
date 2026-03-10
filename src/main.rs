/// [Text] is a lifetime-erased string slice: &[str] so that it always works to
/// slice the [Utf8Buf] as it is append only.
/// Like &[str] but for backing stores that can be reallocated yet remaind valid
/// for the stored range as the backing store is append only.
/// Invariant: Backing store must be append only and allow logical offsets.
pub struct Text
{
    pub from: u32,
    pub to: u32,
}

/// [Utf8Buf] stores the bytes, but someone has to index and slice out strings
pub struct Utf8Buf
{
    buffer: Vec<u8>,
}

/// [Tok]s point into the [Utf8Buf], but someone has to reiterate to get chars.
/// Stores the start byte of a token slice from the file
struct Tok(u32);

// END OF APPLICATION LEVEL TYPES

// BEGIN INFRASTRUCTURE LEVEL TYPES

impl Utf8Buf
{
    fn slice_text(Text { from, to }: Text) {}

    fn slice_tuple((from, to): (u32, u32)) -> () {}

    fn reiterate(starting_from: u32) {}
}

struct Reiterator {}

fn main() {}

// BEGIN IMPLEMENTATIONS

fn push_bytes(buf: &mut Utf8Buf, bytes: &[u8])
{
    buf.buffer.extend_from_slice(bytes);
}

fn slice_bytes(buf: &mut Utf8Buf, from: u32, to: u32) -> Text
{
    Text { from: 0, to: 0 }
}
