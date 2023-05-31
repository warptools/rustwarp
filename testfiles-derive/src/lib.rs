
use glob::glob;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::env;
use std::path::PathBuf;
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
    let mut search_path = PathBuf::from(start_path);
    search_path.push(pattern);

    // Let the glob library do the walking for us.
    //  Note that this gets us paths, but it's still going to leave us some future munging to do:
    //   The env var above gave us an absolute path...
    //   and this glob library has no concept of working relative to another path parameter, either.
    //  So, that's gonna leave us stripping prefixes back off, later.  It's unfortunate that this can't be more efficient.
    let paths: glob::Paths = glob(&search_path.to_string_lossy()).unwrap(); // If this step errors, it's because the pattern didn't compile; we can panic for this.

    // For each found path: make a new token stream with a test declaration.  Collect a vec.
    let tests = paths
        .map(|visit: Result<PathBuf, glob::GlobError>| {
            let path = visit.unwrap(); // I suppose I/O errors might as well be a panic too; what else should they do?
            let path_str = path.to_string_lossy(); // Need this because paths aren't tokenizable ;)  Note that this is an absolute path, whether we like it or not.
		// TODO prefix strip needed as part of constructing test_name.
            let test_name = path_str.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
            // TODO might need a super:: in this next bit?
            quote! {
            #[test]
            fn #test_name() {
                let p = std::path::Path::new(stringify!(#path_str));
                #function_name(p);
            }
            }
        })
        .collect::<Vec<_>>();
    if tests.len() < 1 {
        panic!(
            "this glob matched no files!  (hint: your cwd is {:?}.  we searched at {:?})",
            env::current_dir().unwrap(),
            search_path,
        )
    }

    let test_module_name = function_name;

    quote! {
      #[cfg(test)]
      mod #test_module_name {
        #[test]
        fn it_works() {
            print!("hello!  we did it!\n")
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
