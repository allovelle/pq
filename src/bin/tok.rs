fn main() -> PqResult<()>
{
    let tokens = tokenize("source\u{1F600}")?;
    println!("Tokens: {tokens:?}");
    Ok(())
}

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    range::Range,
};

use thiserror::Error;

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    Io(#[from] std::io::Error),
    LexErr(#[from] LexErr),
}

pub type PqResult<T> = Result<T, PqErr>;

#[derive(Debug, Error)]
pub enum LexErr
{
    #[error("invalid token {0}")]
    InvalidToken(char),

    #[error(
        "invalid state transition: can't transition from state {0:?} with \
        character {1:?} to any known state"
    )]
    InvalidStateTransition(State, char),

    #[error("unexpected character {0}")]
    UnexpectedCharacter(char),

    #[error("unexpected end of character stream")]
    EndOfStream,

    #[error("create token from empty buffer")]
    EmptyAccumulationBuffer,

    #[error("unexpected integer `{0}`")]
    InvalidInteger(String),

    #[error("unexpected decimal `{0}`")]
    InvalidDecimal(String),

    #[error("unexpected boolean `{0}`")]
    InvalidBoolean(String),
}

pub fn tokenize(source: &str) -> PqResult<()>
{
    let code_buffer: Vec<_> = source.chars().collect();

    let transitions = state_transition_table();
    let mut curr = BEG;
    let mut buf = String::with_capacity(32);
    let mut toks: Vec<Tok> = Vec::with_capacity(source.len());

    println!("\u{1F600}");
    println!(
        "{:w$} {:4} {:w$} {:w$} {:w$}",
        "From",
        "Curr Ch",
        "To",
        "Then",
        "Buf",
        w = State::max_variant_name(),
    );

    for ch in source.chars().chain("\0".chars())
    {
        let (next, tok_buf_act) = match transitions.get(&(curr, ch))
        {
            Some(act) => act,
            None => return Err(LexErr::InvalidStateTransition(curr, ch).into()),
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
            match curr
            {
                BEG | END | GAP | GAP_OP | SNG | DEC0 =>
                {
                    eprintln!("Error: invalid token {curr:?}");
                    return Err(LexErr::InvalidStateTransition(curr, ch).into());
                }
                _ => (),
            }

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
    (SYM, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ", SYM, ACC),
    (SYM, "\0", END, FIN),
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

fn token_split_out_learn_them()
{
    let full_unicode_range = '\u{0020}' .. '\u{10FFFF}';

    // character
    //     '0020' . '10FFFF' - '"' - '\'
    let json_character =
        &HashSet::from_iter(full_unicode_range) - &HashSet::from(['"', '\\']);

    let json_string = ('"', json_character, '"');

    trait TokChar {}
    impl TokChar for char {}
    impl TokChar for Range<char> {}
    impl TokChar for &str {}
    impl TokChar for HashSet<char> {}

    // TODO: Char is `in <this> range and not in <that> range`

    /// State transitions are locked to character iteration.
    /// [curr state][ch][next state][tok & buf act]
    const STATE_TRANSITION_TABLE: &[(State, &str, State, Act)] = &[
        (BEG, "\0", END, IGN),
        (BEG, "\n \t\r", BEG, IGN),
        (BEG, '\u{0020}' .. '\u{10FFFF}', SYM, ACC),
        // (BEG, valid_symbols, SYM, ACC),
        (BEG, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ", SYM, ACC),
        (BEG, "0123456789", SYM, ACC),
        (BEG, "-_|/?*&%$#@!X+=;", SYM, ACC),
        (SYM, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ", SYM, ACC),
        (SYM, "\0", END, FIN),
    ];
}
