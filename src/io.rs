//! Takes N filenames from CLI, get's their file length, allocates that space to
//! a buffer, then concurrently reads them all at once into the buffer. Takes
//! STDIN, and writes bytes onto the end of the buffer for free.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug)]
pub struct StreamBuffer
{
    pub buffer: Vec<u8>,
    pub file_ranges: Vec<std::ops::Range<usize>>,
}

pub fn build_stream(paths: &[String]) -> io::Result<StreamBuffer>
{
    let mut file_sizes = Vec::with_capacity(paths.len());
    let mut total_size = 0usize;

    // Step 1: get file sizes
    for path in paths
    {
        let meta = std::fs::metadata(path)?;
        let size = meta.len() as usize;
        file_sizes.push(size);
        total_size += size;
    }

    // Step 2: allocate buffer large enough for files
    let mut buffer = vec![0u8; total_size];

    let mut offset = 0usize;
    let mut file_ranges = Vec::with_capacity(paths.len());

    // Step 3: read files into their slices
    for (path, size) in paths.iter().zip(file_sizes.iter())
    {
        let mut file = File::open(Path::new(path))?;

        let start = offset;
        let end = offset + size;

        file.read_exact(&mut buffer[start .. end])?;

        file_ranges.push(start .. end);
        offset = end;
    }

    // Step 4: append stdin after files
    let mut stdin = io::stdin();
    stdin.read_to_end(&mut buffer)?;

    Ok(StreamBuffer { buffer, file_ranges })
}

fn main_io() -> std::io::Result<()>
{
    let args: Vec<String> = std::env::args().skip(1).collect();

    let stream = build_stream(&args)?;

    println!("Total buffer size: {}", stream.buffer.len());

    Ok(())
}

// Reserve prior amount
// Retain items live
// Release items live or at completion

enum Use
{
    /// Retain system inputs into [Vec]
    In,
    /// Retain system outputs into [Vec]
    Ou,
    /// Retain system inputs and outputs into two [Vec]s.
    Io,
    /// Release system inputs and outpus (iterator passthrough).
    Fw,
}

// Accumulate but do not use the byte buffer until EoF. Forward whole characters
// You can't slice the buffer as it's accumulating (due to reallocations) yet
// you can yield whole characters by storing a [u8; 4].

/// Inputs are either consumed or retained
enum Input<'io, T>
{
    Iterator(Box<dyn Iterator<Item = T> + 'io>),
    Buffer(&'io [T]),
}

impl<'io, T: Copy> Input<'io, T>
{
    fn next(&mut self, pos: &mut usize) -> Option<T>
    {
        match self
        {
            Input::Iterator(iter) => iter.next(),
            Input::Buffer(buf) =>
            {
                if *pos < buf.len()
                {
                    let item = buf[*pos];
                    *pos += 1;
                    Some(item)
                }
                else
                {
                    None
                }
            }
        }
    }
}

struct Utf8Buf<'me>
{
    input: Input<'me, u8>,

    /// When retained, it allows string slices to reference the input stream.
    /// Allows the input stream to close without losing access to input bytes.
    retained_ins: Vec<u8>,
    /// When retained, it allows string slices to be referenced downstream,
    /// even when inputs are not retained.
    retained_ous: Vec<char>,
}

impl<'me> Utf8Buf<'me>
{
    pub fn new(input: Input<'me, u8>) -> Self
    {
        Self {
            input,
            retained_ins: Default::default(),
            retained_ous: Default::default(),
        }
    }

    fn example_so_far(&mut self)
    {
        let mut pos = 0;

        while let Some(x) = self.input.next(&mut pos)
        {
            self.retained_ins.push(x);
            let _ = x;
        }
    }
}

fn io_main()
{
    let buffer = vec![0];
    let input = Input::Buffer(&buffer);
    let mut utf8_buf = Utf8Buf::new(input);
    utf8_buf.example_so_far();
}
