# Major Refactor Plan

1. Create reliable string interning type
2. Create row type that uses 1 u32 for key udx string handle, with 1 offset
    u12 with relative udx to the key udx (set val is key - initial val udx) and
    (get val is key + relative val)
3. Refactor parser.rs to use the new row type with string interning
4. Implement one query using the new table format


1. Use dedicated Row type for the tree table parsing that doesn't require
    the <T> AND
2. Create row type that the parser can use that utilizes the txt buffer for
    strings
3. Create string interning type that hands out byte index offsets instead
    of references because offsets can be 32-bit instead of usize 64-bit

# Pipeline Table Row Type

u4 json token type
u12 json val offset  << these are interned string indexes, what's the range?
u16 json key offset


# String Interner

Hands out indexes so that reallocation of the buffer does not break existing
handles.

The buffer maintains partial UTF-8 codepoint bytes, so a custom iterator allows
a handle start index to iterate to the end of the valid UTF-8 data. The buffer
maintains an index that is the end of the valid range of text. This possibly
invalid range cuts off a max of 3 bytes before becoming a valid codepoint.

The end-valid-range index also allows appending bytes without allowing access to
those bytes, such as when buffering in multiple batches of bytes.

A handle can be exchanged for a string slice.
Where is the start and end of the string slice?

The buffer is a Vec<u8> so that it can grow as needed.

How to allow range calculations when tokenizing to allow handle exchanges?

Append \0?
Allow partial UTF-8 bytes?
