use crossterm::style::{Attribute, Color, Stylize};
use crossterm::{ExecutableCommand, QueueableCommand, cursor, style, terminal};
use std::io::{self, Write, stdout};

fn main() -> Result<(), io::Error>
{
    let mut stdout = stdout();

    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.queue(cursor::MoveTo(5, 5))?;

    let styled =
        "Hello there".with(Color::Yellow).attribute(Attribute::Bold).italic();

    stdout.queue(style::PrintStyledContent(styled))?;
    stdout.queue(cursor::MoveTo(5, 5))?;
    stdout.queue(cursor::MoveDown(1))?;

    let styled = "Hello there".with(Color::Yellow).attribute(Attribute::Italic);
    stdout.queue(style::PrintStyledContent(styled))?;

    // For the query line, each type:
    // stdout.execute(terminal::ClearType::FromCursorUp);
    // stdout.execute(terminal::ClearType::CurrentLine);

    stdout.flush()?;

    Ok(())
}
