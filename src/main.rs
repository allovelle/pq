mod cli;

use std::fs::File;
use std::io;
use std::io::Read;
use std::io::prelude::*;

use sqlx::sqlite::SqliteConnection;
use sqlx::{Connection, Executor};

const SQL_INIT: &str = include_str!("init.sql");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error>
{
    let cli_args = cli::parse();

    let json = match cli_args.file
    {
        Some(path) => std::fs::read_to_string(path)?,
        None =>
        {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    println!("{json}");

    let mut conn = SqliteConnection::connect("sqlite::memory:").await?;
    conn.execute(SQL_INIT).await?;

    Ok(())
}
