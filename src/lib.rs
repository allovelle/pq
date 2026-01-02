pub mod cli;
pub mod diagnostics;
pub mod inc;
pub mod iter;
pub mod range;
pub mod tok;
pub mod tokv1;
pub mod txt;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    LexErr(#[from] crate::tokv1::LexErr),

    #[error(transparent)]
    ParseIntErr(#[from] std::num::ParseIntError),

    #[error(transparent)]
    ParseFloatErr(#[from] std::num::ParseFloatError),

    #[error(transparent)]
    CliErr(#[from] clap::Error),
}

pub type PqResult<T> = Result<T, PqErr>;

/// A macro for early returns based on a condition.
///
/// # Examples
///
/// ```rust
/// use pq::ret_if;
///
/// fn demo(x: i32) -> i32 {
///     ret_if!(x < 0, 0);      // return 0 if x is negative
///     ret_if!(x == 42, 99);   // return 99 if x is 42
///     x + 1
/// }
///
/// assert_eq!(demo(-5), 0);
/// assert_eq!(demo(42), 99);
/// assert_eq!(demo(7), 8);
/// ```
#[macro_export]
macro_rules! ret_if {
    ($cond:expr, $val:expr) => {
        if $cond
        {
            return $val;
        }
    };

    ($cond:expr) => {
        if $cond
        {
            return;
        }
    };
}

/// A macro for breaking loops early based on a condition.
#[macro_export]
macro_rules! brk_if {
    ($cond:expr, $val:expr) => {
        if $cond
        {
            break $val;
        }
    };

    ($cond:expr) => {
        if $cond
        {
            break;
        }
    };
}

/// A macro for continuing loops based on a condition.
#[macro_export]
macro_rules! continue_if {
    ($cond:expr) => {
        if $cond
        {
            continue;
        }
    };
}
