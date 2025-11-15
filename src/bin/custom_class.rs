/*
use starlark::values::UnpackValue;
use starlark::values::ValueError;
use starlark::values::ValueResult;
use starlark::values::typing::Ty;
use starlark::values::{AllocValue, Heap, StarlarkValue, Value};

#[derive(Debug)]
struct MyNumber
{
    val: i32,
    log: Vec<&'static str>, // record operations
}

impl<'v> StarlarkValue<'v> for MyNumber
{
    type Canonical = Self;

    fn get_type(&self) -> &'static str
    {
        "MyNumber"
    }

    fn add(&self, rhs: Value<'v>, heap: &'v Heap) -> ValueResult<'v>
    {
        if let Some(int_rhs) = rhs.unpack_int()
        {
            let mut new = MyNumber {
                val: self.val + int_rhs,
                log: {
                    let mut l = self.log.clone();
                    l.push("add called");
                    l
                },
            };
            Ok(heap.alloc(new))
        }
        else
        {
            Err(ValueError::IncorrectType(rhs.get_type().to_owned()))
        }
    }
}
*/

use allocative::Allocative;
use derive_more::Display;
use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::{NoSerialize, Value};
use starlark_derive::starlark_value;

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative)]
#[display("Foo")]
struct Foo;
#[starlark_value(type = "foo")]
impl<'v> StarlarkValue<'v> for Foo {}

fn main() -> Result<(), starlark::Error>
{
    let globals = Globals::standard();
    let module = Module::new();
    let mut eval = Evaluator::new(&module);

    let code = r#"
        it = IT
        it
    "#;
    let code = r#""hello" + " world!""#;

    let ast =
        AstModule::parse("[MODULE]", code.to_owned(), &Dialect::Standard)?;

    let value: Value = eval.eval_module(ast, &globals)?;

    println!("expr => {}", value);

    Ok(())
}
