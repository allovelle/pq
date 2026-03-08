mod err;
mod lex;
mod utf8;

use error::AppError;
use lex::{tok_ty_val, tokenize};
use std::{cell::RefCell, env, fs, io::Read, rc::Rc};
use utf8::Utf8Iter;

// ── raw stdin byte iterator ───────────────────────────────────────────────────
// Wrap stdin's raw fd in a File without any buffering or termios wrangling.
// On a pipe the OS delivers bytes as they arrive; read() returns as soon as
// data is available.  mem::forget prevents double-close of fd 0.

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
    // Don't close fd 0 when we drop.
    fn drop(&mut self)
    {
        use std::os::unix::io::IntoRawFd;
        let file = std::mem::replace(&mut self.file, unsafe {
            std::fs::File::from_raw_fd(std::io::stdin().as_raw_fd())
        });
        std::mem::forget(file);
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
            _ => None, // EOF or error
        }
    }
}

// ── Tee ───────────────────────────────────────────────────────────────────────
// Forwards each byte to the downstream tokenizer pipeline while accumulating
// into a shared Vec so tok_ty_val() has a &[u8] slice to re-scan.

// ! In order to remove the Tee, each subsystem needs it's equivalent of IO:
// ! Retain: accumulate incremental input into buffer.
// ! Pass Along: temporarily store each input and delete after passing to output
// ! These must happen for BOTH input and output to provide usage ergonomics
// ! such as Lex Pass Along, but Parser Retain, Parser Pass Along its outputs.
// ! Both inputs and outputs of a system can be kept or passed along.
struct Tee
{
    inner: Box<dyn Iterator<Item = u8>>,
    buf: Rc<RefCell<Vec<u8>>>,
}

impl Tee
{
    fn new(inner: Box<dyn Iterator<Item = u8>>) -> Self
    {
        Self { inner, buf: Rc::new(RefCell::new(Vec::new())) }
    }

    fn buf(&self) -> Rc<RefCell<Vec<u8>>>
    {
        Rc::clone(&self.buf)
    }
}

impl Iterator for Tee
{
    type Item = u8;

    fn next(&mut self) -> Option<u8>
    {
        let b = self.inner.next()?;
        self.buf.borrow_mut().push(b);
        Some(b)
    }
}

// ── stream assembly ───────────────────────────────────────────────────────────

fn build_stream(
    paths: &[String],
) -> Result<Box<dyn Iterator<Item = u8>>, AppError>
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

    let tee = Tee::new(stream);
    let buf = tee.buf();
    let chars = Utf8Iter::new(tee);
    let tokens = tokenize(chars);

    for (i, tok) in tokens.enumerate()
    {
        let src = buf.borrow();
        let (ty, val) = tok_ty_val(&src, tok);
        println!(
            "[{i:>4}]  byte={:<6}  ty={:<8}  {}",
            tok.offset(),
            ty.label(),
            val.display()
        );
    }
}
