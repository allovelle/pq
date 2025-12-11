mod platform_read;
mod platform_read_async;
mod platform_write;
mod platform_write_async;

use platform_read::*;
use platform_read_async::*;
use platform_write::*;
use platform_write_async::*;

pub use clap::Parser;
use std::fs::File;
use std::io::{self, ErrorKind, Read};
use std::os::unix::io::{AsRawFd, FromRawFd};

/// Queries JSON from the CLI
#[derive(Parser, Debug)]
#[command()]
pub struct Cli
{
    // /// The JSON file path to ingest & query. Defaults to STDIN.
    // #[arg(short, long)]
    // pub file: Option<String>,

    // /// The query to run on the provided JSON
    // pub query: Option<String>,
    command: String,
}

pub fn parse() -> Cli
{
    Cli::parse()
}

fn main()
{
    let args = Cli::parse();
    match args.command.as_str()
    {
        "read" => main_platform_read(),
        "read-async" => main_platform_read_async(),
        "write" => main_platform_write(),
        "write-async" => main_platform_write_async(),
        _ => (),
    }
}

fn main_handle_would_block()
{
    let mut stdin = unsafe { File::from_raw_fd(std::io::stdin().as_raw_fd()) };
    let mut buf = [0u8; 2];

    loop
    {
        match stdin.read(&mut buf)
        {
            Ok(n) if n > 0 =>
            {
                println!("Read: {:?}", &buf[.. n]);
            }
            Ok(_) =>
            {
                // EOF reached
                break;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock =>
            {
                // Non-blocking mode: either sleep/retry, or block if allowed
                // Here we just block by retrying immediately
                // (replace with sleep or poll if you want async behavior)
                continue;
            }
            Err(e) =>
            {
                eprintln!("Unexpected error: {}", e);
                break;
            }
        }
    }

    std::mem::forget(stdin);
}

/*
Great question — you’re right that when you bypass Rust’s buffered I/O and go straight to raw file descriptors or handles, the **errors you see can carry different meanings depending on which descriptor you’re touching**. Let’s break it down:

---

## 🦀 Error Types You’ll Encounter

Rust’s `std::io::Read` and `Write` traits return `std::io::Result<T>`, which wraps `std::io::Error`. That error has:

- **Kind (`ErrorKind`)** — a coarse category like `BrokenPipe`, `WouldBlock`, `UnexpectedEof`.
- **OS error code (`raw_os_error()`)** — the actual errno (Unix) or Win32 error code.

---

## 🐧 Unix / macOS (POSIX)

When you call `read(0, …)` or `write(1, …)`:

- **`EBADF` (Bad file descriptor)**
  - If stdin/stdout/stderr has been closed or redirected incorrectly.
  - Meaning: “this fd doesn’t exist.”
  - Rare unless you explicitly close them.

- **`EPIPE` (Broken pipe)**
  - Writing to stdout when it’s redirected to a pipe whose reader has closed.
  - Rust surfaces this as `ErrorKind::BrokenPipe`.
  - Example: your program writes to stdout, but the consumer (`head -n1`) exits early.

- **`EIO` (I/O error)**
  - Hardware or driver-level failure.
  - Could mean “terminal is borked” if the tty driver fails.

- **`EAGAIN` / `EWOULDBLOCK`**
  - If stdin/stdout is set non-blocking and no data is available.
  - Rust surfaces this as `ErrorKind::WouldBlock`.

- **`EOF` (End of file)**
  - `read(0, …)` returns 0 bytes when stdin is closed.
  - Rust surfaces this as `Ok(0)` rather than an error.

---

## 🪟 Windows (Win32)

When you call `ReadFile` or `WriteFile` on `STD_INPUT_HANDLE` / `STD_OUTPUT_HANDLE`:

- **`ERROR_INVALID_HANDLE` (6)**
  - If the handle is invalid (stdout closed or not inherited).
  - Equivalent to Unix `EBADF`.

- **`ERROR_BROKEN_PIPE` (109)**
  - Writing to stdout when the pipe is closed.
  - Same semantics as Unix `EPIPE`.

- **`ERROR_OPERATION_ABORTED` (995)**
  - If an async read/write was canceled.
  - More common in overlapped I/O scenarios.

- **`ERROR_NO_DATA` (232)**
  - Reading from a pipe with no data and the other end closed.
  - Similar to EOF.

---

## ⚙️ Why These Errors Feel Different

- **Normal file I/O:**
  - Errors usually mean “file not found,” “permission denied,” or “disk full.”
  - They’re about filesystem semantics.

- **Stdout/stderr/stderr fds (0,1,2):**
  - Errors usually mean “the communication channel is broken.”
  - Example: `BrokenPipe` is almost a signal that your *consumer* disappeared.
  - Example: `Bad file descriptor` means your process environment is misconfigured.

So yes — the **same `ErrorKind` carries different weight depending on the fd**. Seeing `BrokenPipe` on stdout is a very specific “downstream closed” signal, whereas on a file it would be unusual.

---

## 🔑 Takeaway

- **Yes, specific errors do carry extra meaning when you’re working with stdin/stdout/stderr.**
- On stdout: `BrokenPipe` = consumer gone.
- On stdin: `UnexpectedEof` = producer closed.
- On both: `Bad file descriptor` = environment misconfigured.
- On terminals: you may also see `WouldBlock` if you set non-blocking mode.

---

Would you like me to put together a **Rust cheat sheet mapping `ErrorKind` → typical meaning for stdin/stdout/stderr vs. normal files**? That way you’d have a quick reference when debugging these custom I/O setups.
*/
