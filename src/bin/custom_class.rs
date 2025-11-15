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
use starlark::values::{FrozenValue, ProvidesStaticType};
use starlark::values::{NoSerialize, Value};
use starlark::values::{StarlarkValue, StringValue};
use starlark_derive::starlark_value;

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative)]
#[display("Foo")]
struct Foo;
#[starlark_value(type = "foo")]
impl<'v> StarlarkValue<'v> for Foo {}

fn main() -> Result<(), starlark::Error>
{
    // let globals = Globals::standard();
    let mut builder = starlark::environment::GlobalsBuilder::standard();
    builder.set("IT", 123);
    let globals = builder.build();

    let module = Module::new();
    let mut eval = Evaluator::new(&module);

    let value: Value = eval.eval_module(ast_for("IT")?, &globals)?;
    println!("expr => {}", value);

    module.set("prev", value);

    let value: Value = eval.eval_module(ast_for("[prev, IT]")?, &globals)?;
    println!("expr => {}", value);

    Ok(())
}

fn ast_for(code: impl AsRef<str>) -> Result<AstModule, starlark::Error>
{
    AstModule::parse("[MODULE]", code.as_ref().to_owned(), &Dialect::Standard)
}
