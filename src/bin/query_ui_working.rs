use crossterm::event::{KeyEvent, KeyModifiers};
use crossterm::{ExecutableCommand, cursor, terminal};
use serde_json::Value;
use std::io::{Write, stdout};

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

struct Query
{
    commands: Vec<Cmd>,
}

impl Query
{
    pub fn new() -> Self
    {
        Self { commands: Default::default() }
    }

    fn parse(&mut self)
    {
        // Replace query commands
        let mut x = std::collections::HashMap::new();
        x.insert(&213, 213);
    }

    /// Goes to next sibling else first child
    pub fn next_sibling(&self) -> Option<u8>
    {
        let node: Option<u8> = None;
        node.or(self.goto_child())
    }

    /// Goes to previous sibling else parent
    pub fn prev_sibling(&self) -> Option<u8>
    {
        let node: Option<u8> = None;
        node.or(self.goto_parent())
    }

    /// Goes to parent else root
    pub fn goto_parent(&self) -> Option<u8>
    {
        let node: Option<u8> = None;
        node
    }

    /// Goes to first child else next sibling
    pub fn goto_child(&self) -> Option<u8>
    {
        let node: Option<u8> = None;
        node.or(self.next_sibling())
    }

    /// Needs the json to know the next keys/indices. Effectively updates the
    /// query by performing it.
    fn push(&mut self, _from: Value)
    {
        // TODO: Inc index if applicable
        // TODO: Find next key
    }

    fn pop(&mut self)
    {
        self.commands.pop();
    }
}

enum Cmd
{
    Key(String),
    Index(usize),
}

fn main() -> Result<(), Box<dyn Error>>
{
    // Get the json
    let text = std::fs::read_to_string("json3.json")?;
    let json: Value = serde_json::from_str(&text)?;
    let mut stdout = stdout();

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

        fn query_json(mut json: Value, query: Vec<&str>) -> Option<Value>
        {
            for key in query
            {
                json = json.get(key.trim())?.clone();
            }
            Some(json)
        }

        let query: Vec<_> =
            input.split(".").filter(|q| !q.is_empty()).collect();
        write!(stdout, "{:?}", query)?;

        let filtered = query_json(json.clone(), query).unwrap_or(json.clone());
        let formatted = serde_json::to_string_pretty(&filtered)?;

        stdout.execute(cursor::MoveToColumn(0))?;
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
                Event::Key(KeyEvent { code: KeyCode::Tab, .. }) =>
                {
                    input += "";
                }
                Event::Key(KeyEvent { code: KeyCode::BackTab, .. }) =>
                {
                    input += "";
                }
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
