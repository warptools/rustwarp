use starlark::environment::FrozenModule;
use starlark::environment::Globals;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::syntax::AstModule;
use starlark::syntax::Dialect;

fn heyo() -> Result<FrozenModule, starlark::Error> {
	let content = r#"
def hello():
   return "hello"
x = hello() + " world!"
"#;

	// We first parse the content, giving a filename and the Starlark
	// `Dialect` we'd like to use (we pick standard).
	let ast: AstModule =
		AstModule::parse("hello_world.star", content.to_owned(), &Dialect::Standard)?;

	// We create a `Globals`, defining the standard library functions available.
	// The `standard` function uses those defined in the Starlark specification.
	let globals: Globals = Globals::standard();

	// We create a `Module`, which stores the global variables for our calculation.
	let module: Module = Module::new();

	// Evaluation happens in its own block, because of lifetime issues.
	// (Evaluation borrows the module, and we want full ownership of the module after evaluation so we can freeze it,
	// because it's in turn nearly impossibly difficult to use values from the module without freezing it first.
	// Building your own heaps and freezers is a maze; I'm not even certain enough of it is exported to be possible.
	// I see other code using this crate also seems to return FrozenModule, so I guess this is the way!)
	{
		let mut eval: Evaluator = Evaluator::new(&module);
		// eval.set_loader(loader)
		// eval.set_print_handler(handler)

		// Evaluating the module does return a value, but we ignore it because the lifetime issues make it nearly unusable.
		// Fortunately, it's also not usually a super interesting value, since it's just the last thing that happened.
		eval.eval_module(ast, &globals)?;

		// Profiling seems to be gatherable only at the scale of the evaluator, and must also be saved separately.
		// I hope those are easy to aggregate across modules, but have not yet investigated further.
	}

	Ok(module.freeze()?) // Oddly, `freeze` returns an `anyhow` result instead of a starlark error.  Sigh.
}

#[test]
fn plz() {
	assert_eq!(
		heyo().unwrap().get("x").unwrap().unpack_str(),
		Some("hello world!")
	);
}
