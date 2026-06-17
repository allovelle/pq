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

pub type InputFilesBuffer = Box<[u8]>;

pub struct InputFilesDescriptor
{
    pub offsets: Vec<Range<usize>>,
    pub lengths: Vec<usize>,
    pub files: Vec<String>,
}

// TODO: Pull in one big buffer from cli input files
pub fn load_input_files(cli: &Cli) -> io::Result<InputFiles>
{
    let (total_len, lengths) = lengths(cli);
    eprintln!("total_len={total_len}, lengths={:?}", lengths);

    txtmmap::mmap_space(&cli.files)
}

mod txtmmap
{
    use memmap2::Mmap;
    use std::{fs::File, io, ops::Range, path::Path};

    /// Main container: one contiguous buffer + offsets into it
    pub struct InputFiles
    {
        pub data: Box<[u8]>,
        pub offsets: Vec<Range<usize>>,
    }

    /// Step 1: open files and compute total size
    pub fn compute_total_size<P: AsRef<Path>>(
        paths: &[P],
    ) -> io::Result<(usize, Vec<(File, usize)>)>
    {
        let mut total = 0usize;
        let mut files = Vec::with_capacity(paths.len());

        for path in paths
        {
            let file = File::open(path)?;
            let len = file.metadata()?.len() as usize;

            total += len;
            files.push((file, len));
        }

        Ok((total, files))
    }

    /// Step 2: mmap each file and copy into one contiguous buffer
    pub fn load_files_contiguous(
        files: Vec<(File, usize)>,
        total_size: usize,
    ) -> io::Result<Box<[u8]>>
    {
        let mut buffer = vec![0u8; total_size].into_boxed_slice();

        let mut offset = 0usize;

        for (file, len) in files
        {
            let mmap = unsafe { Mmap::map(&file)? };

            buffer[offset .. offset + len].copy_from_slice(&mmap);
            offset += len;
        }

        Ok(buffer)
    }

    /// Step 3: high-level constructor
    pub fn mmap_space<P: AsRef<Path>>(paths: &[P]) -> io::Result<InputFiles>
    {
        let (total_size, files) = compute_total_size(paths)?;

        // build offset table
        let mut offsets = Vec::with_capacity(paths.len());
        let mut cursor = 0usize;

        for (_, len) in &files
        {
            offsets.push(cursor .. cursor + *len);
            cursor += *len;
        }

        // build contiguous buffer
        let data = load_files_contiguous(files, total_size)?;

        Ok(InputFiles { data, offsets })
    }

    impl InputFiles
    {
        /// Random access to a file slice
        pub fn get(&self, index: usize) -> &[u8]
        {
            let range = &self.offsets[index];
            &self.data[range.clone()]
        }

        /// Iterator over file slices (zero-copy)
        pub fn iter(&self) -> InputFilesIter<'_>
        {
            InputFilesIter {
                data: &self.data,
                offsets: &self.offsets,
                index: 0,
            }
        }
    }

    impl InputFiles
    {
        /// Get a single byte from the global contiguous buffer
        pub fn byte(&self, index: usize) -> Option<u8>
        {
            self.data.get(index).copied()
        }

        /// Get a slice from the global contiguous buffer
        pub fn slice(&self, range: Range<usize>) -> Option<&[u8]>
        {
            self.data.get(range)
        }

        /// Unsafe fast version (no bounds checks)
        pub fn byte_unchecked(&self, index: usize) -> u8 { self.data[index] }

        /// Unsafe fast slice (no bounds checks)
        pub fn slice_unchecked(&self, range: Range<usize>) -> &[u8]
        {
            &self.data[range]
        }

        /// Total size of the entire virtual region
        pub fn len(&self) -> usize { self.data.len() }

        pub fn is_empty(&self) -> bool { self.data.is_empty() }

        pub fn as_slice(&self) -> &[u8] { &self.data }
    }

    /// Iterator implementation
    pub struct InputFilesIter<'a>
    {
        data: &'a [u8],
        offsets: &'a [Range<usize>],
        index: usize,
    }

    impl<'a> Iterator for InputFilesIter<'a>
    {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<Self::Item>
        {
            if self.index >= self.offsets.len()
            {
                return None;
            }

            let range = &self.offsets[self.index];
            self.index += 1;

            Some(&self.data[range.clone()])
        }
    }

    use std::ops::{Index, IndexRange, Range};

    impl Index<usize> for InputFiles
    {
        type Output = u8;

        fn index(&self, index: usize) -> &Self::Output { &self.data[index] }
    }
}

// ! MMap enforces only 4kb page sizes so there will be massive gaps for small
// ! files. This means mmap files and mmap memory pages are differentiated and
// ! and cannot be mixed by preallocation & subsequently filling memory.
// ! The main advantage of using mmap is that it allows for near-infinite memory
// ! allocation since they can be swapped out to disk.
pub fn mmap_space(total_len: usize) -> Box<[u8]>
{
    // Maintain a list of pages, each of 4KB in size, where each page is a
    // Box<[u8]> of size 4KB. The total number of pages is total_len / 4096 + 1.

    // Each file is mapped into a block of pages, yet can have empty bytes left
    // over.

    // Alloating a block of contiguous pages is different than mmaping files.

    // Stdin: mmap block of pages and allocate more as needed
    // Files: mmap each file into a block of pages, maintaining a
    // `(page, length)` where page is page index and length is the end of the
    // page in the page block where the file ends.

    // Must create page map type where page refs are tracked and stdio pages are
    // appended, and where iteration over the bytes is done and random access is
    // supported by using increments over the files.

    // The files page maps are static.
    // The stdin page map is dynamic and can be appended onto as needed.

    // CLI file mmaps are done all at once (possibly concurrently), and stdin
    // is mapped incrementally.

    // Concurrent cli file mmaps can be done concurrently because the length of
    // each file is known in advance and any differenes are modulated over 4096.

    struct InputFiles
    {
        files: Vec<(usize, usize)>, // (page, length)
        stdin: Vec<(usize, usize)>, // (page, length)
    }

    impl InputFiles
    {
        fn index(&self, offset: usize) -> (usize, usize)
        {
            // Returns (page, length) for the given offset
            for (page, length) in &self.files
            {
                if offset < *length
                {
                    return (*page, offset);
                }
            }
            for (page, length) in &self.stdin
            {
                if offset < *length
                {
                    return (*page, offset);
                }
            }
            panic!("Offset out of bounds");
        }
    }
}

// TODO: How to exactly mmap a file into a specific location in the buffer?

/// Returns all bytes from all files from cli. Returns a Vec<u8> so that the
/// stdin can be appended onto it.
fn a0(cli: &Cli) -> Vec<u8>
{
    let total_length = lengths(cli).0;
    let mut buffer = Vec::with_capacity(total_length);
    buffer.resize(total_length, 0);

    for file in &cli.files
    {
        // let file_bytes = std::fs::read(file).unwrap();
        // buffer.extend_from_slice(&file_bytes);

        // TODO: Mmap the file into the buffer, maintaining an offset increment
        mmap_space(total_length);
    }

    buffer
}
