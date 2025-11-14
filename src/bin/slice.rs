use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{self, ClearType},
};
use std::io::{self, Write, stdout};

fn main()
{
    // // screen whereon the `Crossterm` methods will be executed.
    // let crossterm = Crossterm::new();
    // let color = crossterm.color();
    // let cursor = crossterm.cursor();
    // let terminal = crossterm.terminal();
    // let input = crossterm.input();

    // let mut terminal = terminal();

    // // Clear all lines in terminal;
    // terminal.clear(ClearType::All);
    // // Clear all cells from current cursor position down.
    // terminal.clear(ClearType::FromCursorDown);
    // // Clear all cells from current cursor position down.
    // terminal.clear(ClearType::FromCursorUp);
    // // Clear current line cells.
    // terminal.clear(ClearType::CurrentLine);
    // // Clear all the cells until next line.
    // terminal.clear(ClearType::UntilNewLine);

    // // Get terminal size
    // let (width, height) = terminal.terminal_size();
    // print!("X: {}, y: {}", width, height);

    // // Scroll down, up 10 lines.
    // terminal.scroll_down(10);
    // terminal.scroll_up(10);

    // // Set terminal size (width, height)
    // terminal.set_size(10, 10);

    // // exit the current process.
    // terminal.exit();

    // // write to the terminal whether you are on the main screen or alternate screen.
    // terminal.write("Some text\n Some text on new line");
}
