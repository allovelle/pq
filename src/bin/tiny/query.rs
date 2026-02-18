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

pub enum QueryOutputs
{
    Rows(Vec<Row>),
    Value(String),
}

pub enum QueryInputs
{
    Rows(Vec<Row>),
    Value(String),
}

pub fn query_pipeline(table: &[Row], pipeline: Vec<Query>) -> Vec<Row>
{
    // Placeholder for query execution logic
    // This is where we would implement the actual querying based on the Query enum
    table.to_vec() // For now, just return the input table
}

// ! The table is append-only. Use scratch space for intermediary row results.
// ! Every query in the pipeline represents a new json lines document

/// Input table is modified by appending resulting rows onto it.
fn execute_pipeline(table: &mut Vec<Row>, pipeline: &[Query])
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

                debug_assert_eq!(table[id].ty, RowType::Obj, "not obj");

                let row =
                    table[id].subnodes(table).find(|row| row.key == *select);

                match row
                {
                    Some(selected) =>
                    {
                        let mut selected = selected.clone();
                        selected.par = selected.id;
                        table.push(selected);
                    }
                    None => todo!(),
                }
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
}
