#[cfg(false)]
mod init
{
    use tokio::fs::File;
    use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    #[tokio::main(flavor = "multi_thread", worker_threads = 3)]
    async fn main() -> io::Result<()>
    {
        let (tx, mut rx) = mpsc::channel::<String>(10);

        // Task 1: read from stdin
        let tx1 = tx.clone();
        tokio::spawn(async move {
            let mut buf = String::new();
            let mut stdin = io::stdin();
            stdin.read_to_string(&mut buf).await.unwrap();
            tx1.send(buf).await.unwrap();
        });

        // Task 2: write to stdout
        let mut stdout = io::stdout();
        if let Some(msg) = rx.recv().await
        {
            stdout.write_all(msg.as_bytes()).await?;
        }

        // Task 3: write to file
        let mut file = File::create("out.txt").await?;
        if let Some(msg) = rx.recv().await
        {
            file.write_all(msg.as_bytes()).await?;
        }

        Ok(())
    }
}
fn main() {}
