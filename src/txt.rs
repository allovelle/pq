//! UTF-8 types and other supporting utilities

// TODO: 1. Go get all lengths of input files
// TODO: 2. Allocate a buffer of the total length
// TODO: 3. Memory map all files into the buffer
// TODO: 4. Graft the buffer for stdin onto the buffer

use crate::cli::Cli;

pub fn lengths(cli: &Cli) -> (usize, Vec<usize>)
{
    let mut total_len = 0;
    let mut lengths = Vec::new();
    for file in &cli.files
    {
        let file_len = std::fs::metadata(file).unwrap().len() as usize;
        total_len += file_len;
        lengths.push(file_len);
    }
    (total_len, lengths)
}

// ! MMap enforces only 4kb page sizes so there will be massive gaps for small
// ! files. This means mmap files and mmap memory pages are differentiated and
// ! and do cannot be mixed by preallocation & subsequently filling memory.
pub fn mmap_space(total_len: usize) -> Box<[u8]> {}
