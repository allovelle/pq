mod err;
mod lex;
mod utf8;

use err::PqErr;
use lex::{Lex, Tokenizer, tok_ty_val};
use std::{env, fs, io::Read};
use utf8::{Utf8Buf, Utf8Iter};

// ── RawStdin ──────────────────────────────────────────────────────────────────

struct RawStdin
{
    file: std::fs::File,
    buf: [u8; 1],
}

impl RawStdin
{
    fn new() -> Self
    {
        use std::os::unix::io::FromRawFd;
        let file =
            unsafe { std::fs::File::from_raw_fd(std::io::stdin().as_raw_fd()) };
        Self { file, buf: [0u8; 1] }
    }
}

impl Drop for RawStdin
{
    fn drop(&mut self)
    {
        use std::os::unix::io::{FromRawFd, IntoRawFd};
        let old = std::mem::replace(&mut self.file, unsafe {
            std::fs::File::from_raw_fd(std::io::stdin().as_raw_fd())
        });
        std::mem::forget(old);
    }
}

impl Iterator for RawStdin
{
    type Item = u8;

    fn next(&mut self) -> Option<u8>
    {
        match self.file.read(&mut self.buf)
        {
            Ok(1) => Some(self.buf[0]),
            _ => None,
        }
    }
}

// ── stream assembly ───────────────────────────────────────────────────────────

fn build_stream(paths: &[String])
-> Result<Box<dyn Iterator<Item = u8>>, PqErr>
{
    let mut parts: Vec<Box<dyn Iterator<Item = u8>>> = Vec::new();
    for path in paths
    {
        let mut data = fs::read(path)?;
        if !data.ends_with(b"\n")
        {
            data.push(b'\n');
        }
        parts.push(Box::new(data.into_iter()));
    }
    parts.push(Box::new(RawStdin::new()));
    Ok(Box::new(parts.into_iter().flatten()))
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main()
{
    let paths: Vec<String> = env::args().skip(1).collect();

    let stream = match build_stream(&paths)
    {
        Ok(s) => s,
        Err(e) =>
        {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    // Utf8Buf accumulates the raw bytes as they're consumed.
    // Lex accumulates the Tok offsets as they're emitted.
    // tok_ty_val borrows utf8_buf.as_bytes() for lookback — always safe.
    let mut utf8_buf = Utf8Buf::new(stream);
    let mut lex = Lex::new();

    for (i, tok) in
        Tokenizer::new(Utf8Iter::new(&mut utf8_buf), &mut lex).enumerate()
    {
        let (ty, val) = tok_ty_val(utf8_buf.as_bytes(), tok);
        println!(
            "[{i:>4}]  byte={:<6}  ty={:<8}  {}",
            tok.offset(),
            ty.label(),
            val.display()
        );
    }
}
