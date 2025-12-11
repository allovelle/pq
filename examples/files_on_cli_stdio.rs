#![allow(clippy::uninit_vec)]

// Cargo.toml dependencies (example):
// [dependencies]
// clap = { version = "4", features = ["derive"] }
// thiserror = "1.0"
// tokio = { version = "1", features = ["full"] }

use clap::Parser;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Minimal command-line:
/// provide file paths. The special name "-" means stdin.
/// The program always includes stdin as an extra stream (so N filenames -> N+1 streams).
#[derive(Parser, Debug)]
struct Args
{
    /// Files to read. Use '-' for stdin. stdin will always be appended as an extra stream,
    /// so it may appear twice if you pass '-' explicitly.
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Index of stream to treat as "direct non-buffered" (0-based, counting files first, then stdin appended).
    /// Defaults to last stream (the appended stdin).
    #[arg(long, value_name = "IDX")]
    direct: Option<usize>,

    /// Chunk size for reads (bytes). Keep modest (e.g., 8192).
    #[arg(long, default_value_t = 8192)]
    chunk_size: usize,
}

#[derive(Error, Debug)]
enum AppError
{
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("utf8 error while decoding: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("invalid direct index {0}")]
    InvalidDirectIndex(usize),
}

/// A small callback trait so processing code can be separated. In this example
/// we simply print chars, but replace `process_char` with whatever you need.
#[allow(unused_variables)]
async fn process_char(c: char)
{
    // Example sink: minimal operation on each char
    // Replace this with your actual per-char processing logic.
    print!("{}", c);
}

/// An async function that reads from a non-buffered file handle (no BufReader)
/// directly into a stack buffer and processes bytes incrementally (converting to chars,
/// handling tail).
async fn process_nonbuffered<R>(
    mut rdr: R,
    chunk_size: usize,
) -> Result<(), AppError>
where
    R: AsyncReadExt + Unpin,
{
    // We'll still use the same consumer logic as buffered mode; for simplicity we
    // call the sync consumer function directly on each chunk.
    let mut buf = vec![0u8; chunk_size];

    // tail stores partial bytes of a truncated UTF-8 code point (length <= 4).
    let mut tail = [0u8; 4];
    let mut tail_len = 0usize;

    loop
    {
        let n = rdr.read(&mut buf).await?;
        if n == 0
        {
            // EOF: if there is an incomplete tail, we can decide how to handle it; here we ignore or error.
            if tail_len != 0
            {
                eprintln!(
                    "\nwarning: dangling {} bytes of partial utf-8 at EOF; ignored",
                    tail_len
                );
            }
            break;
        }

        let chunk = &buf[.. n];
        consume_chunk(chunk, &mut tail, &mut tail_len).await?;
    }

    Ok(())
}

/// Buffered path: producer reads from BufReader into heap-allocated Vec<u8> chunks
/// and sends them to a consumer over an mpsc channel (pipelined).
async fn process_buffered<R>(rdr: R, chunk_size: usize) -> Result<(), AppError>
where
    R: AsyncReadExt + Unpin + Send + 'static,
{
    // small bounded channel to allow pipelining without unbounded allocations
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(4);

    // producer
    let mut producer = tokio::spawn(async move {
        let mut reader = BufReader::new(rdr);
        let mut local_buf = vec![0u8; chunk_size];

        loop
        {
            match reader.read(&mut local_buf).await
            {
                Ok(0) =>
                {
                    // EOF
                    break;
                }
                Ok(n) =>
                {
                    // allocate a Vec<u8> sized exactly n (small heap allocation)
                    // This is the single allocation per chunk; chosen to minimize copying.

                    let mut owned = Vec::with_capacity(n);
                    unsafe {
                        // safety: we set_len immediately and copy bytes
                        owned.set_len(n);
                    }
                    owned.copy_from_slice(&local_buf[.. n]);

                    if tx.send(owned).await.is_err()
                    {
                        // consumer dropped; stop
                        break;
                    }
                }
                Err(e) =>
                {
                    eprintln!("producer read error: {}", e);
                    break;
                }
            }
        }
        // drop tx automatically
    });

    // consumer: receives Vec<u8> and consumes them into chars with tail handling
    // tail and tail_len persist across chunks
    let mut tail = [0u8; 4];
    let mut tail_len = 0usize;

    while let Some(chunk) = rx.recv().await
    {
        consume_chunk(&chunk, &mut tail, &mut tail_len).await?;
    }

    // wait for producer to finish cleanly
    let _ = producer.await;

    // If tail left at EOF, warn
    if tail_len != 0
    {
        eprintln!(
            "\nwarning: dangling {} bytes of partial utf-8 at EOF; ignored",
            tail_len
        );
    }

    Ok(())
}

/// Core chunk consumer: decode bytes -> chars, handle tail of fragmented utf-8.
/// Uses at most a tiny stack temp buffer when needing to combine tail + start of chunk.
async fn consume_chunk(
    chunk: &[u8],
    tail: &mut [u8; 4],
    tail_len: &mut usize,
) -> Result<(), AppError>
{
    // If there's a tail carried from previous chunk, we must try to complete it.
    let mut offset = 0usize;
    if *tail_len > 0
    {
        // Need up to (4 - tail_len) bytes from chunk to attempt to complete the codepoint.
        let need = 4usize.saturating_sub(*tail_len);
        let take = std::cmp::min(need, chunk.len());

        // Create a small temporary buffer that contains tail + the taken bytes
        let mut small = [0u8; 8]; // tail_len <=4, take <=4, so fits
        small[.. *tail_len].copy_from_slice(&tail[.. *tail_len]);
        small[*tail_len .. *tail_len + take].copy_from_slice(&chunk[.. take]);

        // Try to decode as UTF-8. If valid, we can pull the first char and continue with rest of chunk.
        match std::str::from_utf8(&small[.. *tail_len + take])
        {
            Ok(s) =>
            {
                // s may contain multiple chars (unlikely), but we need to take the first char's byte length.
                if let Some(first_char) = s.chars().next()
                {
                    let first_len = first_char.len_utf8();
                    // How many bytes of chunk did we consume to complete that char?
                    let consumed_from_chunk =
                        first_len.saturating_sub(*tail_len);
                    offset += consumed_from_chunk;
                    *tail_len = 0; // we've completed the tail
                    process_char(first_char).await;
                }
                else
                {
                    // This is valid utf8 but empty? shouldn't happen
                }
            }
            Err(e) =>
            {
                // Not complete yet. If chunk didn't provide enough bytes to complete the codepoint, stash them.
                // If chunk provided bytes but still invalid, it might be malformed; handle conservatively.
                let valid_up_to = e.valid_up_to();
                if valid_up_to == 0
                {
                    // no progress; if chunk length < needed, stash chunk bytes into tail
                    if take < need
                    {
                        // append taken bytes to tail
                        tail[*tail_len .. *tail_len + take]
                            .copy_from_slice(&chunk[.. take]);
                        *tail_len += take;
                        return Ok(());
                    }
                    else
                    {
                        // we had enough bytes but still invalid -> malformed sequence
                        // best-effort: try to skip one byte and continue
                        eprintln!(
                            "warning: malformed utf-8 in tail combine; skipping a byte"
                        );
                        // skip first byte of chunk
                        offset += 1;
                        *tail_len = 0;
                    }
                }
                else
                {
                    // Some bytes were valid (unlikely for the small combine), consume them accordingly
                    // Convert valid prefix to chars and emit
                    let valid = &small[.. valid_up_to];
                    let s = std::str::from_utf8(valid)?;
                    for ch in s.chars()
                    {
                        process_char(ch).await;
                    }
                    // Remaining bytes in small are invalid; attempt to stash remainder (rare)
                    let rem = (*tail_len + take).saturating_sub(valid_up_to);
                    if rem > 0
                    {
                        // copy remaining bytes from small into tail
                        let start = valid_up_to;
                        let rem_slice = &small[start .. start + rem];
                        // ensure rem <= 4
                        let copy_len = std::cmp::min(4, rem);
                        tail[.. copy_len]
                            .copy_from_slice(&rem_slice[.. copy_len]);
                        *tail_len = copy_len;
                        return Ok(());
                    }
                }
            }
        }
    }

    // Now process the rest of chunk starting at offset
    if offset < chunk.len()
    {
        let rest = &chunk[offset ..];

        match std::str::from_utf8(rest)
        {
            Ok(s) =>
            {
                // whole rest is valid UTF-8
                for ch in s.chars()
                {
                    process_char(ch).await;
                }
                // tail unchanged
                return Ok(());
            }
            Err(e) =>
            {
                let valid = e.valid_up_to();
                if valid > 0
                {
                    // emit valid prefix
                    let s = std::str::from_utf8(&rest[.. valid])?;
                    for ch in s.chars()
                    {
                        process_char(ch).await;
                    }
                }
                // stash remaining bytes into tail (they are partial or malformed)
                let rem = &rest[valid ..];
                if rem.len() > 4
                {
                    // too many invalid bytes: probably a malformed UTF-8 sequence. We'll keep last 4 bytes and warn.
                    let start = rem.len() - 4;
                    tail[.. 4].copy_from_slice(&rem[start ..]);
                    *tail_len = 4;
                    eprintln!(
                        "warning: large invalid region encountered; keeping last 4 bytes in tail"
                    );
                }
                else
                {
                    tail[.. rem.len()].copy_from_slice(rem);
                    *tail_len = rem.len();
                }

                return Ok(());
            }
        }
    }

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AppError>
{
    let args = Args::parse();

    // Build the stream list: all provided files first, then append stdin
    let mut sources: Vec<StreamSpec> = Vec::new();
    for p in &args.files
    {
        sources.push(StreamSpec::Path(p.clone()));
    }
    // Append stdin as the final stream
    sources.push(StreamSpec::Stdin);

    let total = sources.len();
    let direct_idx = args.direct.unwrap_or(total.saturating_sub(1));
    if direct_idx >= total
    {
        return Err(AppError::InvalidDirectIndex(direct_idx));
    }

    // Process streams sequentially (one at a time). For each stream, if it's the `direct_idx`,
    // run the non-buffered async fn; otherwise, run the buffered pipelined version.
    for (idx, spec) in sources.into_iter().enumerate()
    {
        eprintln!(
            "\n--- processing stream {}/{} ({}) ---",
            idx + 1,
            total,
            spec.describe()
        );
        match spec
        {
            StreamSpec::Path(path) =>
            {
                // open file
                let file = File::open(&path).await?;
                if idx == direct_idx
                {
                    // non-buffered: operate directly on File (no BufReader)
                    process_nonbuffered(file, args.chunk_size).await?;
                }
                else
                {
                    // buffered pipelined path
                    process_buffered(file, args.chunk_size).await?;
                }
            }
            StreamSpec::Stdin =>
            {
                // get tokio stdin handle
                let stdin = io::stdin();
                if idx == direct_idx
                {
                    // non-buffered: read directly from stdin (no BufReader)
                    process_nonbuffered(stdin, args.chunk_size).await?;
                }
                else
                {
                    process_buffered(stdin, args.chunk_size).await?;
                }
            }
        }
    }

    // flush final output if needed
    // (we printed chars to stdout directly; flush to ensure it's displayed)
    tokio::io::stdout().flush().await?;

    Ok(())
}

/// A simple descriptor for streams: either a file path or stdin.
enum StreamSpec
{
    Path(PathBuf),
    Stdin,
}
impl StreamSpec
{
    fn describe(&self) -> String
    {
        match self
        {
            StreamSpec::Path(p) => format!("{}", p.display()),
            StreamSpec::Stdin => String::from("stdin"),
        }
    }
}
