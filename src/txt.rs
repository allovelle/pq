//! UTF-8 types and other supporting utilities

// // TODO: 1. Go get all lengths of input files
// // TODO: 2. Allocate a buffer of the total length
// // TODO: 3. Memory map all files into the buffer
// TODO: 4. Graft the buffer for stdin onto the buffer

use crate::cli::Cli;
use std::{collections::HashMap, io, ops::Range};

// TODO: This is where the new txt modules will enable full UTF-8 support with
// TODO: the ability to append incoming data from stdin. Therefore, they will
// TODO: need full ownwership of the input buffer

// TODO: This is where the new txt modules will enable full UTF-8 support with
// TODO: the ability to append incoming data from stdin. Therefore, they will
// TODO: need full ownwership of the input buffer

// TODO: This is where the new txt modules will enable full UTF-8 support with
// TODO: the ability to append incoming data from stdin. Therefore, they will
// TODO: need full ownwership of the input buffer

// ! This cannot handle UTF-8 decoding. This must be handled by another IO type
impl StaticFiles
{
    // TODO: Can you add new filenames as long as you don't snip into stdin?
    // TODO: IT would mean growable after initial CLI invocation, so I feel not.

    pub fn new() -> Self { Self::default() }

    // This is called multiple times, once per chunk from stdin. File IO is not
    // handled here.
    fn graft_stdin(&mut self, stdin_data: &[u8])
    {
        let stdin_offset = self.buffer.len() - stdin_data.len();
        self.buffer[stdin_offset ..].copy_from_slice(stdin_data);
        self.descriptors.push((stdin_offset, "<stdin>".to_string()));
    }

    fn lookup_filename(&self, index: usize) -> Option<&String>
    {
        // Intrinsically correct in unit case, in cases there are 1 pages and no
        // stdin, and in cases where there is only stdin and no files
        let mut last_offset = 0;

        for (offset, length, _filename) in &self.descriptors
        {
            if index > *offset
            {
                last_offset = *offset;
            }
            else
            {
                break;
            }
        }

        // Now have the last offset that is inside a file start offset and prior
        // to the next file start offset as it is strictly less than it.
        // This works for indexes inside any file as well as inside the stdin
        // region at the end of the buffer.

        self.descriptors.get(last_offset).map(|(_, filename)| filename)
    }

    // ! This is not meant to be used directly. This is used by UTF8 decoders to
    // ! iterate byte by byte
    fn get_byte_by_index(&self, index: usize) -> Option<u8>
    {
        self.buffer.get(index)
    }
}
