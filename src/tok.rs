//! State transition table for the streaming JSON tokenizer

use thiserror::Error;

use crate::PqResult;
use {Act::*, CharMatch::*, State::*};

/*
https://www.crockford.com/mckeeman.html

json
    element

value
    object
    array
    string
    number
    "true"
    "false"
    "null"

object
    '{' ws '}'
    '{' members '}'

members
    member
    member ',' members

member
    ws string ws ':' element

array
    '[' ws ']'
    '[' elements ']'

elements
    element
    element ',' elements

element
    ws value ws

string
    '"' characters '"'

characters
    ""
    character characters

character
    '0020' . '10FFFF' - '"' - '\'
    '\' escape

escape
    '"'
    '\'
    '/'
    'b'
    'f'
    'n'
    'r'
    't'
    'u' hex hex hex hex

hex
    digit
    'A' . 'F'
    'a' . 'f'

number
    integer fraction exponent

integer
    digit
    onenine digits
    '-' digit
    '-' onenine digits

digits
    digit
    digit digits

digit
    '0'
    onenine

onenine
    '1' . '9'

fraction
    ""
    '.' digits

exponent
    ""
    'E' '0'  // added by me
    'e' '0'  // added by me
    'E' sign digits
    'e' sign digits

sign
    ""
    '+'
    '-'

ws
    ""
    '0020' ws
    '000A' ws
    '000D' ws
    '0009' ws
*/

// TODO: Make it so that the tokenization machinery is generic:
// TODO: Bring-Your-Own TokErr enum
// TODO: Bring-Your-Own const State Transition Table
// TODO: Bring-Your-Own States enum
// TODO: Bring-Your-Own State Buffer-To-Token Converter

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

/// Some of these states produce tokens when finalized.
#[derive(
    VariantNames, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[repr(u8)]
pub enum State
{
    BEG,
    SGN,
    ZERO,
    INT,
    FRAC,
    ESGN,
    EXP,
    COM,
    BIT0F,
    BIT0A,
    BIT0L,
    BIT0S,
    BIT1T,
    BIT1R,
    BIT1U,
    NIL0,
    NIL1,
    NIL2,
    TXT,
    ESC,
    ESCHEX0,
    ESCHEX1,
    ESCHEX2,
    ESCHEX3,
    END,
}

// TODO: Could this benefit from a bit of:
// TODO: [TOKEN ACT][BUFFER ACT] since they seem to be combos of that?
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
#[repr(u8)]
pub enum Act
{
    /// Consume buffer as token, and replay current character for the next state
    AGN,
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
    Number(f64),       // +0.0 0.0 0 10
    ArrayOpen,         // [
    ArrayClose,        // ]
    ObjectOpen,        // {
    ObjectClose,       // }
    Comma,             // ,
    Colon,             // :
}

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

const EOS: CharMatch = AnyOf("\0");

/// **State transitions are locked to character iteration. Essentially, check
/// that char is in this range or set and also not in this range or set.**
/// `[curr state][accept ch range][except ch range][next state][tok & buf act]`
const STATE_TRANSITION_TABLE: &[(State, CharMatch, CharMatch, State, Act)] = &[
    // Beginning ---------------------------------------------------------------
    (BEG, EOS, Unused, END, IGN),
    (BEG, AnyOf("\n \t\r"), Unused, BEG, IGN),
    // Numbers -----------------------------------------------------------------
    // Zero
    (BEG, AnyOf("0"), Unused, ZERO, ACC),
    (ZERO, AnyOf("."), Unused, FRAC, ACC),
    (ZERO, AnyOf("eE"), Unused, ESGN, ACC),
    (ZERO, EOS, Unused, BEG, TOK),
    // Sign
    (BEG, AnyOf("-"), Unused, SGN, ACC),
    (SGN, AnyOf("0"), Unused, ZERO, ACC),
    (SGN, Within('1', '9'), Unused, INT, ACC),
    // Integer
    (BEG, Within('1', '9'), Unused, INT, ACC),
    (INT, Within('0', '9'), Unused, INT, ACC),
    (INT, AnyOf("."), Unused, FRAC, ACC),
    (INT, AnyOf("eE"), Unused, ESGN, ACC),
    (INT, EOS, Unused, BEG, TOK),
    // Fraction
    (FRAC, Within('0', '9'), Unused, FRAC, ACC),
    (FRAC, AnyOf("eE"), Unused, ESGN, ACC),
    (FRAC, EOS, Unused, BEG, TOK),
    // Exponent
    (ESGN, AnyOf("+-"), Unused, EXP, ACC),
    (ESGN, Within('0', '9'), Unused, EXP, ACC),
    (EXP, Within('0', '9'), Unused, EXP, ACC),
    (EXP, EOS, Unused, BEG, TOK),
    //
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
    // Null --------------------------------------------------------------------
    (BEG, AnyOf("n"), Unused, NIL0, IGN),
    (NIL0, AnyOf("u"), Unused, NIL1, IGN),
    (NIL1, AnyOf("l"), Unused, NIL2, IGN),
    (NIL2, AnyOf("l"), Unused, BEG, TOK),
    // Comma -------------------------------------------------------------------
    (BEG, AnyOf(","), Unused, BEG, ATK),
    (ZERO, AnyOf(","), Unused, BEG, AGN),
    (INT, AnyOf(","), Unused, BEG, AGN),
    (FRAC, AnyOf(","), Unused, BEG, AGN),
    (EXP, AnyOf(","), Unused, BEG, AGN),
    // Key or Value ------------------------------------------------------------
    (BEG, AnyOf("\""), Unused, TXT, IGN),
    (TXT, AnyOf("\""), Unused, BEG, TOK),
    (TXT, Within('\u{0020}', '\u{10FFFF}'), AnyOf("\\\""), TXT, ACC),
    (TXT, AnyOf("\\"), Unused, ESC, FIN),
    (ESC, AnyOf("\"\\/bfnrt"), Unused, TXT, ATK),
    (ESC, AnyOf("u"), Unused, ESCHEX0, ACC),
    // Hex Escape --------------- 0-9A-F → U+0030 ..= U+0046 - U+003A ..= U+0040
    (ESCHEX0, Within('0', 'F'), Within(':', '@'), ESCHEX1, ACC),
    (ESCHEX1, Within('0', 'F'), Within(':', '@'), ESCHEX2, ACC),
    (ESCHEX2, Within('0', 'F'), Within(':', '@'), ESCHEX3, ACC),
    (ESCHEX3, Within('0', 'F'), Within(':', '@'), TXT, ATK),
    // Array -------------------------------------------------------------------
    (BEG, AnyOf("["), Unused, BEG, ATK),
    (ZERO, AnyOf("]"), Unused, BEG, AGN),
    (INT, AnyOf("]"), Unused, BEG, AGN),
    (FRAC, AnyOf("]"), Unused, BEG, AGN),
    (EXP, AnyOf("]"), Unused, BEG, AGN),
    (BEG, AnyOf("]"), Unused, BEG, ATK),
    // Object ------------------------------------------------------------------
    (BEG, AnyOf("{"), Unused, BEG, ATK),
    (BEG, AnyOf(":"), Unused, BEG, ATK),
    (ZERO, AnyOf("}"), Unused, BEG, AGN),
    (INT, AnyOf("}"), Unused, BEG, AGN),
    (FRAC, AnyOf("}"), Unused, BEG, AGN),
    (EXP, AnyOf("}"), Unused, BEG, AGN),
    (BEG, AnyOf("}"), Unused, BEG, ATK),
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

pub fn tokenize(source: &str, usage_report: &mut UsageReport) -> PqResult<()>
{
    usage_report.new_document();

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
    println!("\n\n\n{}", header.cyan().underlined());

    let mut stream = source.chars().chain("\0".chars()).replayable();
    while let Some(ch) = stream.next()
    // for ch in source.chars().chain("\0".chars())
    {
        // ? Find the transition for the current state and character
        let row = match transitions.iter().find(|row| row.matches(curr, ch))
        {
            Some(row) => *row,
            None => return Err(LexErr::InvalidStateTransition(curr, ch).into()),
        };

        // Debug-print row info
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
                    AGN => String::new(),
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
            // Token can be created from buffer, replay the current character
            Act::AGN =>
            {
                toks.push(row.from.finalize(&buf)?);
                buf.clear();
                stream.replay();
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

        usage_report.log_row(row);

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

    Ok(())
}

const fn state_transition_table() -> [Row; STATE_TRANSITION_TABLE.len()]
{
    #[cfg(false)]
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
    #[cfg(false)]
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
        use self::*;
        let non_terminal = || panic!("Token non-terminal encountered");

        match self
        {
            BEG if buffer == "," => Ok(Tok::Comma),
            BEG if buffer == ":" => Ok(Tok::Colon),
            BEG if buffer == "[" => Ok(Tok::ArrayOpen),
            BEG if buffer == "]" => Ok(Tok::ArrayClose),
            BEG if buffer == "{" => Ok(Tok::ObjectOpen),
            BEG if buffer == "}" => Ok(Tok::ObjectClose),
            BEG => panic!("BEG tokenizer state unutilized"),
            END => todo!(),
            COM => Ok(Tok::Comma),
            BIT1T | BIT1R => non_terminal(),
            BIT1U => Ok(Tok::True),
            TXT => Ok(Tok::Text(buffer.clone())),
            BIT0F | BIT0A | BIT0L => non_terminal(),
            BIT0S => Ok(Tok::False),
            NIL0 | NIL1 => non_terminal(),
            NIL2 => Ok(Tok::Null),
            ESC => Ok(Tok::Escape(buffer.clone())),
            ESCHEX0 | ESCHEX1 | ESCHEX2 => non_terminal(),
            ESCHEX3 => Ok(Tok::EscapeHex(buffer.clone())),
            SGN => buffer.parse().map_err(Into::into).map(Tok::Number),
            ZERO => buffer.parse().map_err(Into::into).map(Tok::Number),
            INT => buffer.parse().map_err(Into::into).map(Tok::Number),
            FRAC => buffer.parse().map_err(Into::into).map(Tok::Number),
            ESGN => non_terminal(),
            EXP => buffer.parse().map_err(Into::into).map(Tok::Number),
        }
    }
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

/// Exists because [RangeInclusive<char>] is not [Copy].
#[derive(Clone, Copy, PartialOrd, Eq)]
pub struct CharRangeInclusive
{
    from: char,
    onto: char,
}

mod impl_char_range_inclusive
{
    use std::collections::HashMap;

    use crate::ret_if;

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
