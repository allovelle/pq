use std::str::Utf8Chunks;

use crossterm::style::Stylize;
use pq::diagnostics::UsageReport;
use pq::tok::{Act::*, CharMatch::*, State::*, state_transition_table};
use pq::{PqResult, brk_if};

// TODO: Rework tokenize() so that it works off of a stream of characters and
// TODO: lazily produces tokens to an output stream
// TODO: file stream |> tokenizer |> parser

use serde_json::Deserializer;
use tokio::io::AsyncWriteExt;
use tokio::io::{self, AsyncReadExt};
use tokio::sync::mpsc;

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> io::Result<()>
{
    let (tx_in, mut rx_in) = mpsc::channel::<String>(8);
    let (tx_out, mut rx_out) = mpsc::channel::<String>(8);

    let mut stdin = io::stdin();
    let mut buf = Vec::<u8>::with_capacity(1024);

    loop
    {
        let consumed = stdin.read_buf(&mut buf).await?;
        {
            brk_if!(consumed == 0);
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

        while let Some(Ok(line)) = std::io::stdin().lines().next()
        {
            println!("{line}");
        }
    }
    Ok(())
}

pub async fn tokenize_stream(stream: &mut impl Iterator<Item = char>)
{
    if let Some(ch) = stream.next()
    {}
}
