//! stdin_async.rs
//!
//! Async (Tokio) unbuffered stdin reader.
//!
//! ## Caveats
//! - Same caveats as synchronous version:
//!   - Pipes/sockets: immediate delivery.
//!   - Terminals: line buffering unless raw mode is enabled.
//! - Tokio’s async I/O works fine with pipes and redirected stdin.
//! - For interactive terminals, you’ll still need raw mode for per‑keystroke reads.

use tokio::io::AsyncReadExt;
use tokio::time::{Duration, sleep};

#[cfg(unix)]
async fn read_bytes(buf: &mut [u8]) -> usize
{
    use std::fs::File;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use tokio::fs::File as TokioFile;

    let stdin = unsafe { File::from_raw_fd(std::io::stdin().as_raw_fd()) };
    let mut async_stdin = TokioFile::from_std(stdin);
    let n = async_stdin.read(buf).await.unwrap_or(0);
    std::mem::forget(async_stdin);
    n
}

#[cfg(windows)]
async fn read_bytes(buf: &mut [u8]) -> usize
{
    use std::fs::File;
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use tokio::fs::File as TokioFile;

    let stdin =
        unsafe { File::from_raw_handle(std::io::stdin().as_raw_handle()) };
    let mut async_stdin = TokioFile::from_std(stdin);
    let n = async_stdin.read(buf).await.unwrap_or(0);
    std::mem::forget(async_stdin);
    n
}

#[tokio::main]
async fn main()
{
    let mut buf = [0u8; 2];
    loop
    {
        let n = read_bytes(&mut buf).await;
        if n > 0
        {
            println!("Read: {:?}", &buf[.. n]);
        }
        // Sleep just to avoid busy loop; remove if you want immediate reads.
        sleep(Duration::from_millis(10)).await;
    }
}
