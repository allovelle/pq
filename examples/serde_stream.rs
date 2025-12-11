use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> io::Result<()>
{
    use serde_json::Deserializer;
    use tokio::io::{self, AsyncReadExt};

    let mut stdin = io::stdin();
    let mut buf = Vec::new();

    loop
    {
        let n = stdin.read_buf(&mut buf).await?;
        if n == 0
        {
            break;
        }

        let stream =
            Deserializer::from_slice(&buf).into_iter::<serde_json::Value>();

        for value in stream
        {
            match value
            {
                Ok(v) =>
                {
                    println!("{}", v);
                    io::stdout().flush().await?;
                }
                Err(e) if e.is_eof() => break, // need more data
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}
