// async_main.rs
//
// Cross-platform unbuffered stdout writer using Tokio.
// Writes 2 bytes every 10ms directly to stdout.

use tokio::time::{Duration, sleep};

#[cfg(unix)]
async fn write_bytes(bytes: &[u8])
{
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::FromRawFd;

    let mut stdout =
        unsafe { File::from_raw_fd(std::io::stdout().as_raw_fd()) };
    let _ = stdout.write_all(bytes);
    std::mem::forget(stdout);
}

#[cfg(windows)]
async fn write_bytes(bytes: &[u8])
{
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::io::FromRawHandle;

    let mut stdout =
        unsafe { File::from_raw_handle(std::io::stdout().as_raw_handle()) };
    let _ = stdout.write_all(bytes);
    std::mem::forget(stdout);
}

#[tokio::main]
pub async fn main_platform_write_async()
{
    loop
    {
        write_bytes(b"Hi").await;
        sleep(Duration::from_millis(10)).await;
    }
}
