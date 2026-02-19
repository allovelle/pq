use super::parser::*;

#[derive(Debug)]
pub enum Query
{
    /// Select key from object.
    /// Selects a key from an object like `key1.key2` where obj[key1][key2] is
    /// the result. This syntax is supported in other context such as building
    /// new objects. `{key1, "new": key2, key3: key3.key4}`
    /// "Pick Key"
    SelectKey(String),
    SelectIndex,

    /// "Drop Key""
    FilterKey(String),
    FilterIndex,

    /// Every array index and element range is a lookup into the JSON value,
    /// and no object keys are supported since arrays do not have keys.
    /// [0 .. 4][0, 1, 2:4] results in: [0, 1, 2, 3]. Works with [::] also.
    NewArr,

    /// `{a, b}`` results in `value["a"], value["b"] = {"a":1, "b":2}` if
    /// value is `{"a":1, "b":2, "c":3}`.
    NewObj,

    /// Array elements each become unique destinations for command output.
    /// For `[1, 2, 3, 4]`, fanout means commands result in a list of
    /// separate JSON value results, one for each element. To join them back
    /// into a single array use the [Self::Join] command. Often becomes Json
    /// Lines output if not joined. Arrays become a list of JSON values from
    /// the elements of the array. If piped directly to a file, the
    /// [Self::Fanout] command creates a JSON Lines document from a normal
    /// JSON array. Json objects become a list of JSON value streams where
    /// each key-value in the object becomes a standalone `{key: value}`.
    Fanout,

    /// Takes multiple JSON value streams (such as rows from a .jsonl file),
    /// and combines them into an array. The [Self::Fanout] command does the
    /// inverse and turns a normal array into the equivalent of JSON Lines.
    /// Joining does nothing if the input is already a JSON value stream.
    /// Joining is designed for combining multiple JSON value streams such
    /// as from input from a Json Lines document.
    Join,

    /// Results in an array of keys for the current JSON value. For an array
    /// this returns the indices as integers rather than strings.
    Keys,

    /// Results in an array of values of either array elements or object
    /// values from keys. For key-values becoming own JSON objects, use the
    /// [Self::Fanout] command.
    Values,

    _Spread,
    _Fanout,
    _Join,
    _Select,

    /// Can filter, select, build, and anything else in the script runtime.
    /// `(i for i in range(10) if i % 2 == 0)`
    Expression,

    /// Objects as a whole: `(username != "admin")`, can discard the entire
    /// object if the expression evaluates to false.
    /// `(temperature > 32)`
    /// Filter map: filter out temps lower than 32, maps the reading to
    /// upper case, leaving only the uppercase reading.
    /// `(temperature > 32, reading.upper())`
    /// `[temperature > 32]`
    /// `(> 32)` or `(!= "admin")`
    /// `[> 32]`
    Filter,
}

use thiserror::Error;

#[derive(Debug, Error)]
#[error("Pique Error")]
pub enum PqErr
{
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("error during tokenization")]
    LexErr,

    #[error(transparent)]
    ParseIntErr(#[from] std::num::ParseIntError),

    #[error(transparent)]
    ParseFloatErr(#[from] std::num::ParseFloatError),

    #[error(transparent)]
    CliErr(#[from] clap::Error),

    #[error("query failed: {0}")]
    QueryErr(&'static str),
}

pub type PqResult<T> = Result<T, PqErr>;

// ! The table is append-only. Use scratch space for intermediary row results.
// ! Every query in the pipeline represents a new json lines document

// TODO: make this use [table::JsonTable] instead of solution in parser.rs
/// Input table is modified by appending resulting rows onto it.
pub fn execute_pipeline(
    table: &mut Vec<Row>,
    pipeline: &[Query],
) -> PqResult<usize>
{
    let mut queries = pipeline.iter();
    let mut scratch: Vec<Row> = vec![];
    let mut id = 0;

    while let Some(query) = queries.next()
    {
        match query
        {
            Query::SelectKey(select) =>
            {
                // If not first and no sibling and not last empty obj/arr?
                // Find first child,
                // Iter siblings
                // Find key
                // Set next id to key's value (whole document becomes that val)
                // Create new doc by assinging val.par = val.id

                if table[id].ty != RowType::Obj
                {
                    let err =
                        PqErr::QueryErr("cannot select key from non-object");
                    return Err(err);
                }

                let row =
                    table[id].subnodes(table).find(|row| row.key == *select);

                if let Some(selected) = row.cloned()
                {
                    id = selected.id as usize;

                    // TODO: When new nodes are pushed, their id's no longer
                    // TODO: equal their index in the overal tree. This is way
                    // TODO: bad for subsequent queries.
                    // TODO: Idea: graft the scratch buf into the existing tree
                    table.push(selected.make_root());

                    // ! 1
                    // table.reserve(selected.slice_tree(table).len());
                    // scratch.extend_from_slice(selected.slice_tree(table));
                    // table.extend_from_slice(&scratch[..]);
                    // scratch.clear();

                    // ! 2
                    table.extend_from_within(
                        id .. id + selected.slice_tree(table).len(),
                    );
                }
                else
                {
                    return Err(PqErr::QueryErr("selected key not found"));
                };
            }
            Query::FilterKey(filter) => todo!(),
            // Query::SelectIndex => todo!(),
            // Query::FilterIndex => todo!(),
            // Query::NewArr => todo!(),
            // Query::NewObj => todo!(),
            // Query::Fanout => todo!(),
            // Query::Join => todo!(),
            // Query::Keys => todo!(),
            // Query::Values => todo!(),
            // Query::_Spread => todo!(),
            // Query::_Fanout => todo!(),
            // Query::_Join => todo!(),
            // Query::_Select => todo!(),
            // Query::Expression => todo!(),
            // Query::Filter => todo!(),
            _ => todo!("query handler not implemented yet"),
        }
    }

    Ok(id)
}
