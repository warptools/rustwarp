use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::env;
use std::result::Result;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn test_per_file(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as syn::AttributeArgs);
	let input = parse_macro_input!(item as syn::ItemFn);
	let function_name = &input.sig.ident;

	// Parse the user's arguments to the macro.
	let pattern = extract_glob_pattern(&args).unwrap(); // Panic if the parameters of the attribute were unexpected.

	// Figure out where we are.
	// This involves a couple of steps:
	// - Rust tooling generally sets the CWD to be the workspace root, so that's nicely stable;
	// - But we want the paths to be relative to either the crate, or ideally the source file itself... that's trickier.
	// So we look around a bit and compose.
	//
	// The ideal thing, IMO, would be to use `proc_macro2::Span::call_site().source_file().path()`.
	//  Unfortunately... that thing is hidden behind so many conditional compilation flags it made my eyes water.
	//   And that, apparently, in turn, is because further upstreams have done similar.
	//    https://github.com/rust-lang/rust/issues/54725 is the issue to watch.
	//
	// So we'll read an environment variable.  Fine.
	//
	// And this only gets us to the crate.  Not the source file.  I guess we'll have to be happy with that.
	let start_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();

	// Let the glob library do the walking for us.
	let walker = globwalk::GlobWalkerBuilder::from_patterns(&start_path, &[pattern])
		.build()
		.unwrap()
		.filter_map(Result::ok);

	// For each found path: make a new token stream with a test declaration.  Collect a vec.
	let tests = walker
		.map(|visit: walkdir::DirEntry| {
			// We'll embed the path as a quoted string in the generated test function.
			// Note that this is an absolute path.
			//  The CWD when cargo is running things is the workspace root (may be different than the CARGO_MANIFEST_DIR, which is the crate root),
			//  but we can totally ignore that since this path is absolute.
			//  (It also means "don't ship these binaries if you don't want to leak details of your host", but hey, we made this for testing.)
			let path_str = visit.path().to_string_lossy();

			// We want a test name that's derived from the filename.
			//  Now here, that globwalk doesn't return paths that are relative to the search root... is annoying.
			//   I guess we'll just crudely chop off the known prefix.
			// Then we'll replace any characters that have any chance of being scary.
			//  (If this produces name collisions, you'll get compile errors; deal with it.)
			let relevant_path = visit
				.path()
				.strip_prefix(start_path.as_str())
				.unwrap()
				.to_string_lossy();
			let test_name = relevant_path.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
			// Create an `Ident` for the test function name.
			//  For some reason, `quote!` requires this value in particular to be an `Ident` or it won't insert it.
			let test_ident = syn::Ident::new(test_name.as_str(), Span::call_site());

			// Now, putting it together is easy: We just want:
			// - a test function, with the annotation,
			// - that calls the user's original function (which we'll refer to with "super" because we're going to wrap this in a module),
			// - and we construct a path from the string we discovered during our walk and give that to them.
			quote! {
			  #[test]
			  fn #test_ident() {
				  super::#function_name(Path::new(#path_str));
			  }
			}
		})
		.collect::<Vec<_>>();
	if tests.is_empty() {
		panic!(
			"this glob matched no files!  (hint: your cwd is {:?}.  we searched at {:?})",
			env::current_dir().unwrap(),
			start_path,
		)
	}

	let test_module_name = function_name;

	// In total:
	// - Pass through the original input (we're still gonna call that function).
	//   - Do this _outside_ the module we're about to introduce.  (We don't want to fuck with its scopes or the namespace it's allowed to `use`.)
	// - Add a new module to contain and namespace all our new tests.
	//   - (We use the same name as the function; that doesn't collide.  Modules and funcs are in different namespaces.)
	// - (Between the above points: This is why "super::" is used inside each test to call out.)
	// - Emit all the new test token streams inside the module.
	// - That's it!  Ta-da!
	quote! {
	  #input

	  mod #test_module_name {
	  use std::path::Path;

		#[test]
		fn this_is_test_per_file_generated() {
			print!("hello!  this is a dummy test to show that test_per_file worked!\n")
		}

		#(#tests)*
	  }
	}
	.into()
}

fn extract_glob_pattern(args: &[syn::NestedMeta]) -> Result<String, syn::Error> {
	for arg in args {
		if let syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) = arg {
			if name_value.path.is_ident("glob") {
				if let syn::Lit::Str(pattern) = &name_value.lit {
					return Ok(pattern.value());
				} else {
					return Err(syn::Error::new_spanned(
						&name_value.lit,
						"Expected string literal",
					));
				}
			}
		}
	}

	Err(syn::Error::new(
		Span::call_site(),
		"Missing 'glob' argument in #[test_per_file]",
	))
}
