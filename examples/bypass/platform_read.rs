//! stdin_sync.rs
//!
//! Cross‑platform unbuffered stdin reader.
//!
//! ## Caveats
//! - On **pipes/sockets/redirected stdin**, bytes arrive immediately and `read()` returns as soon as data is available.
//! - On **interactive terminals**, the OS line discipline usually buffers input until Enter is pressed.
//!   - To get truly raw keystrokes, you must disable canonical mode (POSIX `termios`) or use Windows console APIs.
//! - This example uses raw file descriptors (Unix/macOS) or raw handles (Windows) to bypass Rust’s buffered `stdin`.

use std::io::Read;

#[cfg(unix)]
fn read_bytes(buf: &mut [u8]) -> usize
{
    use std::fs::File;
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let mut stdin = unsafe { File::from_raw_fd(std::io::stdin().as_raw_fd()) };
    let n = stdin.read(buf).unwrap_or(0);
    std::mem::forget(stdin);
    n
}

#[cfg(windows)]
fn read_bytes(buf: &mut [u8]) -> usize
{
    use std::fs::File;
    use std::os::windows::io::{AsRawHandle, FromRawHandle};

    let mut stdin =
        unsafe { File::from_raw_handle(std::io::stdin().as_raw_handle()) };
    let n = stdin.read(buf).unwrap_or(0);
    std::mem::forget(stdin);
    n
}

pub fn main_platform_read()
{
    let mut buf = [0u8; 2];
    loop
    {
        let n = read_bytes(&mut buf);
        if n > 0
        {
            println!("Read: {:?}", &buf[.. n]);
        }
    }
}
