// src/main.rs
use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::{Heap, Value};

// fn eval_code(code: &str) -> Result<Value, starlark::Error>
// {
//     // Parse Starlark source using the Standard dialect.
//     let ast =
//         AstModule::parse("repl.star", code.to_owned(), &Dialect::Standard)?;

//     // Set up a module (globals + locals) and an evaluator.
//     let globals = Globals::standard();
//     let module = Module::new();
//     let mut eval = Evaluator::new(&module);

//     // Evaluate the module; returns the last expression’s value (or None if no expression).
//     let v = eval.eval_module(ast, &globals)?;
//     Ok(v)
// }

// fn main() -> Result<(), starlark::Error>
// {
//     // 1) Evaluate a pure expression
//     let expr = r#""hello" + " world!""#;
//     let v = eval_code(expr)?;
//     println!("expr => {}", v);

//     // 2) Evaluate a statement (side effect)
//     let stmt = r#"x = 41 + 1"#;
//     let v = eval_code(stmt)?;
//     // Statements don’t produce an expression value; you can read variables from the module if needed.
//     println!("stmt => {}", v); // Often prints None

//     Ok(())
// }

// struct Interpreter<'eval, 'io, 'extra>
struct Interpreter
{
    globals: Box<Globals>,
    module: Box<Module>,
    // eval: Box<Evaluator<'eval, 'io, 'extra>>,
    // result: Box<Value<'eval>>,
}

impl Interpreter
{
    pub fn new() -> Self
    {
        let globals = Box::new(Globals::standard());
        let module = Box::new(Module::new());
        // let result = Box::new(Value::new_none());

        Self { globals, module }
    }

    pub fn eval(&mut self, code: String) -> Result<Value<'_>, starlark::Error>
    {
        let ast = AstModule::parse("[MODULE]", code, &Dialect::Standard)?;
        let mut eval = Box::new(Evaluator::new(&self.module));
        eval.eval_module(ast, &self.globals)
    }
}

fn main() -> Result<(), starlark::Error>
{
    let mut interpreter = Interpreter::new();
    let res: Value = interpreter.eval("[i for i in range(10)]".into())?;

    let heap = &Heap::new();
    let mut result = Vec::<i32>::new();
    for element in res.iterate(heap)?
    {
        result.push(element.unpack_i32().unwrap());
    }

    // let result = res.unpack_str().unwrap();

    println!("starlark result -> {result:?}");

    Ok(())
}
