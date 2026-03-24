// 👍🏻👍🏻👍🏻👍🏻👍🏻👍🏻👍🏻👍🏻
// //! # jq-starlark
// //!
// //! Entry point and orchestration.
// //!
// //! ## Pipeline
// //!
// //! ```text
// //! [files + STDIN]
// //!       │  raw-mode unbuffered reads
// //!       ▼
// //!   Utf8Buf  (append-only, partial-codepoint aware)
// //!       │
// //!       ▼
// //!   Lex::iter  (streaming structural indexing, no token storage)
// //!       │
// //!       ▼
// //!   Parser::next  (flat row stream, id/par/key/val/ty)
// //!       │
// //!       ▼
// //!   [formatter / query engine — future]
// //! ```
// //!
// //! ## Raw-mode STDIN
// //!
// //! `crossterm` is used to put the terminal into raw mode so that we read
// //! bytes as they arrive rather than waiting for a newline buffer to fill.
// //! This lets the pipeline start processing before the entire input is present.

// mod err;
// mod lex;
// mod parse;
// mod utf8;

// use std::fs;
// use std::io::{self, Read, Write};

// use crossterm::terminal;

// use err::AppError;
// use parse::{Parser, Row};
// use utf8::Utf8Buf;

// // ---------------------------------------------------------------------------

// fn main()
// {
//     if let Err(e) = run()
//     {
//         eprintln!("error: {e}");
//         std::process::exit(1);
//     }
// }

// fn run() -> Result<(), AppError>
// {
//     let args: Vec<String> = std::env::args().skip(1).collect();

//     let mut buf = Utf8Buf::with_capacity(64 * 1024);

//     // -----------------------------------------------------------------------
//     // 1. Read all named files first (they are already fully available).
//     // -----------------------------------------------------------------------
//     for path in &args
//     {
//         let bytes = fs::read(path).map_err(|e| {
//             AppError::Io(io::Error::new(e.kind(), format!("{path}: {e}")))
//         })?;
//         buf.push_bytes(&bytes);
//         // Separate JSONL documents from different files with a newline so
//         // that root detection stays clean.
//         buf.push_byte(b'\n');
//     }

//     // -----------------------------------------------------------------------
//     // 2. Read STDIN in raw mode to avoid waiting for full buffer / EOF.
//     // -----------------------------------------------------------------------
//     let stdin_bytes = read_stdin_raw()?;
//     buf.push_bytes(&stdin_bytes);

//     // -----------------------------------------------------------------------
//     // 3. Incremental lex + parse pipeline.
//     // -----------------------------------------------------------------------
//     let stdout = io::stdout();
//     let mut out = io::BufWriter::new(stdout.lock());

//     // We use the *streaming* parser (not the store) — it drives Lex::iter
//     // internally and never buffers the whole document.
//     let mut parser = Parser::new(buf.as_bytes());

//     while let Some(row) = parser.next()?
//     {
//         emit_row(&row, &buf, &mut out)?;
//     }

//     out.flush().map_err(AppError::Io)?;
//     Ok(())
// }

// // ---------------------------------------------------------------------------
// // Raw-mode STDIN reader
// // ---------------------------------------------------------------------------

// /// Read all of STDIN in **terminal raw mode** so we do not wait for the OS
// /// line-buffer to fill before bytes become available.
// ///
// /// Falls back to ordinary blocking read if STDIN is not a tty (e.g. piped).
// fn read_stdin_raw() -> Result<Vec<u8>, AppError>
// {
//     // If STDIN is not a tty there is no buffer to fight; read normally.
//     if !terminal::is_raw_mode_enabled().unwrap_or(false)
//         && !crossterm::tty::IsTty::is_tty(&io::stdin())
//     {
//         let mut bytes = Vec::new();
//         io::stdin().read_to_end(&mut bytes).map_err(AppError::Io)?;
//         return Ok(bytes);
//     }

//     // Enable raw mode: keypresses are available immediately.
//     terminal::enable_raw_mode()
//         .map_err(|e| AppError::Terminal(e.to_string()))?;

//     let mut bytes = Vec::new();
//     let mut tmp = [0u8; 256];

//     let result = (|| -> Result<(), AppError> {
//         loop
//         {
//             let n = io::stdin().read(&mut tmp).map_err(AppError::Io)?;
//             if n == 0
//             {
//                 break; // EOF
//             }
//             bytes.extend_from_slice(&tmp[.. n]);

//             // Ctrl-D (0x04) signals manual EOF in raw mode.
//             if tmp[.. n].contains(&0x04)
//             {
//                 bytes.retain(|&b| b != 0x04);
//                 break;
//             }
//         }
//         Ok(())
//     })();

//     // Always restore terminal state, even on error.
//     let _ = terminal::disable_raw_mode();
//     result?;

//     Ok(bytes)
// }

// // ---------------------------------------------------------------------------
// // Row emitter (placeholder formatter)
// // ---------------------------------------------------------------------------

// /// Emit one [`Row`] to `out` in a simple human-readable debug format.
// ///
// /// This will be replaced by the real formatter/query engine in a later phase.
// fn emit_row(
//     row: &Row,
//     buf: &Utf8Buf,
//     out: &mut impl Write,
// ) -> Result<(), AppError>
// {
//     let key_str = if row.key.byte_len() > 0 { buf.slice(row.key) } else { "" };
//     let val_str = if row.val.byte_len() > 0 { buf.slice(row.val) } else { "" };

//     writeln!(
//         out,
//         "id={:<4} par={:<4} root={} ty={:<10} key={:<20} val={}",
//         row.id,
//         row.par,
//         row.is_root() as u8,
//         format!("{:?}", row.ty),
//         key_str,
//         val_str,
//     )
//     .map_err(AppError::Io)
// }

// mod lex;
mod init;
mod utf8;

/* fn main2()
{
    use lex::*;

    let buf = br#"  { "key": -1.5e2, "ok": true, "arr": [null, false] }  "#;

    // iterate — collect all token indices
    let mut tokens: Vec<TokIdx> = vec![];
    let mut idx = 0;
    while let Ok((tok, n)) = tok_from(buf, idx)
    {
        tokens.push(tok);
        idx += n;
    }

    // print all
    for tok in &tokens
    {
        let t = tok_at(buf, *tok);
        println!("{:?}  {:?}", t.kind, str::from_utf8(t.val).unwrap());
    }

    // random access — scrape token 2 by stored index
    let t = tok_at(buf, tokens[2]);
    println!(
        "\ntoken[2] => {:?}  {:?}",
        t.kind,
        str::from_utf8(t.val).unwrap()
    );
}
 */

fn main()
{
    use utf8::*;

    let json = br#"  { "key": -1.5e2, "ok": true, "arr": [null, false] }  "#;

    let mut buffer: Utf8Buf = Utf8Buf::new();
    assert_eq!(buffer.raw_len(), 0, "buf should be empty");
    buffer.push_bytes(json);
    assert_eq!(buffer.raw_len(), json.len(), "buf should contain the json");

    let yu: [u8; 4] = [0xE8, 0xAA, 0x9E, 0x00]; // 語
    buffer.push_byte(yu[0]);
    buffer.push_byte(yu[1]);
    buffer.push_byte(yu[2]);

    // ? What is the expected use case of Utf8Buf? It's not for tokens, its for
    // ? piecemeal iteration through the buffer.
    // The use case of Utf8Buf is piecemeal indexing of the buffer while
    // allowing synchronous appends consisting of partial or complete UTF-8
    // codepoints.

    // TODO: Iterator returns None when partial codepoint, yet the caller has no
    // TODO: choice to continue until there are more bytes in the buffer

    let mut iter = buffer.iter_from(0);
    let mut offset = 0;
    #[allow(clippy::while_let_on_iterator)]
    while let Some((at, ch)) = iter.next()
    {
        // ? This is doing nothing:
        // More bytes to read, yields the same char as before
        /* if iter.partial() && at == iter.byte_pos()
        {
            println!("cancel");
            continue;
        } */

        let codepoint = buffer.text(offset as u32, at as u32);
        println!("{ch:?} <-> {codepoint:?}");

        offset = at
    }
}
