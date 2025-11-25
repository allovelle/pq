use std::io;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

pub struct StdinReader
{
    receiver: mpsc::Receiver<String>,
}

impl StdinReader
{
    pub fn new() -> Self
    {
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut stdin = tokio::io::stdin();
            let mut buffer = [0u8; 1024];

            loop
            {
                match stdin.read(&mut buffer).await
                {
                    Ok(0) => break, // EOF
                    Ok(n) =>
                    {
                        let text =
                            String::from_utf8_lossy(&buffer[.. n]).to_string();
                        if tx.send(text).await.is_err()
                        {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) =>
                    {
                        eprintln!("Error reading stdin: {}", e);
                        break;
                    }
                }
            }
        });

        Self { receiver: rx }
    }

    pub fn try_read(&mut self) -> Option<String>
    {
        self.receiver.try_recv().ok()
    }
}

impl Default for StdinReader
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[tokio::main]
async fn main() -> io::Result<()>
{
    let mut reader = StdinReader::new();
    println!("StdinReader started. Type something...");

    loop
    {
        // Example of how to do this:
        // tokio::time::sleep(Duration::from_millis(1000)).await;

        while let Some(input) = reader.try_read()
        {
            println!("... {}", input);
        }
    }
}

mod tok
{
    use std::collections::HashMap;

    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error("Pique Error")]
    pub enum PqErr
    {
        Io(#[from] std::io::Error),
    }

    pub type PqResult<T> = Result<T, PqErr>;

    #[derive(Debug, Error)]
    pub enum LexErr
    {
        #[error("invalid token {0}")]
        InvalidToken(char),

        #[error("unexpected character {0}")]
        UnexpectedCharacter(char),

        #[error("unexpected end of character stream")]
        EndOfStream,
    }

    /*
    pub fn tokenize(source: &str) -> PqResult<()>
    {
        let source = String::from("éllo World!");
        let code_buffer1 = TxtView::from(source.as_str());
        let ch = code_buffer1.get_char(CharIndex::from(0)).unwrap();
        println!("{ch}");
        let ch = code_buffer1.get_char(CharIndex::from(1)).unwrap();
        println!("{ch}");

        let code_buffer = TxtView::from(&source);

        let transitions = state_transition_table();
        let mut curr = BEG;
        let mut buf = String::with_capacity(32);
        let mut toks: Vec<Tok> = Vec::with_capacity(source.len());

        println!("---------");
        for udx in 0 .. code_buffer.char_len
        {
            let ch = code_buffer.get_char(udx.into()).unwrap();

            eprint!("{ch}");

            let (next, tok_buf_act) = match transitions.get(&(curr, ch))
            {
                Some(act) => act,
                None =>
                {
                    eprintln!("\nUnexpected char: {ch:?} {curr:?}\n");
                    let line_range =
                        code_buffer.line_containing(udx.into()).unwrap();
                    let line = code_buffer.slice(line_range.clone()).unwrap();
                    eprintln!("{}", line.trim_start_matches('\n').trim_end());
                    let pad_offset = udx - line_range.start.0.0 - 1;
                    eprintln!("{}^", " ".repeat(pad_offset));

                    return Err(LexErr::UnexpectedCharacter(ch).into());
                }
            };
        }
        println!("---------");

        for ch in source.chars().chain("\0".chars())
        {
            let (next, tok_buf_act) = match transitions.get(&(curr, ch))
            {
                Some(act) => act,
                None =>
                {
                    eprintln!("\nUnexpected char: {ch:?} {curr:?}");
                    eprintln!("{source}");
                    eprintln!("{}^", " ".repeat(4));

                    return Err(LexErr::UnexpectedCharacter(ch).into());
                }
            };

            println!(
                "{:w$} {:4} {:w$} {:w$} {:w$}",
                format!("{curr:?}"),
                format!("{ch:?}"),
                format!("{next:?}"),
                format!("{tok_buf_act:?}"),
                format!("{buf:?}"),
                w = State::max_variant_name(),
            );

            if let Act::Tok | Act::Fin = tok_buf_act
            {
                /*
                match curr
                {
                    BEG | END | GAP | GAP_OP | SNG | DEC0 =>
                    {
                        return eprintln!("Error: invalid token {curr:?}")
                    }
                    OP => toks.push(Tok::Symbol(match buf.chars().next()
                    {
                        Some(op) => op.to_string(),
                        None => return eprintln!("Error: empty buffer"),
                    })),
                    SI => toks.push(Tok::Signed(match buf.parse()
                    {
                        Ok(num) => num,
                        Err(_) => return eprintln!("Invalid signed int {buf:?}"),
                    })),
                    UI => toks.push(Tok::Unsigned(match buf.parse()
                    {
                        Ok(num) => num,
                        Err(_) => return eprintln!("Invalid unsigned int {buf:?}"),
                    })),
                    DEC1 => toks.push(Tok::Decimal(match buf.parse()
                    {
                        Ok(num) => num,
                        Err(_) => return eprintln!("Invalid decimal {buf:?}"),
                    })),
                }

                */
                buf.clear();
            }

            if let Act::Acc | Act::Fin = tok_buf_act
            {
                buf.push(ch);
            }

            curr = *next;
        }

        println!();
        println!("Code: `{source}`");
        println!("Tokens: {toks:?}");

        Ok(())
    }
     */

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(u8)]
    enum Act
    {
        /// Finalize buffer as token *first*, then accumulate current character.
        Fin,
        /// Finalize buffer as token, then ignore current character
        Tok, // ? Can this be removed if prev state is tracked?
        /// Accumulate current character.
        Acc,
        /// Ignore the current character.
        Ign,
    }

    // TODO: #[derive(Debug, Clone, Copy, PartialEq)]
    // TODO: enum Act
    // TODO: {
    // TODO:     /// Record that a state transition took place. Add a character to the buffer
    // TODO:     /// used to produce a final token.
    // TODO:     Acc, // Token: record state transition. Buffer: record current character
    // TODO:     /// Ignore state transitions when the to and from states are the same: no
    // TODO:     /// reason to lose history for nothing. Ignore characters like spaces or
    // TODO:     /// comment characters
    // TODO:     Ign, // Token: ignore state transition. Buffer: ignore current character
    // TODO:     /// Finalize a token and record the state transition. Clear the buffer.
    // TODO:     Fin, // Token: record state transition & generate token. Buffer: clear
    // TODO:     /// For token state actions, this bubbles up an error but tries to transfer
    // TODO:     /// to a new state to recover and therefore generate more error messages.
    // TODO:     /// For buffer actions, panic immediately with a critical error message.
    // TODO:     Err(&'static str), // Token: bubble up error message. Buffer: panic
    // TODO: }

    const FIN: Act = Act::Fin;
    const TOK: Act = Act::Tok;
    const ACC: Act = Act::Acc;
    const IGN: Act = Act::Ign;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(u8)]
    enum State
    {
        Begin,
        Gap,
        GapOperator,
        Operator,
        Sign,
        Signed,
        Unsigned,
        Symbol,
        Dec0,
        Dec1,
        End,
    }

    const BEG: State = State::Begin;
    const GAP: State = State::Gap;
    const GAP_OP: State = State::GapOperator;
    const OP: State = State::Operator;
    const SNG: State = State::Sign;
    const SI: State = State::Signed;
    const UI: State = State::Unsigned;
    const DEC0: State = State::Dec0;
    const DEC1: State = State::Dec1;
    const SYM: State = State::Symbol;
    const END: State = State::End;

    impl State
    {
        fn max_variant_name() -> usize
        {
            let mut longest = 0;
            for ((state, ..), ..) in state_transition_table().into_iter()
            {
                longest = longest.max(format!("{state:?}").len());
            }
            longest
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Tok
    {
        Symbol(String),
        Text(String),
        Colon,            // :
        Dot,              // .
        Throw,            // ^^a
        Bind,             // a:: b
        NamespaceSlashes, // \\a
        PathSlash,        // \a
        AngleOpen,        // <
        AngleClose,       // >
        ParenOpen,        // (
        ParenClose,       // )
        SquareOpen,       // [
        SquareClose,      // ]
        BlockOpen,        // {
        BlockClose,       // }
        Signed(i32),      // +0 -0
        Unsigned(u32),    // ~0
        Decimal(f64),     // +0.0 -0.0
    }

    // /// State transitions are locked to character iteration.
    // /// [curr state][ch][next state][tok & buf act]
    // const STATE_TRANSITION_TABLE: &[(State, &str, State, Act)] = &[
    //     (BEG, "\0", END, IGN),
    //     (BEG, "\n \t\r", BEG, IGN),
    //     (BEG, "+-", SNG, ACC),
    //     (BEG, "~", UI, IGN),
    //     (SNG, "\0", END, ACC),
    //     (SNG, "\n \t\r", SNG, IGN),
    //     (SNG, "0123456789", SI, ACC),
    //     (SI, "\0", END, FIN),
    //     (SI, "\n \t\r", GAP_OP, TOK),
    //     (SI, "0123456789", SI, ACC),
    //     (SI, "+-*/", OP, FIN),
    //     (SI, "eE", DEC0, ACC),
    //     (UI, "\0", END, FIN),
    //     (UI, "\n \t\r", GAP_OP, TOK), // ? Remove buf states if prev state tracked?
    //     (UI, "0123456789", UI, ACC),
    //     (UI, "+-*/", OP, FIN),
    //     (DEC0, "+-", DEC1, ACC), // * Use a buffer state for staged states
    //     (DEC1, "\0", END, FIN),
    //     (DEC1, "\n \t\r", GAP_OP, TOK),
    //     (DEC1, "0123456789", DEC1, ACC),
    //     (DEC1, "+-*/", OP, FIN),
    //     (OP, "\n \t\r", BEG, TOK), // ? Could also go to GAP
    //     (OP, "+-", SNG, FIN),
    //     (OP, "~", UI, TOK),
    //     (GAP_OP, "\n \t\r", GAP_OP, IGN),
    //     (GAP_OP, "+-*/", OP, ACC),
    // ];

    const ALPHAS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const DIGITS: &str = "0123456789";
    const ALPHA_DIGITS: &str = concat!(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "0123456789"
    );
    const VALID_SYMBOLS: &str = concat!(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "0123456789",
        "-_|/?*&%$#@!X+=;"
    );

    /// State transitions are locked to character iteration.
    /// [curr state][ch][next state][tok & buf act]
    const STATE_TRANSITION_TABLE: &[(State, &str, State, Act)] = &[
        (BEG, "\0", END, IGN),
        (BEG, "\n \t\r", BEG, IGN),
        // (BEG, valid_symbols, SYM, ACC),
        (BEG, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ", SYM, ACC),
        (BEG, "0123456789", SYM, ACC),
        (BEG, "-_|/?*&%$#@!X+=;", SYM, ACC),
    ];

    fn state_transition_table() -> HashMap<(State, char), (State, Act)>
    {
        STATE_TRANSITION_TABLE
            .iter()
            .copied()
            .flat_map(|(curr, mat, next, tok_buf_act)| {
                mat.chars().map(move |c| ((curr, c), (next, tok_buf_act)))
            })
            .collect()
    }
}
