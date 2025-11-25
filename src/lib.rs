use thiserror::Error;

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    Io(#[from] std::io::Error),
}

pub type PqResult<T> = Result<T, PqErr>;
