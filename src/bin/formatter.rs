use serde_json::Value;

/*
"Containers": "N/A",
"CreatedAt": "2023-01-27 10:19:03 -0500 EST",
"CreatedSince": "2 years ago",
"Digest": "<none>",
"ID": "24bc64e91103",
"Repository": "registry.k8s.io/etcd",
"SharedSize": "N/A",
"Size": "181MB",
"Tag": "3.5.7-0",
"UniqueSize": "N/A",
"VirtualSize": "180.9MB"
*/

fn main() -> Result<(), std::io::Error>
{
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin())?;

    // Load the keys & values into rows in SQL or Polars
    // [int id, int parent, string key, int type, int value]
    // [0, 0, "", 0, 0]
    // If value is arr: value = id of arr
    // If value is obj: value = id of obj
    // Else: value = bool|num|null|str stored as strings in value table

    // All of this assumes that no rows are deleted or inserted in between
    #[rustfmt::skip]
    #[repr(u8)]
    #[derive(Clone, Copy)]
    enum ValType { NewObj, EndObj, NewArr, EndArr, Null, Bool, Str, Num, }

    enum Cmd
    {
        MapExpr,
        FilterExpr,
        SelectKey,
        BuildObj,
        BuildArr,
        Fanout,
        Join,
        // FilterKeys,
    }

    impl std::fmt::Display for ValType
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
        {
            format!("{self:?}")
                .split_once("::")
                .map(|(_, a)| f.write_str(a))
                .ok_or(Err(std::fmt::Error));

            let name =
                format!("{self:?}").split_once("::").ok_or(std::fmt::Error)?;

            // if let Some((_, type_name)) = format!("{self:?}").split_once("::")
            // {
            //     f.write_fmt(format_args!("{type_name}"))
            // }
            // else
            // {
            //     Err(std::fmt::Error)
            // }
        }
    }

    impl std::fmt::Debug for ValType
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
        {
            match self
            {
                Self::NewObj => write!(f, "new_obj"),
                Self::EndObj => write!(f, "end_obj"),
                Self::NewArr => write!(f, "new_arr"),
                Self::EndArr => write!(f, "end_arr"),
                Self::Null => write!(f, "null"),
                Self::Bool => write!(f, "bool"),
                Self::Str => write!(f, "str"),
                Self::Num => write!(f, "num"),
            }
        }
    }

    type KeysRow = (u32, String, u32); // parent id, key, value id
    type ValsRow = (ValType, String); // type, value (bool/null/num/str stringified)

    let mut tab_keys: Vec<KeysRow> = Vec::new();
    let mut tab_vals: Vec<ValsRow> = Vec::new();
    let mut queue: Vec<(u32, Value)> = vec![(0, value)]; // parent id, json
    let mut id_counter = 0;

    while let Some((parent, node)) = queue.pop()
    {
        let id = tab_keys.len() as u32;
        // if hit end of obj, pop parent id index counter
        // if hit start of arr, push parent id index counter
        match node
        {
            Value::Null => todo!(),
            Value::Bool(_) => todo!(),
            Value::Number(number) => todo!(),
            Value::String(_) => todo!(),
            Value::Array(values) => todo!(),
            Value::Object(map) =>
            {
                tab_vals.push((ValType::NewObj, String::new()));
                for (key, val) in map.iter()
                {
                    let val_id = tab_vals.len() as u32;

                    tab_keys.push((id, key.clone(), val_id));

                    match val
                    {
                        Value::Null =>
                        {
                            tab_vals.push((ValType::Null, String::new()));
                        }
                        Value::Bool(tf) =>
                        {
                            tab_vals.push((ValType::Bool, tf.to_string()))
                        }
                        Value::Number(num) =>
                        {
                            tab_vals.push((ValType::Num, num.to_string()))
                        }
                        Value::String(string) =>
                        {
                            tab_vals.push((ValType::Str, string.clone()))
                        }
                        // Value::Array(values) => ValType::Arr,
                        // Value::Object(map) => ValType::Obj,
                        _ => todo!(),
                    };
                }
                tab_vals.push((ValType::EndObj, String::new()));
            }
        }
    }

    println!("parent id, key, value id");
    for (udx_parent, key, udx_val) in tab_keys.into_iter()
    {
        println!("{udx_parent}, {key}, {udx_val}");
    }

    println!();

    println!("type, value");
    for (val_type, value) in tab_vals.into_iter()
    {
        println!("{val_type}, {value}");
    }

    Ok(())
}

// fn recurse(value: Value) -> Value {}
