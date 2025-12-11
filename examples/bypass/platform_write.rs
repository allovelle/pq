// main.rs
//
// Cross-platform unbuffered stdout writer.
// Writes 2 bytes every 10ms directly to stdout.

use std::time::Duration;

#[cfg(unix)]
fn write_bytes(bytes: &[u8])
{
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::FromRawFd;

    // SAFETY: constructing File from raw fd 1 (stdout).
    let mut stdout =
        unsafe { File::from_raw_fd(std::io::stdout().as_raw_fd()) };
    let _ = stdout.write_all(bytes);
    // Prevent dropping which would close fd
    std::mem::forget(stdout);
}

#[cfg(windows)]
fn write_bytes(bytes: &[u8])
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

pub fn main_platform_write()
{
    loop
    {
        write_bytes(b"Hi");
        std::thread::sleep(Duration::from_millis(10));
    }
}
