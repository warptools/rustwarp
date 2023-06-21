/*
Some notes about macros:

- The docs at https://developerlife.com/2022/03/30/rust-proc-macro/ are pretty wonderful.
- Doing several features together in one macro like this has some consequences...
	- The upside is fewer macros for the user to be bothered with calling, obviously.
	- Downside: it's all or nothing.  If there's a panic or invalid token streams from any of the work, then you don't get *any* usable result.
		- Since we're generating stringers that you might actually use in debugging, this is... not ideal.
	- Downside: if there's a compile error within the generated output (that is, token stream is valid, just not typechecking)... the users gets all errors pointing at just the one macro line.
		- If using the '-Z macro-backtrace' feature of the compiler, this probably isn't a problem.  (I don't know why Rust doesn't ship that beyond nightlies.  It's so useful.)
- There's different kinds of things i'd have called attributes.
	- a `proc_macro_attribute` is for parsing stuff like `#[foobar(key = "value")]`.
	- apparently there's also just "non-macro attributes", and that's what `#[foobar = "value"]` is (... I infer, from trying to grok some compiler error messages?).
	- oh wow there's also "derive macro helper attributes", and that's a whole other thing.
	- ... I still have very little idea what's going on here.  There's a lot of twisty little passages here, very much alike, except for the ways in which they're not.
	- https://doc.rust-lang.org/reference/procedural-macros.html#derive-macro-helper-attributes is one of several docs I wish I had found much earlier than I did.

If you're looking for alternative crates that do similar things to this:

- https://jeltef.github.io/derive_more/derive_more/display.html looks a bit close!
	- It's more powerful.  You can customize quite a bit with its annotations, including even handing more code strings into its macros.
	- It's just the Display part, not the FromStr part.
		- https://jeltef.github.io/derive_more/derive_more/from_str.html does exist, but it's only for structs with one field -- very very limited.

*/

/*
Some notes about rust enums (aka sum types):

- Rust enums do already have a property called discriminants... but it's distinct from what we're handling here.
	- Rust's defn of it is (almost) always an int.  It has more to do with memory layout than anything else.
	- That's fine and dandy, but much of what we're working with here is strings.  So that means we have our own annotations to attach those.
		- I don't think it's either possible, nor even desirable, to attempt to overload the existing rust concept with our strings.
	- See https://doc.rust-lang.org/reference/items/enumerations.html#discriminants for more detail.

*/

/*
Some notes about other serialization and data munging options out there:

- `#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr, catverters::Stringoid)]` play very very nicely together!
- https://docs.rs/serde_with/3.0.0/serde_with/guide/serde_as_transformations/index.html describes a lot of good stuff you can do with serde.

*/

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// derive_stringoid generates implementations of `std::fmt::Display` and `std::str::FromStr`
/// for the struct or enum type it's applied on.
///
/// Separator characters must be defined.  TODO how??
///
/// It is assumed all fields themselves also implement `std::fmt::Display` and `std::str::FromStr`.
///
/// For enums, the discrimant string will be prefixed, then the separator,
/// then the member value a string; parsing is the reverse.
/// The discriminant is the variant name by default,
/// but can be overriden with an annotation: `#[discriminant = "override"]` on a member does the trick.
///
/// For structs, each field is stringified, and they are joined by the separator.
/// Parsing splits on the separator, greedily
/// (meaning excess instances of the separator are acceptable, and will simply
/// end up treated as part of string of the final value of the struct, and handed to that type to process further).
///
/// TODO: separators aren't configurable!  Coming soon!
#[proc_macro_derive(Stringoid, attributes(discriminant))]
pub fn derive_stringoid(input: TokenStream) -> TokenStream {
	let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

	// TODO: this isn't the main purpose, but maybe we should do something like this as part of also emitting helpers for generating error messages.
	let description_str = match &data {
		syn::Data::Struct(typ) => match &typ.fields {
			syn::Fields::Named(fields) => {
				let my_named_field_idents = fields.named.iter().map(|it| &it.ident);
				format!(
					"a struct with these named fields: {}",
					quote! {#(#my_named_field_idents), *}
				)
			}
			syn::Fields::Unnamed(fields) => {
				let my_unnamed_fields_count = fields.unnamed.iter().count();
				format!("a struct with {} unnamed fields", my_unnamed_fields_count)
			}
			syn::Fields::Unit => format!("a unit struct"),
		},
		syn::Data::Enum(typ) => {
			let my_variant_idents = typ.variants.iter().map(|it| &it.ident);
			format!(
				"an enum with these variants: {}",
				quote! {#(#my_variant_idents),*}
			)
		}
		syn::Data::Union(_) => panic!("unsupported!"),
	};

	let fmt_body: proc_macro2::TokenStream = match &data {
		syn::Data::Struct(typ) => {
			let field_strings = match &typ.fields {
				syn::Fields::Named(fields) => fields
					.named
					.iter()
					.map(|field| {
						let field_name = field.ident.clone().unwrap();
						quote! {
							self.#field_name.to_string()
						}
					})
					.collect::<Vec<_>>(),
				syn::Fields::Unnamed(fields) => fields
					.unnamed
					.iter()
					.enumerate()
					.map(|(index, _)| {
						let field_index = syn::Index::from(index);
						quote! {
							self.#field_index.to_string()
						}
					})
					.collect::<Vec<_>>(),
				syn::Fields::Unit => vec![],
			};

			if field_strings.is_empty() {
				quote! {
					write!(f, "{}", stringify!(#ident))
				}
			} else {
				quote! {
					write!(f, "{}", vec![#(#field_strings),*].join(":"))
				}
			}
		}
		syn::Data::Enum(typ) => {
			let arms = typ.variants.iter().map(|variant: &syn::Variant| {
				let variant_name = &variant.ident;
				let variant_descrim = get_variant_discriminant(variant);

				// Write the match arm.
				//  This starts with the match pattern, which is the variant type name.
				//  We always call the value "val".
				//  And we let the write macro, which is ending up in the real output, simply take val,
				//   and stringify it as best it can... which means it's going to look for Display traits, and thus should compose nicely.
				quote! {
				  #ident::#variant_name ( val ) => {
					write!(f, "{}:{}", #variant_descrim, val)
				  }
				}
			});
			// Gather up all the match arms into the actual match; and that's it: that's the whole body of fmt for enums.
			quote! {
			  match self {
				#(#arms),*
				}
			}
		}
		syn::Data::Union(_) => panic!("unsupported!"),
	};

	let fromstr_body: proc_macro2::TokenStream = match &data {
		syn::Data::Struct(typ) => match &typ.fields {
			syn::Fields::Named(fields) => {
				// For each field, create a local value of that type and with the same name as the field with a let, and parse into it.
				let entries = fields.named.iter().map(|field| {
					let field_name = &field.ident;
					let field_type = &field.ty;
					quote! {
						let #field_name: #field_type = parts.next().ok_or("unreachable length mismatch")?.parse()?;
					}
				});

				// The whole func body is just splitting on delimiter first,
				//  then gathering our 'let' lines above,
				//  then if all that went well, return a success Result containing a struct gathering all the values.
				// (We're sort of abusing the fact that the field names and our let's above used the same string.  It's positional assignemnt here.)
				let field_count = fields.named.len();
				let field_names = fields.named.iter().map(|field| &field.ident);
				quote! {
					let mut parts = s.splitn(#field_count, ':');

					#(#entries)*

					Ok(#ident{
						#(#field_names),*
					})
				}
			}
			syn::Fields::Unnamed(_) => {
				quote! {
					Err(<Self as std::str::FromStr>::Err::from("not yet implemented"))
				}
			}
			syn::Fields::Unit => panic!("unsupported!"),
		},
		syn::Data::Enum(typ) => {
			let arms = typ.variants.iter().map(|variant: &syn::Variant| {
                let variant_name = &variant.ident;
                let variant_descrim = get_variant_discriminant(variant);
		    let variant_type = match &variant.fields {
			syn::Fields::Named(fields) => {
				if fields.named.len() != 1 {
					panic!("unsupported!  Stringoid enums must have one type in each of their members.")
				};
				fields.named.first()
			},
			syn::Fields::Unnamed(fields) =>  {
				if fields.unnamed.len() != 1 {
					panic!("unsupported!  Stringoid enums must have one type in each of their members.")
				};
				fields.unnamed.first()
			},
			syn::Fields::Unit => panic!("unsupported!  Stringoid enums must have one type in each of their members."),
		  };

                // Write the match arm.
                //  The string of the descriminator is the match clause;
		    //  then we call the from_str on the inhabitant type (of which there must only be one, for clarity's sake -- no tuples);
		    //  and then assuming that flies, we wrap it in the enum type and that in a successful Result.
            // The use of `as ::std::str::FromStr` is because we use these tokens in both `FromStr` and `TryFrom<&str>`... I don't know if this should be regarded as clean, but it's what this code does right now.
		    // TODO: the error from the inhabitant's from_str call should probably get wrapped with further explanation.  It doesn't currently explain how we got to trying to parse that type.
                quote! {
                  #variant_descrim => {
				let inhabitant = <#variant_type as ::std::str::FromStr>::from_str(rest)?;
				Ok(#ident::#variant_name(inhabitant))
			}
                }
            });

			// The first thing the parse needs to do is split on the separator.
			// Then it's a matching job: gather up the arms we prepared above.
			// FUTURE: consider if the error type `serde::de::Error::unknown_variant` might be appropriate.  (This isn't serde, though... so perhaps not.)
			quote! {
			 let (prefix, rest) = s.split_once(':').ok_or("wrong number of separators")?;
			  match prefix {
				#(#arms),*,
				_ => Err(Box::new(catverters::Error::UnknownDiscriminant{type_name: stringify!(#ident).to_string(), value: prefix.to_string()})),
			  }
			}
		}
		syn::Data::Union(_) => panic!("unsupported!"),
	};

	quote! {
	impl #ident {
		fn describe(&self) -> String {
			let mut string = String::from(stringify!(#ident));
			string.push_str(" is ");
			string.push_str(#description_str);
			string
		}
	}

	impl std::fmt::Display for #ident {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			#fmt_body
		}
	  }

	  impl std::str::FromStr for #ident {
		type Err = Box<dyn std::error::Error>; // TODO: why can't this be more specific, like `type Err = Box<catverters::Error>;`?  And why is Box seemingly necessary?

		fn from_str(s: &str) -> Result<Self, Self::Err> {
			#fromstr_body
		}
	}

	impl std::convert::TryFrom<&str> for #ident {
		type Error = Box<dyn std::error::Error>; // TODO: why can't this be more specific, like `type Err = Box<catverters::Error>;`?  And why is Box seemingly necessary?
		fn try_from(s: &str) -> Result<Self, Self::Error> {
			#fromstr_body
		}
	}
	}
	.into()
}

// Returns a string -- unquoted -- for the variant discriminator.
// If it's been annotated explicitly, you get that string.
// Otherwise, you get the variant's name.
fn get_variant_discriminant(variant: &syn::Variant) -> String {
	variant
		.attrs
		.iter()
		.find_map(|attr| {
			if attr.path.is_ident("discriminant") {
				if let Ok(syn::Meta::NameValue(meta_name_value)) = attr.parse_meta() {
					if let syn::Lit::Str(lit_str) = meta_name_value.lit {
						return Some(lit_str.value());
					}
				}
			}
			None
		})
		.unwrap_or_else(|| variant.ident.to_string())
}
