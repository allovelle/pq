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
use std::ops::{Range, RangeBounds, RangeInclusive, Sub};
use std::{default, fmt};
use thiserror::Error;
use {Accept::*, Act::*, State::*};

/// A macro for early returns based on a condition.
///
/// # Examples
///
/// ```rust
/// use pq::ret_if;
///
/// fn demo(x: i32) -> i32 {
///     ret_if!(x < 0, 0);      // return 0 if x is negative
///     ret_if!(x == 42, 99);   // return 99 if x is 42
///     x + 1
/// }
///
/// assert_eq!(demo(-5), 0);
/// assert_eq!(demo(42), 99);
/// assert_eq!(demo(7), 8);
/// ```
macro_rules! ret_if {
    ($cond:expr, $val:expr) => {
        if $cond
        {
            return $val;
        }
    };
}

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
#[derive(Clone, Copy)]
struct CharRangeInclusive
{
    from: char,
    onto: char,
}

mod impl_char_range_inclusive
{
    use super::*;

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

        #[inline]
        pub const fn contains(&self, ch: char) -> bool
        {
            self.from <= ch && ch <= self.onto
        }
    }

    /// Compare [CharRangeInclusive] == [RangeInclusive<char>]
    impl PartialEq<RangeInclusive<char>> for CharRangeInclusive
    {
        fn eq(&self, other: &RangeInclusive<char>) -> bool
        {
            self.from == *other.start() && self.onto == *other.end()
        }
    }

    /// Compare [RangeInclusive<char>] == [CharRangeInclusive]
    impl PartialEq<CharRangeInclusive> for RangeInclusive<char>
    {
        fn eq(&self, other: &CharRangeInclusive) -> bool
        {
            *self.start() == other.from && *self.end() == other.onto
        }
    }

    /// Compare [CharRangeInclusive] == [CharRangeInclusive]
    impl PartialEq for CharRangeInclusive
    {
        fn eq(&self, other: &Self) -> bool
        {
            self.from == other.from && self.onto == other.onto
        }
    }

    impl fmt::Debug for CharRangeInclusive
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            if f.alternate()
            {
                f.debug_struct("CharRangeInclusive")
                    .field("from", &self.from)
                    .field("onto", &self.onto)
                    .finish()
            }
            else
            {
                ret_if!(
                    *self == ('\0' ..= '\0'),
                    f.write_fmt(format_args!("{:^8}", "--"))
                );

                let from = format!("{:?}", self.from);
                let onto = format!("{:?}", self.onto);
                f.write_fmt(format_args!(
                    "{:>2} .. {:>2}",
                    from.trim_matches('\''),
                    onto.trim_matches('\'')
                ))
            }
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
        Self::new(BEG, '\0' ..= '\0', '\0' ..= '\0', BEG, IGN)
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

    /// Checks that this row's state matches the current state of the tokenizer.
    /// Checks that the
    fn matches(&self, state: State, ch: char) -> bool
    {
        ret_if!(state != self.from, false);

        let allow = self.accept.contains(ch);
        let enable_except = self.except != ('\0' ..= '\0');
        let deny = self.except.contains(ch) && enable_except;

        allow && !deny
    }
}

pub fn tokenize(source: &str) -> PqResult<()>
{
    let transitions: [Row; _] = state_transition_table();
    let mut curr = BEG;
    let mut buf = String::with_capacity(32);
    let mut toks: Vec<Tok> = Vec::with_capacity(source.len());

    // TODO: Fix the widths :D
    const W1: usize = 6;
    const W2: usize = 5;

    println!(
        "{:W1$}{:W2$}{:W1$}{:W2$}{:W2$}",
        "From", "Char", "To", "Then", "Buf",
    );

    for ch in source.chars().chain("\0".chars())
    {
        // Find the transition for the current state and character
        let row = match transitions.iter().find(|row| row.matches(curr, ch))
        {
            Some(row) => row,
            None => return Err(LexErr::InvalidStateTransition(curr, ch).into()),
        };

        println!(
            "{:W1$}{:W2$}{:W1$}{:W2$}{:W2$}",
            format!("{curr:?}"),
            format!("{ch:?}"),
            format!("{:?}", row.onto),
            format!("{:?}", row.action),
            format!("{buf:?}"),
        );

        // The buffer may be able to be converted into a token
        if let Act::TOK | Act::FIN = row.action
        {
            if let Some(tok) = row.from.finalize(&buf)
            {
                toks.push(tok);
            }

            // match curr
            // {
            //     // TODO: this is a call for having an action for Error since this is
            //     // TODO: handling language specific transition stuff in the harness
            //     Begin | End | Gap | GapOperator | Sign | Dec0 =>
            //     {
            //         // ! Should not be doing error recovery here
            //         println!("Error: invalid token {curr:?}");
            //         return Err(LexErr::InvalidStateTransition(curr, ch).into());
            //     }

            //     SI => toks.push(Tok::Sgn(match buf.parse()
            //     {
            //         Ok(num) => num,
            //         Err(_) => return eprintln!("Invalid signed int {buf:?}"),
            //     })),
            // };

            buf.clear();
        }

        // If no token can be constructed, continue accumulating the buffer
        if let Act::ACC | Act::FIN = row.action
        {
            buf.push(ch);
        }

        curr = row.onto;
    }

    println!();
    println!("Code: `{source}`");
    println!("Tokens: {toks:?}");

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Act
{
    /// Finalize buffer as token *first*, then accumulate current character.
    FIN,
    /// Finalize buffer as token, then ignore current character
    TOK, // ? Can this be removed if prev state is tracked?
    /// Accumulate current character.
    ACC,
    /// Ignore the current character.
    IGN,
    // TODO: can add a LexErr(&'static str) variant for manual state filtering
    // TODO: Err(&'static str),
}

/// Some of these states produce tokens when finalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum State
{
    BEG,
    OBJ0,
    ARR0,
    TXT0,
    NIL,
    BIT,
    NUM,
    GAP,
    GAPOPER,
    OPER,
    SNG,
    SYM,
    DEC0,
    DEC1,
    END,
}

impl State
{
    fn finalize(self, buffer: &String) -> Option<Tok>
    {
        match self
        {
            Self::BEG => todo!(),
            Self::OBJ0 => todo!(),
            Self::ARR0 => todo!(),
            Self::TXT0 => todo!(),
            Self::NIL => todo!(),
            Self::BIT => todo!(),
            Self::NUM => todo!(),
            Self::GAP => todo!(),
            Self::GAPOPER => todo!(),
            Self::OPER => todo!(),
            Self::SNG => todo!(),
            Self::SYM => todo!(),
            Self::DEC0 => todo!(),
            Self::DEC1 => todo!(),
            Self::END => todo!(),
        }
    }

    fn max_variant_name() -> usize
    {
        let mut longest = 0;
        for (from, _, _, onto, _) in STATE_TRANSITION_TABLE.iter()
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
    (BEG, AnyOf("\0"), Unused, END, IGN),
    (BEG, AnyOf("\n \t\r"), Unused, BEG, IGN),
    (BEG, Within('0', '9'), Unused, NUM, ACC),
    (NUM, Within('0', '9'), Unused, NUM, ACC),
    (NUM, AnyOf("\0"), Unused, END, TOK),
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

const fn state_transition_table() -> [Row; max_state_transitions()]
{
    const EXPANDED_TABLE_LEN: usize = max_state_transitions();
    let mut rows: [Row; EXPANDED_TABLE_LEN] = [Row::zero(); EXPANDED_TABLE_LEN];

    // Slow index is input table, fast index is output table because it adds
    // more rows than the input table. Destination index doesn't matter since
    // lookup will be O(N) anyway.
    let (mut slow, mut fast) = (0, 0);

    while slow < STATE_TRANSITION_TABLE.len()
    {
        let (from, accept, except, onto, act) = STATE_TRANSITION_TABLE[slow];
        slow += 1;

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

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
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

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
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
                rows[fast] = row;
                fast += 1;
            }
            (Within(from_in, upto_in), Unused) =>
            {
                let accept = CharRangeInclusive::from(from_in ..= upto_in);
                let except = CharRangeInclusive::from('\0' ..= '\0');
                let row = Row { from, accept, except, onto, action: act };
                rows[fast] = row;
                fast += 1;
            }
            _ => panic!("this is an invalid state transition combo"),
        }
    }

    assert!(fast == rows.len(), "Sanity check: were offsets correct?");

    rows
}

fn emit_table(table: &[Row])
{
    const W1: usize = 10;
    const W2: usize = 6;

    println!(
        "| {:W2$} | {:W1$} | {:W1$} | {:W2$} | {:W2$} |",
        "From", "Accept", "Except", "Onto", "Action",
    );

    for row in table
    {
        println!(
            "| {:W2$} | {:W1$} | {:W1$} | {:W2$} | {:W2$} |",
            format!("{:?}", row.from),
            format!("{:?}", row.accept),
            format!("{:?}", row.except),
            format!("{:?}", row.onto),
            format!("{:?}", row.action),
        );
    }
    println!();
}

fn main() -> PqResult<()>
{
    emit_table(&state_transition_table()[..]);

    if let Err(err) = tokenize("\t \r\n12")
    {
        println!("{}", format!("{err}").red());
    }

    Ok(())
}
