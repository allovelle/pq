// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output
// ! The goal is to not need serde_json for input or output

use crossterm::style::Stylize;
use pq::txt::utf8_char_on;
use std::collections::{HashMap, HashSet};
use std::default;
use std::ops::{Range, RangeBounds, RangeInclusive, Sub};
use thiserror::Error;
use {Accept::*, Act::*, State::*};

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
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

/// Exists because [RangeInclusive<char>] is not [Copy].
#[derive(Debug, Clone, Copy)]
struct CharRangeInclusive
{
    from: char,
    onto: char,
}

impl CharRangeInclusive
{
    /// **Exists because [From] & [Into] are not `const`**
    pub const fn from(value: RangeInclusive<char>) -> Self
    {
        Self { from: *value.start(), onto: *value.end() }
    }

    /// **Exists because [From] & [Into] are not `const`**
    pub const fn into(self) -> RangeInclusive<char>
    {
        self.from ..= self.onto
    }
}

impl From<RangeInclusive<char>> for CharRangeInclusive
{
    fn from(value: RangeInclusive<char>) -> Self
    {
        Self { from: *value.start(), onto: *value.end() }
    }
}

impl From<CharRangeInclusive> for RangeInclusive<char>
{
    fn from(value: CharRangeInclusive) -> Self
    {
        value.from ..= value.onto
    }
}

#[derive(Debug, Clone, Copy)]
struct Row
{
    /// The state performing an examination for transition determination
    from: State,
    /// Current character is within this range (matches first)
    accept: CharRangeInclusive,
    /// Current character is outside this range (matches second)
    except: CharRangeInclusive,
    /// The state to transition to if accept & except ranges match on char
    onto: State,
    /// Discard or accumulate current character, append to or clear buffer,
    /// and record new token
    action: Act,
}

impl Row
{
    const fn zero() -> Self
    {
        Self::new(Begin, '\0' ..= '\0', '\0' ..= '\0', Begin, Ign)
    }

    const fn new(
        from: State,
        accept: RangeInclusive<char>,
        except: RangeInclusive<char>,
        onto: State,
        action: Act,
    ) -> Self
    {
        let accept = CharRangeInclusive::from(accept);
        let except = CharRangeInclusive::from(except);
        Self { from, accept, except, onto, action }
    }

    fn matches(&self, state: State, ch: char) -> bool
    {
        let acc: RangeInclusive<char> = self.accept.into();
        let exc: RangeInclusive<char> = self.except.into();
        state == self.from && acc.contains(&ch) && !exc.contains(&ch)
    }
}

// StateFrom, Accept, Except
// HashMap<(State, RangeInclusive<char>, RangeInclusive<char>)>
// If table.contains(&(curr_state, ))

pub fn tokenize(source: &str) -> PqResult<()>
{
    let code_buffer: Vec<_> = source.chars().collect();

    let transitions = state_transition_table();
    let mut curr = Begin;
    let mut buf = String::with_capacity(32);
    let mut toks: Vec<Tok> = Vec::with_capacity(source.len());

    println!(
        "{:width$} {:4} {:width$} {:width$} {:width$}",
        "From",
        "Curr Ch",
        "To",
        "Then",
        "Buf",
        width = State::max_variant_name(),
    );

    for ch in source.chars().chain("\0".chars())
    {
        // !let (next, tok_buf_act) = match transitions.get(&(curr, ch))
        // {
        //     Some(act) => act,
        //     None => return Err(LexErr::InvalidStateTransition(curr, ch).into()),
        // };

        // ! println!(
        //     "{:w$} {:4} {:w$} {:w$} {:w$}",
        //     format!("{curr:?}"),
        //     format!("{ch:?}"),
        //     format!("{next:?}"),
        //     format!("{tok_buf_act:?}"),
        //     format!("{buf:?}"),
        //     w = State::max_variant_name(),
        // );

        // ! if let Act::Tok | Act::Fin = tok_buf_act
        // {
        //     // TODO: this is a call for having an action for Error since this is
        //     // TODO: handling language specific transition stuff in the harness
        //     // if let Begin | End | Gap | GapOperator | Sign | Dec0 = curr
        //     // {
        //     //     println!("Error: invalid token {curr:?}");
        //     //     return Err(LexErr::InvalidStateTransition(curr, ch).into());
        //     // }

        //     buf.clear();
        // }

        // ! if let Act::Acc | Act::Fin = tok_buf_act
        // {
        //     buf.push(ch);
        // }

        // ! curr = *next;
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
    // TODO: can add a LexErr(&'static str) variant for manual state filtering
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum State
{
    Begin,
    Obj0,
    Arr0,
    Txt0,
    Nil,
    Bit,
    Num,
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

impl State
{
    fn max_variant_name() -> usize
    {
        let mut longest = 0;
        for (from, _, _, onto, _) in STATE_TRANSITION_TABLE.into_iter()
        {
            let from_len = format!("{from:?}").len();
            let onto_len = format!("{onto:?}").len();
            longest = longest.max(from_len.max(onto_len));
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

/// **Allowed & disallowed patterns for state transitions.**
#[derive(Debug, Clone, Copy)]
pub enum Accept
{
    /// **Explicitly listed elements**
    AnyOf(&'static str),
    /// **Elements explicitly within this range. Equivalent to `char ..= char`**
    Within(char, char),
    /// **Ignored accept/except bound**
    Unused,
}

/// **State transitions are locked to character iteration. Essentially, check
/// that char is in this range or set and also not in this range or set.**
/// `[curr state][accept ch range][except ch range][next state][tok & buf act]`
const STATE_TRANSITION_TABLE: &[(State, Accept, Accept, State, Act)] = &[
    (Begin, AnyOf("\0"), Unused, End, Ign),
    (Begin, AnyOf("{"), Unused, Obj0, Ign),
    (Begin, AnyOf("["), Unused, Arr0, Ign),
    (Begin, AnyOf("\""), Unused, Txt0, Ign),
    (Begin, AnyOf("\n \t\r"), Unused, Begin, Ign),
    (Symbol, Within('\u{0020}', '\u{10FFFF}'), AnyOf("\"\\"), Symbol, Acc),
];

/// Each row represents at least one tokenizer state transition. When examining
/// a range of allowed characters that excludes a set of speciied characters
/// (that are not a consecutive range), one new row is created for each of the
/// specified exclusion characters. This also works the other way around for
/// disallowed range & allowed specified character set.
const fn max_state_transitions() -> usize
{
    let mut transitions = 0;
    let mut udx = 0;
    while udx < STATE_TRANSITION_TABLE.len()
    {
        let (_, accept, except, ..) = &STATE_TRANSITION_TABLE[udx];
        udx += 1;

        if let (AnyOf(chars), Unused) | (Within(..), AnyOf(chars)) =
            (accept, except)
        {
            transitions += chars.len(); // Range * len(chars) = len(chars) rows
        }
        else if let (Within(..), Within(..)) | (Within(..), Unused) =
            (accept, except)
        {
            transitions += 1; // Ranges pair counts as one row
        }
        else
        {
            panic!("this is an invalid state transition combo")
        }
    }
    transitions
}

const fn array_test(buffer: &[u8])
{
    utf8_char_on("buffer".as_bytes(), 0);
}

const fn state_transition_table() -> [Row; max_state_transitions()]
{
    const EXPANDED_TABLE_LEN: usize = max_state_transitions();
    let mut rows: [Row; EXPANDED_TABLE_LEN] = [Row::zero(); EXPANDED_TABLE_LEN];

    let mut udx_row = 0;
    while udx_row < EXPANDED_TABLE_LEN
    {
        let (from, accept, except, onto, act) = STATE_TRANSITION_TABLE[udx_row];

        match (accept, except)
        {
            (AnyOf(chars), Unused) =>
            {
                // If it's any of these characters, add a new 'accept' range for
                // each one since they are single element not a range
                let mut udx_ch = 0;
                while let Some(ch) = utf8_char_on(chars.as_bytes(), udx_ch)
                    && udx_ch < chars.len()
                {
                    udx_ch += ch.len_utf8();

                    let accept_range = ch ..= ch;
                    let unused_range = '\0' ..= '\0';
                    let row =
                        Row::new(from, accept_range, unused_range, onto, act);

                    rows[udx_row] = row;
                }
            }
            (Within(..), AnyOf(chars)) =>
            {
                // TODO: Split the accept range such that there is one copy that
                // TODO: excludes a ch for each ch in AnyOf.

                // If it's any of these characters, add a new 'except' range for
                // each one since they are single element not a range
                let mut udx_ch = 0;
                while let Some(ch) = utf8_char_on(chars.as_bytes(), udx_ch)
                    && udx_ch < chars.len()
                {
                    udx_ch += ch.len_utf8();

                    let unused_range = '\0' ..= '\0';
                    let except_range = ch ..= ch;
                    let row =
                        Row::new(from, unused_range, except_range, onto, act);

                    rows[udx_row] = row;
                }
            }
            (Within(from_in, upto_in), Within(from_ou, upto_ou)) =>
            {
                // TODO: Split the accept range such that there is one copy that
                // TODO: excludes a ch for each ch in AnyOf.
                let accept_range =
                    CharRangeInclusive::from(from_in ..= upto_in);
                let except_range =
                    CharRangeInclusive::from(from_ou ..= upto_ou);
                // let row = Row::new(from, accept_range, except_range, onto, act);
                let row = Row {
                    from,
                    accept: accept_range,
                    except: except_range,
                    onto,
                    action: act,
                };
                rows[udx_row] = row;
            }
            (Within(..), Unused) => todo!(),
            _ => panic!("this is an invalid state transition combo"),
        }

        udx_row += 1;
    }

    // TODO: Create ranges from each of the chars involved:
    let accept = AnyOf("abcd"); // TODO: a .. a, c .. d
    let except = AnyOf("b"); // TODO: For each exception, split range

    rows
}

// TODO:  1. Compacted table (with text, ranges, etc.)
// TODO:  2. Generated const table expanded with only ranges

// const fn expand_state_transition_table()
// {
//     const N: usize = determine_compact_state_transition_table_allocation();
//     let rows: [Row; N];

//     let mut rows: &mut [usize] = &mut [];

//     for (from, accept, except, to, action) in STATE_TRANSITION_TABLE.into_iter()
//     {
//         match (accept, except)
//         {
//             (AnyOf(chars), Unused) =>
//             {
//                 for ch in chars.chars()
//                 {
//                     // tab.insert((*from, ch), (*to, *action));
//                 }
//             }
//             (Within(r1), AnyOf(_)) =>
//             {
//                 // TODO: Split the accept range such that there is one copy that
//                 // TODO: excludes a ch for each ch in AnyOf.
//             }
//             (Within(r1), Within(r2)) =>
//             {
//                 // TODO: Split the accept range such that there is one copy that
//                 // TODO: excludes a ch for each ch in AnyOf.
//             }
//             (Within(r1), Unused) => todo!(),
//             _ => unreachable!("this is an invalid state transition combo"),
//         }
//     }
// }

fn main() -> PqResult<()>
{
    if let Err(err) = tokenize("    \t\r\n")
    {
        println!("{}", format!("{err}").red());
    }
    Ok(())
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
}
