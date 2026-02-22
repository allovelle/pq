use crate::tok::{Act, Tok};
use strum::*;
use thiserror::Error;

/*
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
    Comma,
    Colon,
}
*/

#[derive(Debug, Error)]
#[error("Parse Error")]
pub enum ParseErr
{
    // While building
    #[error("unexpected token `{1:?}` while building `{2:?}`: expected {0:?}")]
    Expectation(Tok, Tok, AstRowType),
}

pub type ParseResult<T> = Result<T, ParseErr>;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum AstRowType { Arr = 1, Obj, Nil, Bit, Txt, Num, }

/// A row in the output tree of objects, arrays, keys, and values.
#[derive(Debug, Clone)]
pub struct AstRow
{
    id: u32,
    parent: u32,
    key: String,
    value: String,
    ty: AstRowType,
}

pub fn accept_value(token: &Tok) -> bool
{
    match token
    {
        Tok::True
        | Tok::False
        | Tok::Null
        | Tok::Text(_)
        | Tok::Number(_)
        | Tok::ArrayOpen
        | Tok::ObjectOpen => true,

        Tok::Escape(_)
        | Tok::EscapeHex(_)
        | Tok::ArrayClose
        | Tok::ObjectClose
        | Tok::Comma
        | Tok::Colon => false,
    }
}

pub fn accept_open_obj(token: &Tok) -> bool
{
    matches!(token, Tok::ObjectOpen)
}

pub fn expect_open_obj(token: &Tok) -> ParseResult<AstRow>
{
    let ty = AstRowType::Arr;
    match token
    {
        Tok::ObjectOpen => Ok(AstRow {
            id: todo!(),
            parent: todo!(),
            key: todo!(),
            value: todo!(),
            ty,
        }),

        // TODO: Tok(index) & ::val(), TokVal so that Tok is clone if source offset exists
        _ => Err(ParseErr::Expectation(Tok::ArrayOpen, token.clone(), ty)),
    }
}

pub fn expect_value(udx: usize, tokens: &[Tok]) -> ParseResult<(usize, ())>
{
    Ok((0, ()))
}

/*
BEG, TRU, -> END_ARR
BEG, TRU, -> END_OBJ
BEG, TRU, -> END_COM
BEG, FAL, -> END_ARR
BEG, FAL, -> END_OBJ
BEG, FAL, -> END_COM

BEG, NEW_OBJ, -> END_OBJ
BEG, NEW_OBJ, -> KEY
KEY, END_OBJ -> FIN??
*/

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
enum TokTy
{
    TRU = 1, FAL, NUL, TXT, ESC, HEX, NUM, NEW_ARR, END_ARR, NEW_OBJ, END_OBJ,
    COM, COL,
}

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
pub enum State
{
    BEG = 1, END, OBJ, ARR, TXT,
}

#[rustfmt::skip]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(VariantNames, Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
#[repr(u8)]
pub enum Action
{
    FIN = 1,
    KEY,
}

// Expect/Accept
// Tab/Nest record parent ID, clip/take astnode id
// [ID][PARENT][KEY][VALUE][TYPE]
struct Row(usize, usize, String, String, TokTy);
struct Transition(State, TokTy, State, Action);

fn parser_state_transition_table() -> Vec<Transition>
{
    use self::{Action::*, State::*, TokTy::*};

    vec![
        Transition(BEG, TRU, END, FIN),
        Transition(BEG, TRU, END, FIN),
        Transition(BEG, TRU, END, FIN),
        Transition(BEG, TRU, END, FIN),
        Transition(BEG, TRU, END, FIN),
    ]
}

fn iter()
{
    use self::{Action::*, State::*, TokTy::*};

    let transitions = parser_state_transition_table();
    let mut rows: Vec<Row> = vec![];
    let mut curr: State = BEG;
    let mut key = String::default();
    // let mut id_stack = vec![];

    // TODO: The token buffer should be 1 and should end with value
    // TODO: The parent ID tracking should be just inc id each row

    // !!!!!!! ALL THE TOKEN TYPES SHOULD BE STRINGS STILL, THEY ARE CONVERTED
    // !!!!!!! TO STRS HERE. THE VALUE SHOULD STILL BE A STRING

    // TODO: [id 0][parent 0][key ''][val '{'][ty obj]
    // TODO: [id 1][parent 0][key 'name'][val 'Alo'][ty obj]

    let test_tokens = vec![Tok::True];

    for token in test_tokens
    {
        let tok_ty = TokTy::from(token);

        // let id = id_stack.last().unwrap_or_default();

        for Transition(from, ty, to, act) in transitions.iter()
        {
            if curr == *from && tok_ty == *ty
            {
                curr = *to;
                // match act
                // {
                //     FIN => rows.push(Row(0, 0, key, val, *ty)),
                // }
            }
        }
    }
}

/* pub fn build_ast_table(tokens: &[Tok]) -> ParseResult<Vec<AstRow>>
{
    let mut state = State::BEG;
    let mut id = 0;
    let mut udx = 0;

    while let Some(token) = tokens.get(udx)
    {
        match state
        {
            State::BEG if accept_value(token) =>
            {
                let (consumed, row) = expect_value(udx, tokens)?;
            }
            _ => panic!("unhandled syntax stage"),
        }

        // if state == State::BEG && accept_open_obj(token)
        // {
        //     let row = expect_open_obj(token).unwrap();
        // }

        udx += 1;
    }

    Ok(vec![])
}
 */
mod convert
{
    use super::*;
    use crate::tok::Tok;

    impl From<Tok> for TokTy
    {
        fn from(value: Tok) -> Self
        {
            use TokTy::*;
            match value
            {
                Tok::True => TRU,
                Tok::False => FAL,
                Tok::Null => NUL,
                Tok::Text(_) => TXT,
                Tok::Escape(_) => ESC,
                Tok::EscapeHex(_) => HEX,
                Tok::Number(_) => NUM,
                Tok::ArrayOpen => NEW_ARR,
                Tok::ArrayClose => END_ARR,
                Tok::ObjectOpen => NEW_OBJ,
                Tok::ObjectClose => END_OBJ,
                Tok::Comma => COM,
                Tok::Colon => COL,
            }
        }
    }
}
