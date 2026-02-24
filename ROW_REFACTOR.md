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
