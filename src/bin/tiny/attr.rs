//! Row attributes to make editing, querying, and formatting easier.

use crate::parser::Row;
use smallvec::{SmallVec, smallvec};
use strum::VariantNames;

// TODO: Bitflags instead? Less convenient for formatting routines?
#[derive(VariantNames)]
#[repr(i8)]
pub enum Attr
{
    NotEmpty = -1, // ! Could be more expressive like this
    Empty = 1,
    Array,
    Last,
    Lone,
    EndArray,
    EndObject,
    EndParent,
    Sibling,
    Key,
    Parent,
    Object,
    Value,
}

// end par: (query par to get arr or obj ty)
// where is last *exactly* calculated?
// maybe: sibling = comma
// maybe: key = colon
// maybe: last = no comma
// maybe: lone = very low cost to wrap (dynamic programming?)
// quora: when is this *not* a sibling? lone?

pub fn attributes(row: Row, table: &Vec<Row>) -> SmallVec<[Attr; 8]>
{
    // smallvec![Attr::Sibling, Attr::Value];
    let mut attrs: SmallVec<[Attr; 8]> = SmallVec::new();
    attrs.push(Attr::Key);
    attrs.push(Attr::Sibling);

    match row.ty
    {
        crate::parser::RowType::Obj => attrs.push(Attr::Object),
        crate::parser::RowType::Arr => attrs.push(Attr::Array),
        crate::parser::RowType::Str => attrs.push(Attr::Value),
        crate::parser::RowType::Num => attrs.push(Attr::Value),
        crate::parser::RowType::Bit => attrs.push(Attr::Value),
        crate::parser::RowType::Nil => attrs.push(Attr::Value),
    }

    attrs
}
