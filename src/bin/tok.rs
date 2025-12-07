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

use crossterm::style::{PrintStyledContent, Stylize};
use pq::txt::{self, utf8_char_on};
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, RangeInclusive};
use std::{fmt, hash};
use strum::*;
use thiserror::Error;
use {Act::*, CharMatch::*, State::*};

fn longest_variant_name<E: VariantNames>() -> usize
{
    E::VARIANTS.iter().map(Deref::deref).map(str::len).max().unwrap_or_default()
}

trait EmitTable {}
trait EmitColumn
{
    fn as_column() -> String;
}

impl<T: fmt::Debug> ToDebug for T {}
trait ToDebug: fmt::Debug
{
    /// Equivalent to `format!("{:?}", thing);`
    fn to_debug(&self) -> String
    {
        format!("{self:?}")
    }

    /// Equivalent to `format!("{:#?}", thing);`
    fn to_long_debug(&self) -> String
    {
        format!("{self:#?}")
    }

    /// A debug view of a debug view (includes the outer quotes)
    fn to_debug_literal(&self) -> String
    {
        format!("{:?}", format!("{}", self.to_debug()))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_left(&self, space: usize) -> String
    {
        format!("{:<space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_right(&self, space: usize) -> String
    {
        format!("{:>space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_center(&self, space: usize) -> String
    {
        format!("{:^space$}", format!("{self:?}"))
    }
}

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

    #[error(transparent)]
    ParseIntErr(#[from] std::num::ParseIntError),
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

/// **Format ASCII & multi-byte codepoints as either their escape-code format
/// `\u{AB12}` or their Unicode codepoint `U+AB12`.**
trait CodepointView
{
    fn fmt_escape(self) -> String;
    fn fmt_unicode(self) -> String;
}

impl CodepointView for char
{
    fn fmt_escape(self) -> String
    {
        format!("\\u{:04X}", self as u32)
    }

    fn fmt_unicode(self) -> String
    {
        format!("U+{:04X}", self as u32)
    }
}

/// Exists because [RangeInclusive<char>] is not [Copy].
#[derive(Clone, Copy, PartialOrd, Eq)]
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
        pub const fn zero() -> Self
        {
            Self::from('\0' ..= '\0')
        }

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

        // #[inline]
        // pub const fn split(self, at: char) -> (Self, Self)
        // {
        //     if at < self.from || at > self.onto
        //     {
        //         return (self, self);
        //     }
        //     if at == self.from && at == self.onto
        //     {
        //         return (Self::zero(), Self::zero());
        //     }
        //     if at == self.from
        //     {
        //         return (self.from, self.to);
        //     }
        //     (self, self)
        // }

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

    impl hash::Hash for CharRangeInclusive
    {
        fn hash<H: hash::Hasher>(&self, state: &mut H)
        {
            self.from.hash(state);
            self.onto.hash(state);
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
                    // f.write_fmt(format_args!("{:^8}", "--"))
                    f.write_fmt(format_args!("{0:>2} .. {0:>2}", "\\0"))
                );

                let spaces_lookup_table = HashMap::from([
                    ('\n', "\\n"),
                    ('\t', "\\t"),
                    ('\r', "\\r"),
                    (' ', "\\_"),
                    ('\'', "`\\'`"),
                    ('\"', "`\"`"),
                    ('\0', "\\0"),
                    ('\\', "`\\`"),
                ]);

                // ! {{
                let from = if spaces_lookup_table.contains_key(&self.from)
                {
                    String::from(spaces_lookup_table[&self.from])
                }
                else if self.from.is_whitespace() || !self.from.is_ascii()
                {
                    format!("{:?}", self.from.fmt_unicode())
                }
                else
                {
                    format!("{:?}", self.from.to_string())
                };

                let from = from.trim_matches('"').trim_matches('\'');
                // ! }}

                // ! {{
                let onto = if spaces_lookup_table.contains_key(&self.onto)
                {
                    String::from(spaces_lookup_table[&self.onto])
                }
                else if self.onto.is_whitespace() || !self.onto.is_ascii()
                {
                    format!("{:?}", self.onto.fmt_unicode())
                }
                else
                {
                    format!("{:?}", self.onto.to_string())
                };

                let onto = onto.trim_matches('"').trim_matches('\'');
                // ! }}

                match self.onto
                {
                    ch if spaces_lookup_table.contains_key(&ch) =>
                    {}
                    ch if !self.onto.is_ascii_whitespace()
                        || !self.onto.is_ascii() =>
                    {}
                    _ => (),
                }

                f.write_fmt(format_args!("{:>2} .. {:>2}", from, onto))
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash)]
struct Row
{
    /// The state performing an examination for transition determination
    from: State,
    /// Current character is within this range or set (matches first)
    accept: CharMatch,
    /// Current character is outside this range or set (matches second)
    except: CharMatch,
    /// The state to transition to if accept & except ranges match on char
    onto: State,
    /// Discard or accumulate current character, append to or clear buffer,
    /// and record new token
    act: Act,
}

impl Row
{
    const fn zero() -> Self
    {
        Self::new(BEG, Unused, Unused, BEG, IGN)
    }

    const fn new(
        from: State,
        accept: CharMatch,
        except: CharMatch,
        onto: State,
        action: Act,
    ) -> Self
    {
        Self { from, accept, except, onto, act: action }
    }

    /// Checks that this row's state matches the current state of the tokenizer.
    fn matches(&self, state: State, ch: char) -> bool
    {
        ret_if!(state != self.from, false);

        // TODO: Row state matches and
        // TODO: Row accept allows OR is unused and
        // TODO: Row except lacks OR is unused

        // panic!("The Within&AnyOf must match ALL combinations");

        // let enable_accept = self.accept != ('\0' ..= '\0');
        // let allow = self.accept.contains(ch) && enable_accept;

        // let enable_except = self.except != ('\0' ..= '\0');
        // let deny = self.except.contains(ch) && enable_except;

        // allow && !deny

        self.accept.contains(ch) && !self.except.contains(ch)
    }
}

pub fn tokenize(source: &str) -> PqResult<()>
{
    let transitions: [Row; _] = state_transition_table();
    let mut curr = BEG;
    let mut buf = String::with_capacity(32);
    let mut toks: Vec<Tok> = Vec::with_capacity(source.len());

    let w_state = longest_variant_name::<State>();
    let w_tok_act = longest_variant_name::<Act>() + 2;
    let w_ch = format!("{:?}", '\u{10FFFF}').len();
    let w_buf = 8;

    let header = format!(
        "{:<w_state$} {:<w_ch$} {:<w_state$} {:<w_tok_act$} {:<16} {:<16} {:<w_buf$} {:<w_buf$}",
        "State", "Char", "Next", "Act", "Accept", "Except", "PreBuf", "EndBuf"
    );
    println!("\n\n\n{}", header.underlined());

    let mut dbg_used_transitions: HashSet<Row> =
        HashSet::with_capacity(max_state_transitions());
    let dbg_expect_transitions: HashSet<Row> =
        HashSet::from_iter(state_transition_table());

    for ch in source.chars().chain("\0".chars())
    {
        // ? Find the transition for the current state and character
        let row = match transitions.iter().find(|row| row.matches(curr, ch))
        {
            Some(row) => *row,
            None => return Err(LexErr::InvalidStateTransition(curr, ch).into()),
        };

        if !(row.from == row.onto && row.act == IGN)
        {
            println!(
                "{from:<w_state$} {char:<w_ch$} {next:<w_state$} {act:<w_tok_act$} {acc:<16} {exc:<16} {prebuf:<w_buf$} {postbuf:w_buf$}",
                from = format!("{:?}", curr),
                char = format!("{:?}", ch),
                next = format!("{:?}", row.onto),
                act = format!("{:?}", row.act),
                acc = format!("{:?}", row.accept),
                exc = format!("{:?}", row.except),
                prebuf = format!("{:?}", buf),
                postbuf = format!("{:?}   ", match row.act
                {
                    FIN => ch.to_string(),
                    TOK | ATK => String::new(),
                    ACC => format!("{buf}{ch}"),
                    IGN => buf.clone(),
                })
            );
        }

        match row.act
        {
            // Token can be created using the currect character and the buffer
            Act::ATK =>
            {
                buf.push(ch);
                toks.push(row.from.finalize(&buf)?);
                buf.clear();
            }
            // Token can be created from the buffer, do not accumulate character
            Act::TOK =>
            {
                toks.push(row.from.finalize(&buf)?);
                buf.clear();
            }
            // If no token can be constructed, continue accumulating the buffer
            Act::ACC =>
            {
                buf.push(ch);
            }
            // Token an be created while preserving the current character
            Act::FIN =>
            {
                toks.push(row.from.finalize(&buf)?);
                buf.clear();
                buf.push(ch);
            }
            // Discard the current character and leave the buffer intact
            Act::IGN => (),
        }

        dbg_used_transitions.insert(row);

        if row.onto == State::end_state()
        {
            println!("Hit explicit {} state", "END".underlined());
            break;
        }

        curr = row.onto;
    }

    println!();
    println!("{}", format!("Tokens: {toks:?}").green());
    println!();

    let dbg_msg = format!(
        "Only hit {} out of {} state transitions, missed:",
        dbg_used_transitions.len(),
        dbg_expect_transitions.len(),
    );

    let style = if dbg_used_transitions.len() < dbg_expect_transitions.len()
    {
        <String as Stylize>::yellow
    }
    else
    {
        <String as Stylize>::reset
    };

    println!("{}", style(dbg_msg));

    if dbg_used_transitions.len() < dbg_expect_transitions.len()
    {
        for unused in
            dbg_expect_transitions.difference(&dbg_used_transitions).take(4)
        {
            println!("{}", style(unused.to_debug()))
        }
        println!("{}", style("...".to_string()));
        println!("{}", style("...".to_string()));
    }

    Ok(())
}

#[derive(VariantNames, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
#[repr(u8)]
pub enum Act
{
    /// Consume buffer as token, accumulate current character into empty buffer
    FIN,
    /// Consume buffer as token, ignore current character
    TOK,
    /// Accumulate current character, consume buffer as token
    ATK,
    /// Accumulate current character, append to buffer
    ACC,
    /// Ignore current character, leave buffer untouched
    IGN,
    // /// Collect current character into buffer *first*, then Finalize buffer as token *second*
    // COL,

    // /// Finalize buffer as token *first*, then accumulate current character.
    // FIN,

    // /// Finalize buffer as token, then ignore current character
    // TOK, // ? Can this be removed if prev state is tracked?

    // /// Accumulate current character. Append to buffer.
    // ACC,

    // /// Ignore the current character. Do not append to buffer.
    // IGN,
}

impl State
{
    /// Callback to be used by the tokenizer machinery as a sentinel on when to stop
    /// lexing, even with a non-empty buffer.
    const fn end_state() -> Self
    {
        Self::END
    }

    fn finalize(self, buffer: &String) -> PqResult<Tok>
    {
        match self
        {
            Self::BEG if buffer == "," => Ok(Tok::Comma),
            Self::BEG => unreachable!("BEG tokenizer state unutilized"),
            Self::NUM => buffer.parse().map_err(Into::into).map(Tok::Signed),
            Self::COM => Ok(Tok::Comma),
            Self::BIT0S => Ok(Tok::False),
            Self::BIT0F => todo!(),
            Self::BIT0A => todo!(),
            Self::BIT0L => todo!(),
            Self::BIT1E => todo!(),
            Self::ComOrClose => todo!(),
            Self::BIT1T => todo!(),
            Self::BIT1R => todo!(),
            Self::BIT1U => Ok(Tok::True),
            Self::TXT => Ok(Tok::Text(buffer.clone())),
            Self::ESC => Ok(Tok::Escape(buffer.clone())),
            Self::ESCHEX3 => Ok(Tok::EscapeHex(buffer.clone())),
            Self::END => todo!(),
            Self::ESCHEX0 | Self::ESCHEX1 | Self::ESCHEX2 => unreachable!(),
        }
    }
}

#[derive(VariantNames, Debug, Clone, PartialEq)]
pub enum Tok
{
    True,              // true
    False,             // false
    Null,              // null
    Text(String),      // "a"
    Escape(String),    // "\n"
    EscapeHex(String), // "\uAb34"
    Comma,             // ,
    //
    Colon,        // :
    Dot,          // .
    SquareOpen,   // [
    SquareClose,  // ]
    BlockOpen,    // {
    BlockClose,   // }
    Signed(i32),  // +0 -0
    Decimal(f64), // +0.0 -0.0
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
#[derive(
    VariantNames, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum CharMatch
{
    /// **Explicitly listed elements**
    AnyOf(&'static str),
    /// **Elements explicitly within this range. Equivalent to `char ..= char`**
    Within(char, char),
    /// **Ignored accept/except bound**
    Unused,
}

mod impl_char_match
{
    use super::*;

    impl CharMatch
    {
        pub fn contains(&self, ch: char) -> bool
        {
            match self
            {
                AnyOf(these_chars) => these_chars.contains(ch),
                Within(from, onto) => (from ..= onto).contains(&&ch),
                Unused => false,
            }
        }
    }

    // /// Compare [CharRangeInclusive] == [RangeInclusive<char>]
    // impl PartialEq<RangeInclusive<char>> for CharRangeInclusive
    // {
    //     fn eq(&self, other: &RangeInclusive<char>) -> bool
    //     {
    //         self.from == *other.start() && self.onto == *other.end()
    //     }
    // }

    // /// Compare [RangeInclusive<char>] == [CharRangeInclusive]
    // impl PartialEq<CharRangeInclusive> for RangeInclusive<char>
    // {
    //     fn eq(&self, other: &CharRangeInclusive) -> bool
    //     {
    //         *self.start() == other.from && *self.end() == other.onto
    //     }
    // }

    // Compare [CharMatch] == [CharMatch]
    // impl PartialEq for CharMatch
    // {
    //     fn eq(&self, other: &Self) -> bool
    //     {
    //         match self
    //         {
    //             AnyOf(_) => todo!(),
    //             Within(..) => todo!(),
    //             Unused => todo!(),
    //         }

    //         self.from == other.from && self.onto == other.onto
    //     }
    // }

    // impl hash::Hash for CharMatch
    // {
    //     fn hash<H: hash::Hasher>(&self, state: &mut H)
    //     {
    //         self.from.hash(state);
    //         self.onto.hash(state);
    //     }
    // }
}

const EOS: CharMatch = AnyOf("\0");
const WHITESPACE: CharMatch = AnyOf("\n \t\r");

const fn ignore_spaces_after(
    status: State,
) -> (State, CharMatch, CharMatch, State, Act)
{
    (status, WHITESPACE, Unused, status, IGN)
}

/// Some of these states produce tokens when finalized.
#[derive(
    VariantNames, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[repr(u8)]
pub enum State
{
    BEG,
    NUM,
    COM,
    ComOrClose,
    BIT0F,
    BIT0A,
    BIT0L,
    BIT0S,
    BIT1T,
    BIT1R,
    BIT1U,
    BIT1E,
    TXT,
    ESC,
    ESCHEX0,
    ESCHEX1,
    ESCHEX2,
    ESCHEX3,
    END,
    // -------------------------------------------------------------------------
    // OBJ0,
    // ARR0,
    // TXT0,
    // NIL,
    // OP0,
    // OP1,
    // OP2,
    // SNG,
    // SYM,
    // DEC0,
    // DEC1,
}

/// **State transitions are locked to character iteration. Essentially, check
/// that char is in this range or set and also not in this range or set.**
/// `[curr state][accept ch range][except ch range][next state][tok & buf act]`
const STATE_TRANSITION_TABLE: &[(State, CharMatch, CharMatch, State, Act)] = &[
    ignore_spaces_after(COM),
    ignore_spaces_after(ComOrClose),
    (BEG, Within('0', '9'), Unused, NUM, ACC),
    (NUM, Within('0', '9'), Unused, NUM, ACC),
    (NUM, WHITESPACE, Unused, ComOrClose, IGN),
    (ComOrClose, AnyOf(","), Unused, COM, IGN),
    (ComOrClose, AnyOf("]"), Unused, END, IGN),
    (ComOrClose, AnyOf("}"), Unused, END, IGN),
    (COM, Within('0', '9'), Unused, NUM, FIN),
    (NUM, AnyOf(","), Unused, COM, TOK),
    // Beginning ---------------------------------------------------------------
    (BEG, EOS, Unused, END, IGN),
    ignore_spaces_after(BEG),
    // Boolean -----------------------------------------------------------------
    (BEG, AnyOf("f"), Unused, BIT0F, IGN),
    (BIT0F, AnyOf("a"), Unused, BIT0A, IGN),
    (BIT0A, AnyOf("l"), Unused, BIT0L, IGN),
    (BIT0L, AnyOf("s"), Unused, BIT0S, IGN),
    (BIT0S, AnyOf("e"), Unused, BEG, TOK),
    (BEG, AnyOf("t"), Unused, BIT1T, IGN),
    (BIT1T, AnyOf("r"), Unused, BIT1R, IGN),
    (BIT1R, AnyOf("u"), Unused, BIT1U, IGN),
    (BIT1U, AnyOf("e"), Unused, BEG, TOK),
    // Comma -------------------------------------------------------------------
    (BEG, AnyOf(","), Unused, COM, FIN),
    (COM, AnyOf("f"), Unused, BIT0F, IGN),
    (COM, AnyOf("t"), Unused, BIT1T, IGN),
    (COM, Within('0', '9'), Unused, NUM, ACC),
    // Key or Value ------------------------------------------------------------
    (BEG, AnyOf("\""), Unused, TXT, IGN),
    (TXT, AnyOf("\""), Unused, BEG, TOK),
    (TXT, Within('\u{0020}', '\u{10FFFF}'), AnyOf("\\\""), TXT, ACC),
    (TXT, AnyOf("\\"), Unused, ESC, FIN),
    (ESC, AnyOf("\"\\/bfnrt"), Unused, TXT, ATK),
    (ESC, AnyOf("u"), Unused, ESCHEX0, ACC),
    // Hex Escape --------------- 0-9A-F → U+0030 ..= U+0046 - U+003A ..= U+0040
    (ESCHEX0, Within('0', 'F'), Within(':', '@'), ESCHEX1, ACC),
    (ESCHEX0, Within('a', 'f'), Unused, ESCHEX1, ACC),
    (ESCHEX1, Within('0', 'F'), Within(':', '@'), ESCHEX2, ACC),
    (ESCHEX1, Within('a', 'f'), Unused, ESCHEX2, ACC),
    (ESCHEX2, Within('0', 'F'), Within(':', '@'), ESCHEX3, ACC),
    (ESCHEX2, Within('a', 'f'), Unused, ESCHEX3, ACC),
    (ESCHEX3, Within('0', 'F'), Within(':', '@'), TXT, ATK),
    (ESCHEX3, Within('a', 'f'), Unused, TXT, ATK),
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

        match (accept, except)
        {
            // Range * len(chars) = len(chars) rows
            (AnyOf(chars), Unused)
            | (Within(..), AnyOf(chars))
            | (Unused, AnyOf(chars)) => transitions += chars.len(),

            // CharRangeInclusives count as one row
            (Within(..), Within(..))
            | (Within(..), Unused)
            | (Unused, Within(..)) => transitions += 1,

            _ => panic!("this is an invalid state transition combo"),
        }
    }
    transitions
}

const fn state_transition_table() -> [Row; STATE_TRANSITION_TABLE.len()]
{
    const EXPANDED_TABLE_LEN: usize = max_state_transitions();
    // let mut rows: [Row; EXPANDED_TABLE_LEN] = [Row::zero(); EXPANDED_TABLE_LEN];
    let mut rows: [Row; STATE_TRANSITION_TABLE.len()] =
        [Row::zero(); STATE_TRANSITION_TABLE.len()];

    let mut row_udx = 0usize;
    while row_udx < rows.len()
    {
        let (from, accept, except, onto, act) = STATE_TRANSITION_TABLE[row_udx];
        rows[row_udx] = Row { from, accept, except, onto, act };
        row_udx += 1;
    }

    // Slow index is input table, fast index is output table because it adds
    // more rows than the input table. Destination index doesn't matter since
    // lookup will be O(N) anyway.
    let (mut slow, mut fast) = (0, 0);

    #[cfg(false)]
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

                    let empty = '\0' ..= '\0';
                    let row = Row::new(from, ch ..= ch, empty, onto, act);

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
                }
            }

            (Within(begin, close), AnyOf(chars)) =>
            {
                // TODO: Split the accept range such that there is one copy that
                // TODO: excludes a ch for each ch in AnyOf.

                // panic!("i dont think this is working: unused range skips ..");

                // -----------------

                // -----------------

                let mut iter = txt::utf8_iter_chars_const(chars);
                while let Some(ch) = iter.next()
                {
                    // let accept = CharRangeInclusive::from(begin ..= close);
                    // let except = CharRangeInclusive::from(ch ..= ch);
                    // debug_assert!(
                    //     !accept.contains(ch) || except.contains(ch),
                    //     "Invalid state transition: {:?} {:?}",
                    //     range,
                    //     range,
                    // );

                    let row =
                        Row::new(from, begin ..= close, ch ..= ch, onto, act);

                    // * 100% chance of success: continuously split accept by ch

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
                }

                /*
                // If it's any of these characters, add a new 'except' range for
                // each one since they are single element not a range
                let mut udx_ch = 0;
                while let Some(ch) = utf8_char_on(chars.as_bytes(), udx_ch)
                    && udx_ch < chars.len()
                {
                    udx_ch += ch.len_utf8();

                    let row =
                        Row::new(from, begin ..= close, ch ..= ch, onto, act);

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
                }
                */
            }

            (Unused, AnyOf(chars)) =>
            {
                // If it's any of these characters, add a new 'accept' range for
                // each one since they are single element not a range
                let mut udx_ch = 0;
                while let Some(ch) = utf8_char_on(chars.as_bytes(), udx_ch)
                    && udx_ch < chars.len()
                {
                    udx_ch += ch.len_utf8();

                    let empty = '\0' ..= '\0';
                    let row = Row::new(from, empty, ch ..= ch, onto, act);

                    rows[fast] = row;
                    fast += 1; // Outpace input table index
                }
            }

            (Within(from_in, upto_in), Within(from_ou, upto_ou)) =>
            {
                let row = Row::new(
                    from,
                    from_in ..= upto_in,
                    from_ou ..= upto_ou,
                    onto,
                    act,
                );
                rows[fast] = row;
                fast += 1;
            }

            (Within(from_in, upto_in), Unused) =>
            {
                let empty = '\0' ..= '\0';
                let row = Row::new(from, from_in ..= upto_in, empty, onto, act);
                rows[fast] = row;
                fast += 1;
            }

            (Unused, Within(from_in, upto_in)) =>
            {
                let empty = '\0' ..= '\0';
                let row = Row::new(from, empty, from_in ..= upto_in, onto, act);
                rows[fast] = row;
                fast += 1;
            }

            _ => panic!("this is an invalid state transition combo"),
        }
    }

    // assert!(fast == rows.len(), "Sanity check: were offsets correct?");

    rows
}

fn emit_table(table: &[Row])
{
    let state = longest_variant_name::<State>();
    let tok_act = longest_variant_name::<Act>();
    let accept = {
        table
            .iter()
            .map(|s| {
                // Within('\u{10FFFF}', '\u{10FFFF}');
                let len_acc = format!("{:?}", s.accept).len();
                let len_exc = format!("{:?}", s.except).len();
                len_acc.max(len_exc)
            })
            .max()
            .unwrap_or_default()
    };
    println!(
        "| {:<state$} | {:^accept$} | {:^accept$} | {:<state$} | {:<tok_act$} |",
        "From", "Accept", "Except", "Onto", "Action",
    );

    for row in table
    {
        println!(
            "| {fro:<state$} | {acc:^accept$} | {exc:^accept$} | {to:<state$} | {act:<tok_act$} |",
            fro = format!("{:?}", row.from),
            acc = format!("{:?}", row.accept),
            exc = format!("{:?}", row.except),
            to = format!("{:?}", row.onto),
            act = format!("{:?}", row.act),
        );
    }
    println!();
}

fn main() -> PqResult<()>
{
    emit_table(&state_transition_table()[..]);

    let json = r#"
        "init"
        818
        "\n"
        true, false
        null
        0.22
        []
        {}
    "#;

    for line in json.lines().take(2).filter(|line| !line.trim().is_empty())
    {
        if let Err(err) = tokenize(line)
        {
            println!("{}", format!("{err}").red());
        }
    }

    Ok(())
}

// mod iter
// {
//     pub struct SeqIter<'col, T>
//     {
//         at: usize,
//         of: &'col [T],
//     }

//     impl<'col, T> SeqIter<'col, T>
//     {
//         pub const fn next(&mut self) -> Option<&'col T>
//         {
//             if self.at < self.of.len()
//             {
//                 let item: T = self.of[self.at];
//                 self.at += 1;
//                 return Some(&item);
//             }
//             None
//         }
//     }
// }
