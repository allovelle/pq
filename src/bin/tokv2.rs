use pq::tokv1::{Act::*, CharMatch::*, State::*};
use std::io::Read;
use thiserror::Error;

// TODO: Rework tokenize() so that it works off of a stream of characters and
// TODO: lazily produces tokens to an output stream
// TODO: file stream |> tokenizer |> parser

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

fn main() -> Result<(), PqErr>
{
    let mut stdin = std::io::stdin();
    let mut buffer = Vec::with_capacity(4096);
    let mut read_upto = 0;
    match stdin.read(&mut buffer)
    {
        Ok(read) => read_upto = read,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
        Err(e) => eprintln!("Error reading stdin: {}", e),
    }

    {
        let borrowed = std::str::from_utf8(&buffer[.. read_upto])?;
        println!("Read {} bytes", borrowed);
    }
    buffer.clear();

    Ok(())
}

fn process_chunk() {}

pub async fn tokenize_stream(stream: &mut impl Iterator<Item = char>)
{
    if let Some(ch) = stream.next()
    {}
}
