use std::io;
use std::time::Duration;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> io::Result<()>
{
    let mut stdin = tokio::io::stdin();
    let mut buffer = [0u8; 1024];

    loop
    {
        match tokio::time::timeout(
            Duration::from_millis(500),
            stdin.read(&mut buffer),
        )
        .await
        {
            Ok(Ok(0)) =>
            {
                println!("EOF reached.");
                break;
            }
            Ok(Ok(n)) =>
            {
                let received_bytes = &buffer[.. n];
                match String::from_utf8(received_bytes.to_vec())
                {
                    Ok(s) =>
                    {
                        println!("Read ({} bytes): {}", n, s);
                    }
                    Err(e) =>
                    {
                        eprintln!("Error decoding UTF-8: {}", e);
                    }
                }
            }
            Ok(Err(e)) =>
            {
                eprintln!("IO error: {}", e);
                break;
            }
            Err(_) =>
            {
                println!(
                    "Read timed out after 500ms. No data or partial data read."
                );
            }
        }
    }

    Ok(())
}
