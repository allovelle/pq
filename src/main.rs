use sqlx::Connection;
use sqlx::sqlite::SqliteConnection;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error>
{
    println!("Hello, world!");

    let conn = SqliteConnection::connect("sqlite::memory:").await?;

    Ok(())
}
