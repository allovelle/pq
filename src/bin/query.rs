use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use crossterm::style::{Attribute, Color, Stylize};
use crossterm::{ExecutableCommand, QueueableCommand, cursor, style, terminal};
use std::io::{self, Write, stdout};

// fn main() -> Result<(), io::Error>
// {
//     let mut stdout = stdout();

//     stdout.execute(terminal::Clear(terminal::ClearType::All))?;
//     stdout.queue(cursor::MoveTo(5, 5))?;

//     let styled =
//         "Hello there".with(Color::Yellow).attribute(Attribute::Bold).italic();

//     stdout.queue(style::PrintStyledContent(styled))?;
//     stdout.queue(cursor::MoveTo(5, 5))?;
//     stdout.queue(cursor::MoveDown(1))?;

//     let styled = "Hello there".with(Color::Yellow).attribute(Attribute::Italic);
//     stdout.queue(style::PrintStyledContent(styled))?;

//     // For the query line, each type:
//     // stdout.execute(terminal::ClearType::FromCursorUp);
//     // stdout.execute(terminal::ClearType::CurrentLine);

//     stdout.flush()?;

//     Ok(())
// }
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::ClearType,
};
use std::{error::Error, time::Duration};

fn main() -> Result<(), Box<dyn Error>>
{
    // Get the json
    let text = std::fs::read_to_string("json3.json")?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let mut stdout = stdout();

    // Sample string to slice
    let source = "Line 0\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9";
    let lines: Vec<&str> = source.lines().collect();

    // Enable raw mode
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;
    execute!(stdout, cursor::Hide)?;

    let mut input = String::from("");

    loop
    {
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        stdout.execute(cursor::MoveToColumn(0))?;
        let formatted = serde_json::to_string_pretty(&json)?;
        for line in formatted.lines()
        {
            stdout.execute(cursor::MoveToColumn(0))?;
            stdout.execute(cursor::MoveDown(1))?;
            write!(stdout, "{line}")?;
        }

        // Draw input at bottom
        let size = terminal::size()?;
        let last_line = size.1 - 1;
        execute!(stdout, cursor::MoveTo(0, last_line))?;
        write!(stdout, "> {input}")?;
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))?
        {
            use KeyCode::*;
            match event::read()?
            {
                Event::Key(KeyEvent {
                    code: Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => break,
                Event::Key(k) => match k.code
                {
                    Esc => break,
                    Char(c) => input.push(c),
                    Backspace =>
                    {
                        input.pop();
                    }
                    Enter =>
                    {
                        input.clear();
                    }
                    _ =>
                    {}
                },
                _ =>
                {}
            }
        }
    }

    execute!(stdout, cursor::Show)?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
