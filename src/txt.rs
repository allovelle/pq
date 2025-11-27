/// Returns the byte index of the codepoint *after* the one provided.
pub const fn utf8_byte_udx_after(buffer: &[u8], udx: usize) -> Option<usize>
{
    if let Some(ch) = utf8_char_on(buffer, udx)
    {
        let stride = ch.len_utf8();
        let start = udx + stride - 1;
        return Some(udx + start);
    }
    None
}

/// Returns a [char] for a specific byte index. If the byte index is not on a
/// valid codepoint boundary (specifically the start byte of that codepoint),
/// [Option::None] is returned.
pub const fn utf8_char_on(buffer: &[u8], udx: usize) -> Option<char>
{
    const UTF8_MASKS: [[u8; 4]; 4] = [
        [0b1000_0000, 0, 0, 0b1111_1111],
        [0b1110_0000, 0b1100_0000, 1, 0b0001_1111],
        [0b1111_0000, 0b1110_0000, 2, 0b0000_1111],
        [0b1111_1000, 0b1111_0000, 3, 0b0000_0111],
    ];

    let byte0 = if udx < buffer.len() { buffer[udx] } else { 0 };

    let mut udx_checker = 0;
    while udx_checker < UTF8_MASKS.len()
    {
        let [mask, valid_mask, additional_len, strip] = UTF8_MASKS[udx_checker];
        udx_checker += 1;

        if udx + additional_len as usize >= buffer.len()
        {
            return None;
        }

        if byte0 & mask == valid_mask
        {
            let byte1 = if udx < buffer.len() { buffer[udx + 1] } else { 0 };
            let byte2 = if udx < buffer.len() { buffer[udx + 2] } else { 0 };
            let byte3 = if udx < buffer.len() { buffer[udx + 3] } else { 0 };

            let bytes = [byte0, byte1, byte2, byte3];
            let strip_masks = [strip, 0b0011_1111, 0b0011_1111, 0b0011_1111];
            let mut codepoint: u32 = 0;

            let mut udx_i = 0usize;
            while udx_i < additional_len as usize + 1
            {
                let shift = 6 * (additional_len - udx_i as u8);
                let stripped_byte = bytes[udx_i] & strip_masks[udx_i];
                codepoint |= (stripped_byte as u32) << (shift as u32);
                udx_i += 1;
            }

            return std::char::from_u32(codepoint);
        }
    }

    None
}
