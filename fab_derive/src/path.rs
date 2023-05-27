use std::fmt::Write;

use heck::AsUpperCamelCase;
use proc_macro2::{Ident, TokenStream, TokenTree};
use syn::{parse::Parse, Token};

const ATTR_SYNTAX_MSG: &str = "No input paths were specified, cannot know which field \
    of the item to use. The syntax for modify attributes is: \n\n\
    #[modify(write([ident =] .path.0.[\"to\"].field[3]))].\n\n\
    [ident =] is optional.";
struct FieldEnumName(Ident);
impl Parse for FieldEnumName {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use TokenTree as TT;

        let mut name = String::new();
        while let Ok(path_elem) = TT::parse(input) {
            match path_elem {
                // TODO: .0[0].hello["world"] Tuple0_Get0_Hello_GetWorld
                TT::Group(_) => {}
                TT::Ident(ident) => {
                    write!(&mut name, "{}", AsUpperCamelCase(ident.to_string()))
                        .expect("Can always write to a string");
                }
                TT::Punct(_) | TT::Literal(_) => {}
            }
        }
        if name.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                format!("Invalid path: {input}"),
            ));
        }
        Ok(FieldEnumName(Ident::new(&name, input.span())))
    }
}
#[derive(Debug)]
pub(crate) struct Path {
    pub ident: Ident,
    pub tokens: TokenStream,
    /// Name in the modify field enum variants.
    pub variant_ident: Ident,
}
impl Parse for Path {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let is_assignment = input.peek(syn::Ident) && input.peek2(Token![=]);

        let ident = if is_assignment {
            let ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;
            ident
        } else {
            let ident = input.cursor().token_stream().into_iter().last();
            let Some(ident) = ident else {
                return Err(syn::Error::new(input.span(), ATTR_SYNTAX_MSG));
            };
            let TokenTree::Ident(ident) = ident else {
                let msg = "The path spec doesn't end in an identifier, it cannot \
                    be referred to, use `(ident = .path.to[\"field\"])` syntax instead.";
                return Err(syn::Error::new(ident.span(), msg));
            };
            ident
        };
        let variant_ident = input.fork().parse::<FieldEnumName>()?.0;
        let tokens = input
            .parse::<TokenStream>()
            .expect("parsing TokenStream is infallible");
        Ok(Path { ident, tokens, variant_ident })
    }
}
