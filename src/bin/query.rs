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

fn parse_slice(input: &str, len: usize) -> Option<Vec<usize>>
{
    // Parse syntax like: start:stop:step
    let parts: Vec<&str> = input.split(':').collect();

    let (start, stop, step) = match parts.len()
    {
        1 =>
        {
            // only start
            let s = parts[0].parse().ok()?;
            (s, len, 1)
        }
        2 =>
        {
            // start:stop
            let s =
                if parts[0].is_empty() { 0 } else { parts[0].parse().ok()? };
            let e = if parts[1].is_empty()
            {
                len
            }
            else
            {
                parts[1].parse().ok()?
            };
            (s, e, 1)
        }
        3 =>
        {
            // start:stop:step
            let s =
                if parts[0].is_empty() { 0 } else { parts[0].parse().ok()? };
            let e = if parts[1].is_empty()
            {
                len
            }
            else
            {
                parts[1].parse().ok()?
            };
            let st =
                if parts[2].is_empty() { 1 } else { parts[2].parse().ok()? };
            (s, e, st)
        }
        _ => return None,
    };

    if step == 0
    {
        return None;
    }

    let mut out = Vec::new();
    let mut i = start;

    if step > 0
    {
        while i < stop && i < len
        {
            out.push(i);
            i += step;
        }
    }
    else
    {
        let mut i = start;
        while i > stop && i < len
        {
            out.push(i);
            i = (i as i64 + step as i64) as usize;
        }
    }

    Some(out)
}

fn main() -> Result<(), Box<dyn Error>>
{
    let mut stdout = stdout();

    // Sample string to slice
    let source = "Line 0\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9";
    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len();

    // Enable raw mode
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;
    execute!(stdout, cursor::Hide)?;

    let mut input = String::new();

    loop
    {
        // Clear + redraw
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Compute slice
        if let Some(indexes) = parse_slice(&input, total)
        {
            for i in indexes
            {
                if let Some(line) = lines.get(i)
                {
                    writeln!(stdout, "{line}")?;
                }
            }
        }
        else
        {
            writeln!(stdout, "(invalid slice)")?;
        }

        // Draw input at bottom
        let size = terminal::size()?;
        let last_line = size.1 - 1;

        execute!(stdout, cursor::MoveTo(0, last_line))?;
        write!(stdout, "slice> {input}")?;
        stdout.flush()?;

        // Handle events
        if event::poll(Duration::from_millis(100))?
        {
            match event::read()?
            {
                Event::Key(k) => match k.code
                {
                    KeyCode::Esc => break,
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace =>
                    {
                        input.pop();
                    }
                    KeyCode::Enter =>
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

    // Cleanup
    execute!(stdout, cursor::Show)?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
