use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{self, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::io::{self};

fn main() -> io::Result<()>
{
    let json = std::io::read_to_string(std::io::stdin())?;
    let mut stdout = std::io::stdout();

    stdout
        .execute(Clear(ClearType::All))?
        .execute(terminal::DisableLineWrap)?
        .queue(style::PrintStyledContent(json.magenta()))?
        .queue(terminal::EnableLineWrap)?;

    Ok(())
}
