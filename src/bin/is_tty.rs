use crossterm::{
    execute,
    terminal::{SetSize, size},
    tty::IsTty,
};
use std::io::{stdin, stdout};

pub fn main()
{
    println!("size: {:?}", size().unwrap());
    execute!(stdout(), SetSize(10, 10)).unwrap();
    println!("resized: {:?}", size().unwrap());

    if stdin().is_tty()
    {
        println!("Running normally");
    }
    else
    {
        println!("Running within pipe");
    }
}
