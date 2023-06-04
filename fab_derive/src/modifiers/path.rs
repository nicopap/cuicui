use std::fmt;

use heck::AsUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{bracketed, Token};

fn attr_no_ident_terminal(full: &impl fmt::Display, terminal: &impl fmt::Display) -> String {
    format!(
        "\
In modify field path specification, the last
component of the path was '{terminal}'.
Since '{terminal}' is not an identifier, we
cannot use it as parameter name to pass
to the modify function.

Please provide an alternative with the following syntax:

    #[modify(read(alt_parameter_name = {full}))]
"
    )
}
const ATTR_SYNTAX_MSG: &str = "\
In modify attribute, no field path is declared,
we cannot know which field of the item to use as
modify function parameter. The syntax for modify
attributes is:

    #[modify(write([ident =] .path.0.[\"to\"].field[3]))].

[ident =] is the alternative parameter name. This
is optional, by default, the terminal field name is
used as parameter name to pass to the modify function.

If you want to access directly the whole context or
item, use a single identifier:

    #[modify(context(get_path))].
";

#[derive(Clone, PartialEq, Copy)]
pub enum Source {
    Context,
    Item,
}
impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Context => write!(f, "Context"),
            Source::Item => write!(f, "Item"),
        }
    }
}
impl Source {
    pub fn parse_path(&self, input: syn::parse::ParseStream) -> syn::Result<Path> {
        let span = input.span();
        let is_assignment = input.peek(syn::Ident) && input.peek2(Token![=]);

        let ident = if is_assignment {
            let ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;
            ident
        } else {
            let ident = input.cursor().token_stream().into_iter().enumerate().last();
            let Some((i, ident)) = ident else {
                return Err(syn::Error::new(input.span(), ATTR_SYNTAX_MSG));
            };
            let TokenTree::Ident(ident) = ident else {
                let msg = attr_no_ident_terminal(&input, &ident);
                return Err(syn::Error::new(ident.span(), msg));
            };
            // There is exactly one identifer as the whole path spec
            if i == 0 {
                let _ = input.parse::<Ident>()?;
                return Ok(Path { ident, span, components: Components::empty(*self) });
            }
            ident
        };
        let components = self.parse_comps(input)?;
        Ok(Path { ident, span, components })
    }
    fn parse_comps(&self, input: syn::parse::ParseStream) -> syn::Result<Components> {
        let span = input.span();
        let parse_single = || match () {
            () if input.peek(Token![.]) && input.peek2(syn::Ident) => {
                let _ = input.parse::<Token![.]>()?;
                Ok(Some(Component::Field(input.parse()?)))
            }
            () if input.peek(Token![.]) => {
                let _ = input.parse::<Token![.]>()?;
                Ok(Some(Component::TupleField(input.parse()?)))
            }
            () if input.peek(syn::token::Bracket) => {
                let content;
                bracketed!(content in input);

                match content.parse()? {
                    syn::ExprLit { lit: syn::Lit::Int(value), .. } => {
                        Ok(Some(Component::IntIndex(value)))
                    }
                    syn::ExprLit { lit: syn::Lit::Str(value), .. } => {
                        Ok(Some(Component::StringIndex(value)))
                    }
                    // TODO: be less vague
                    _ => Err(syn::Error::new(span, "Invalid modify path specifier")),
                }
            }
            () => Ok(None),
        };
        let mut components = Vec::new();
        while let Some(parsed) = parse_single()? {
            components.push(parsed);
        }
        if components.is_empty() {
            Err(syn::Error::new(span, "modify targets must be non-empty"))
        } else {
            Ok(Components { path: components, source: *self })
        }
    }
}
#[derive(Clone)]
pub struct Components {
    pub(super) path: Vec<Component>,
    pub(super) source: Source,
}
impl Components {
    fn empty(source: Source) -> Self {
        Components { path: Vec::new(), source }
    }
    pub fn to_tokens(&self) -> TokenStream {
        let components = self.path.iter().map(|c| c.to_tokens());
        quote!( #( #components )* )
    }
    pub(super) fn variant_ident(&self, span: Span) -> Ident {
        let name = self.variant_fmt().to_string();
        Ident::new(&name, span)
    }

    pub(crate) fn doc_string(&self) -> String {
        if self.path.is_empty() {
            format!("The [`Modify::{}`].", self.source)
        } else {
            format!(
                "The path `{}` in the [`Modify::{}`].",
                self.rust_fmt(),
                self.source
            )
        }
    }
    fn rust_fmt(&self) -> PathRust {
        PathRust(&self.path)
    }
    fn variant_fmt(&self) -> PathVariant {
        PathVariant(self)
    }
    pub(super) fn pretty_fmt(&self) -> PathPretty {
        PathPretty(self)
    }
}
#[derive(Debug, Clone)]
pub(super) enum Component {
    Field(Ident),
    TupleField(syn::Index),
    StringIndex(syn::LitStr),
    IntIndex(syn::LitInt),
}
impl PartialEq for Component {
    fn eq(&self, other: &Self) -> bool {
        use Component::*;

        match (self, other) {
            (Field(ident1), Field(ident2)) => ident1 == ident2, // https://docs.rs/proc-macro2/latest/src/proc_macro2/lib.rs.html#1003-1007
            (TupleField(idx1), TupleField(idx2)) => idx1 == idx2, // https://docs.rs/syn/latest/src/syn/expr.rs.html#820-824,
            (StringIndex(idx1), StringIndex(idx2)) => idx1.value() == idx2.value(),
            (IntIndex(idx1), IntIndex(idx2)) => idx1.base10_digits() == idx2.base10_digits(),
            _ => false,
        }
    }
}
impl Component {
    pub(super) fn rust_fmt(&self) -> ComponentRust {
        ComponentRust(self)
    }
    pub(super) fn variant_fmt(&self) -> ComponentVariant {
        ComponentVariant(self)
    }
    fn to_tokens(&self) -> TokenStream {
        use Component::*;

        match self {
            Field(field) => quote!(. #field),
            TupleField(field) => quote!(. #field),
            StringIndex(lit) => quote!([#lit]),
            IntIndex(lit) => quote!([#lit]),
        }
    }
}
pub(super) struct PathRust<'a>(&'a [Component]);
impl fmt::Display for PathRust<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for comp in self.0.iter() {
            write!(f, "{}", comp.rust_fmt())?;
        }
        Ok(())
    }
}
pub(super) struct PathVariant<'a>(&'a Components);
impl fmt::Display for PathVariant<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.source)?;
        for comp in self.0.path.iter() {
            write!(f, "{}", comp.variant_fmt())?;
        }
        Ok(())
    }
}
pub(super) struct PathPretty<'a>(&'a Components);
impl fmt::Display for PathPretty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.source)?;
        write!(f, "{}", self.0.rust_fmt())
    }
}
pub(super) struct ComponentRust<'a>(&'a Component);
impl fmt::Display for ComponentRust<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Component::*;
        match self.0 {
            Field(ident) => write!(f, ".{ident}"),
            TupleField(idx) => write!(f, ".{}", idx.index),
            StringIndex(idx) => write!(f, "[\"{}\"]", idx.value()),
            IntIndex(idx) => write!(f, "[{}]", idx.base10_digits()),
        }
    }
}
pub(super) struct ComponentVariant<'a>(&'a Component);
impl fmt::Display for ComponentVariant<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Component::*;

        match &self.0 {
            Field(ident) => write!(f, "{}", AsUpperCamelCase(ident.to_string())),
            TupleField(index) => write!(f, "{}", index.index),
            StringIndex(index) => write!(f, "At{}", AsUpperCamelCase(index.value())),
            IntIndex(index) => write!(f, "At{}", index.base10_digits()),
        }
    }
}

pub struct Path {
    pub ident: Ident,
    pub components: Components,
    pub span: Span,
}
impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = ", &self.ident)?;
        if self.components.path.is_empty() {
            write!(f, "{}", &self.ident)?;
        }
        for comp in &self.components.path {
            write!(f, "{}", comp.rust_fmt())?;
        }
        Ok(())
    }
}
impl Path {
    pub fn to_tokens(&self) -> TokenStream {
        self.components.to_tokens()
    }
}
