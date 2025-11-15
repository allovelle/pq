use allocative::Allocative;
use derive_more::Display;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::typing::Ty;
use starlark::values::Value;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::{AllocValue, StarlarkValue};
use starlark::values::{Heap, ProvidesStaticType};
use starlark::values::{NoSerialize, ValueError};
use starlark_derive::starlark_value;

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative)]
#[display("Foo")]
struct Foo;
#[starlark_value(type = "foo")]
impl<'v> StarlarkValue<'v> for Foo {}

//
//
//

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative)]
// For more basic
struct MyNumber
{
    #[display("int: {_0}")]
    val: i32,
    // #[display("int: {_0}")]
    // log: Vec<&'static str>, // record operations
}

impl MyNumber
{
    fn new(val: i32) -> Self
    {
        Self { val }
    }
}

impl<'v> AllocValue<'v> for MyNumber
{
    fn alloc_value(self, heap: &'v Heap) -> Value<'v>
    {
        heap.alloc_simple(self)
    }
}

#[starlark_value(type = "my_num", UnpackValue, StarlarkTypeRepr)]
impl<'v> StarlarkValue<'v> for MyNumber
{
    type Canonical = Self;

    // fn get_type(&self) -> &'static str
    // {
    //     "MyNumber"
    // }

    // fn add(&self, rhs: Value<'v>, heap: &'v Heap) -> ValueResult<'v>        Option<crate::Result<Value<'v>>>
    fn add(
        &self,
        rhs: Value<'v>,
        heap: &'v Heap,
    ) -> Option<starlark::Result<Value<'v>>>
    {
        if let Some(int_rhs) = rhs.unpack_i32()
        {
            let mut new = MyNumber {
                val: self.val + int_rhs,
                // log: {
                //     let mut l = self.log.clone();
                //     l.push("add called");
                //     l
                // },
            };
            Some(Ok(heap.alloc(new)))
        }
        else
        {
            Some(Err(ValueError::IncorrectParameterType.into()))
        }
    }
}

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

    let allocated_value: Value = module.heap().alloc(MyNumber::new(-123456));
    module.set("my_num", allocated_value);

    let value: Value = eval.eval_module(ast_for("my_num")?, &globals)?;
    println!("expr => {}", value);

    Ok(())
}

fn ast_for(code: impl AsRef<str>) -> Result<AstModule, starlark::Error>
{
    AstModule::parse("[MODULE]", code.as_ref().to_owned(), &Dialect::Standard)
}
