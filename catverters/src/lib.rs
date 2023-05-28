/*
Some notes about macros:

- The docs at https://developerlife.com/2022/03/30/rust-proc-macro/ are pretty wonderful.

*/

/*
Some notes about rust enums (aka sum types):

- Rust enums do already have a property called discriminants... but it's distinct from what we're handling here.
    - Rust's defn of it is (almost) always an int.  It has more to do with memory layout than anything else.
    - That's fine and dandy, but much of what we're working with here is strings.  So that means we have our own annotations to attach those.
        - I don't think it's either possible, nor even desirable, to attempt to overload the existing rust concept with our strings.
    - See https://doc.rust-lang.org/reference/items/enumerations.html#discriminants for more detail.

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
/// but can be overriden with an annotation.  TODO how??
///
/// For structs, each field is stringified, and they are joined by the separator.
/// Parsing splits on the separator, greedily
/// (meaning excess instances of the separator are acceptable, and will simply
/// end up treated as part of string of the final value of the struct, and handed to that type to process further).
///
/// TODO: so far only Display is actually implemented!  FromStr coming soon!
/// TODO: separators aren't configurable!  Coming soon!
/// TODO: discriminants aren't overridable!  Coming soon!
#[proc_macro_derive(Stringoid)]
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

    let fmt_body = match &data {
        syn::Data::Struct(typ) => {
            let field_strings = match &typ.fields {
                syn::Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|field| {
                        let field_name = field.ident.clone().unwrap();
                        quote! {
                            self.#field_name.to_string()
                        }
                    })
                    .collect::<Vec<_>>(),
                syn::Fields::Unnamed(unnamed) => unnamed
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
                let variant_descrim = variant_name; // TODO: make this customizable.

                // Write the match arm.
                //  This starts with the match pattern, which is the variant type name.
                //  We always call the value "val".
                //  And we let the write macro, which is ending up in the real output, simply take val,
                //   and stringify it as best it can... which results in overall correct composition, if everything you've got is using Display/FromStr consistently.
                //    (It's not particularly compile-time safe if something is inconsistent on that front, though; it's arguably a bit too willing to procede whimsically.)
                quote! {
                  #ident::#variant_name ( val ) => {
                    write!(f, "{}:{}", stringify!(#variant_descrim), val)
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
    }
    .into()
}
