// // $ stdout | pq .
// // $ stdout | pq -q
// // $ stdout | pq --query
// // Is tty? Different than STDIN?

// use crossterm::style::{Color, Stylize};
// use serde_json::{Number, Value};
// use std::convert::From;

// fn main() -> Result<(), std::io::Error>
// {
//     let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

//     println!("{}", serde_json::to_string_pretty(&value)?);

//     Ok(())
// }
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor},
    terminal::{self, ClearType},
};
use std::io::{Write, stdout};

fn main() -> Result<(), std::io::Error>
{
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;

    // Example content: 50 lines
    let content: Vec<String> =
        (0 .. 50).map(|i| format!("Line {}", i)).collect();
    let mut offset = 0;

    loop
    {
        let (cols, rows) = terminal::size()?;
        let view_height = rows.saturating_sub(1); // leave last line for input bar

        // Clear screen
        execute!(stdout, terminal::Clear(ClearType::All))?;

        // Draw visible lines
        for (i, line) in
            content.iter().skip(offset).take(view_height as usize).enumerate()
        {
            execute!(stdout, cursor::MoveTo(0, i as u16))?;
            println!("{}", line);
        }

        // Draw bottom bar with background color
        execute!(
            stdout,
            cursor::MoveTo(0, rows - 1),
            SetBackgroundColor(Color::Blue),
            Print(">>> "),
            ResetColor
        )?;
        stdout.flush()?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            match key.code
            {
                KeyCode::Esc => break,
                KeyCode::Up =>
                {
                    offset = offset.saturating_sub(1);
                }
                KeyCode::Down =>
                {
                    if offset + 1 < content.len()
                    {
                        offset += 1;
                    }
                }
                KeyCode::Char(c) =>
                {
                    // Example: crude parser for "start-end-step"
                    // For demo, just quit on 'q'
                    if c == 'q'
                    {
                        break;
                    }
                }
                _ =>
                {}
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}
