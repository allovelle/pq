use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{self, ClearType},
};
use std::io::{self, Write, stdout};

fn main() -> io::Result<()>
{
    // Enter raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();

    let (cols, rows) = terminal::size()?; // terminal dimensions
    let mut buffer = String::new();

    loop
    {
        // Print terminal_height - 1 lines
        stdout.execute(crossterm::terminal::Clear(ClearType::All))?;
        for _ in 0 .. rows - 1
        {
            println!();
        }
        print!(">>> {}", buffer);
        stdout.flush()?;

        // Wait for key event
        if event::poll(std::time::Duration::from_millis(500))?
        {
            if let Event::Key(key_event) = event::read()?
            {
                match key_event.code
                {
                    KeyCode::Char(c) if c.is_ascii_digit() =>
                    {
                        // interpret digit as "length to print"
                        let len = c.to_digit(10).unwrap() as usize;
                        buffer = "Hello, world!".chars().take(len).collect();
                    }
                    KeyCode::Esc => break, // exit on ESC
                    _ =>
                    {}
                }
            }
            else
            {
                continue;
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}
