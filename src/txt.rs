//! Buffer push_bytes &mut and maintain cursor of end of bytes len and end of
//! chars len. Append only means callers can slice by usize/u32 and always
//! return valid string slices or byte slice that do not expire when the buffer
//! is reallocated.

// Make one buffer for reading all filenames from CLI all at once.

// Make another buffer for piecemeal reading bytes from STDIN.

// Referencing (&str) the buffer via a usize/u32 requires Utf8::retain = true

// The buffer could accumulate or relase bytes but forward chars only
// The lexer could accumulate or release chars but forward toks only

// You can't reference the buffer to string slice unless retain = true.
// Otherwise, chars are forwarded, bytes are discarded, so there's no way to
// look back to slice them out. Forwarding chars or accumulating forwarded chars
// is another configuration that could help the pipeline make sense.

pub struct Utf8Buf
{
    buffer: Vec<u8>,
    char_len: usize,
}

impl Utf8Buf
{
    pub fn push_bytes(&mut self, bytes: &[u8])
    {
        self.buffer.extend(bytes);
    }

    pub fn slice(&self, start: usize) -> &str
    {
        &self.buffer[..].str
    }

    pub fn char_len(&self) -> usize
    {
        self.char_len
    }

    pub fn byte_len(&self) -> usize
    {
        self.buffer.len()
    }
}
