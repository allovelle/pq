/// main.rs — pq entry point.
///
/// Pipeline:
///   files + STDIN  →  byte iterator  →  Utf8Iter  →  Tokenizer  →  print
mod err;
mod lex;
mod utf8;

use err::PqErr;
use lex::tokenize;
use std::{
    env, fs,
    io::{self, Read},
};
use utf8::Utf8Iter;

// ── byte source ───────────────────────────────────────────────────────────────
// Collects all input (files then stdin) into one Vec<u8> so we have a single
// owned byte sequence to drive the iterator pipeline from.

fn collect_input(paths: &[String]) -> Result<Vec<u8>, PqErr>
{
    let mut buf: Vec<u8> = Vec::new();

    for path in paths
    {
        let bytes = fs::read(path)?;
        buf.extend_from_slice(&bytes);
        // Ensure each file ends with a newline so JSON values don't blur together.
        if !bytes.ends_with(b"\n")
        {
            buf.push(b'\n');
        }
    }

    // Read STDIN — use raw (non-canonical) mode on Unix so we don't wait for
    // a full buffer or a newline before bytes arrive.
    let stdin_bytes = read_stdin()?;
    if !stdin_bytes.is_empty()
    {
        buf.extend_from_slice(&stdin_bytes);
        if !stdin_bytes.ends_with(b"\n")
        {
            buf.push(b'\n');
        }
    }

    Ok(buf)
}

// ── raw STDIN ─────────────────────────────────────────────────────────────────

#[cfg(unix)]
fn read_stdin() -> Result<Vec<u8>, PqErr>
{
    use libc::{ECHO, ICANON, STDIN_FILENO, TCSANOW, tcgetattr, tcsetattr};
    use std::mem;

    let mut buf = Vec::new();
    let mut old: libc::termios = unsafe { mem::zeroed() };
    let is_tty = unsafe { tcgetattr(STDIN_FILENO, &mut old) } == 0;

    if is_tty
    {
        let mut raw = old;
        raw.c_lflag &= !(ICANON | ECHO);
        raw.c_cc[libc::VMIN] = 0; // non-blocking read
        raw.c_cc[libc::VTIME] = 1; // 0.1 s timeout
        unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &raw) };
    }

    let mut tmp = [0u8; 4096];
    loop
    {
        match io::stdin().lock().read(&mut tmp)
        {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[.. n]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) =>
            {
                if is_tty
                {
                    unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &old) };
                }
                return Err(e.into());
            }
        }
    }

    if is_tty
    {
        unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &old) };
    }
    Ok(buf)
}

#[cfg(not(unix))]
fn read_stdin() -> Result<Vec<u8>, PqErr>
{
    let mut buf = Vec::new();
    io::stdin().lock().read_to_end(&mut buf)?;
    Ok(buf)
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main()
{
    let paths: Vec<String> = env::args().skip(1).collect();

    let bytes = match collect_input(&paths)
    {
        Ok(b) => b,
        Err(e) =>
        {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if bytes.is_empty()
    {
        eprintln!("pq: no input (pass filenames or pipe JSON on STDIN)");
        return;
    }

    // Pipeline:  Vec<u8>  →  Utf8Iter  →  Tokenizer  →  print
    let chars = Utf8Iter::new(bytes.iter().copied());
    let tokens = tokenize(chars);

    for (i, tok) in tokens.enumerate()
    {
        println!("[{i:>4}]  {}", tok.display());
    }
}
