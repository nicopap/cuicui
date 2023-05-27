use std::mem;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse::Parse, parse::Parser, punctuated::Punctuated, spanned::Spanned, ItemFn, Token};

use crate::{extensions::GetIdentExt, path::Path};

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Read,
    Write,
}
impl Mode {
    fn pick<T>(self, (read, write): (T, T)) -> T {
        match self {
            Mode::Read => read,
            Mode::Write => write,
        }
    }
    fn is_mode(self) -> impl Fn(&&Modify) -> bool {
        move |m| match self {
            Mode::Read => m.is_read(),
            Mode::Write => m.is_write(),
        }
    }
}
struct ReadAndWrite {
    read: Ident,
    write: Ident,
    param_name: Ident,
}
impl Parse for ReadAndWrite {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let read = input.parse()?;
        let _ = input.parse::<Token![,]>()?;
        let write = input.parse()?;

        let param_name = if input.parse::<Token![,]>().is_ok() {
            input.parse()?
        } else {
            Ident::new("item", input.span())
        };
        Ok(ReadAndWrite { read, write, param_name })
    }
}
const MODIFY_ATTR_DESCR: &str = "\
- `context(ident)`: The context declared as `type Context = Foo;`
- `write(.path.in.item)`: path in item to write return value
- `write_mut([ident =] .path.in.item)`: write-only path in item to pass as `&mut ident`
- `read([ident =] .path.in.item)`: read-only path in item to pass as `&ident`
- `read_write([ident =] .path.in.item)`: read/write path in item to pass as `&mut ident`
- `dynamic_read_write(read_ident, write_ident [, ident])`: pass `&mut item` and read
  those fields for checking which paths in item are read from and writen to.
  The thirs optional parameter is which function argument to passt it to
  (by default it is `item`)\n";
#[derive(Debug)]
enum Modify {
    Context(Ident),
    Write(Path),
    WriteMut(Path),
    Read(Path),
    ReadWrite(Path),
    DynamicReadWrite(Ident, Ident, Ident),
}
fn iter_attrs(tokens: TokenStream) -> syn::Result<Punctuated<syn::MetaList, Token![,]>> {
    let parser = Punctuated::<syn::MetaList, Token![,]>::parse_separated_nonempty;
    parser.parse2(tokens)
}
impl Modify {
    fn parse_individual(input: syn::MetaList) -> syn::Result<Self> {
        use syn::parse2 as parse;

        let is = |ident: &str, path: &syn::Path| path.get_ident().map_or(false, |i| i == ident);
        let parsed = match &input.path {
            path if is("context", path) => Modify::Context(parse(input.tokens)?),
            path if is("write", path) => Modify::Write(parse(input.tokens)?),
            path if is("write_mut", path) => Modify::WriteMut(parse(input.tokens)?),
            path if is("read", path) => Modify::Read(parse(input.tokens)?),
            path if is("read_write", path) => Modify::ReadWrite(parse(input.tokens)?),
            path if is("dynamic_read_write", path) => {
                let ReadAndWrite { read, write, param_name } = parse(input.tokens)?;
                Modify::DynamicReadWrite(read, write, param_name)
            }
            path => {
                let msg = format!(
                    "'{path:?}' is not a modify attribute, use one of:\n{MODIFY_ATTR_DESCR}",
                );
                return Err(syn::Error::new(input.path.span(), msg));
            }
        };
        Ok(parsed)
    }
    fn parse(attr: &mut syn::Attribute) -> syn::Result<Punctuated<syn::MetaList, Token![,]>> {
        use syn::{Meta::List, MetaList};

        let is_modify = |path: &syn::Path| path.get_ident().map_or(false, |i| i == "modify");
        match &mut attr.meta {
            List(MetaList { path, tokens, .. }) if is_modify(path) => iter_attrs(mem::take(tokens)),
            _ => Ok(Punctuated::new()),
        }
    }

    fn is_write(&self) -> bool {
        use Modify::*;
        match self {
            Write(_) | WriteMut(_) | ReadWrite(_) | DynamicReadWrite(..) => true,
            Read(_) | Context(_) => false,
        }
    }
    fn is_read(&self) -> bool {
        use Modify::*;
        match self {
            Read(_) | ReadWrite(_) | DynamicReadWrite(..) => true,
            Write(_) | WriteMut(_) | Context(_) => false,
        }
    }

    fn ident(&self) -> Option<&Ident> {
        use Modify::*;
        match self {
            Context(ident) | DynamicReadWrite(_, _, ident) => Some(ident),
            WriteMut(path) | Read(path) | ReadWrite(path) => Some(&path.ident),
            Write(_) => None,
        }
    }
    fn requires_identifier(&self) -> bool {
        self.ident().is_some()
    }
    fn has_ident(&self, ident: &Ident) -> bool {
        self.ident() == Some(ident)
    }

    fn field_enum_name(&self) -> Option<&Ident> {
        use Modify::*;
        match self {
            Context(_) | DynamicReadWrite(..) => None,
            WriteMut(path) | Write(path) | Read(path) | ReadWrite(path) => {
                Some(&path.variant_ident)
            }
        }
    }
    fn call_param(&self, ctx: &Ident, item: &Ident) -> Option<TokenStream> {
        use Modify::*;
        match self {
            Context(_) => Some(quote! { & #ctx }),
            DynamicReadWrite(_, _, _) => Some(quote! { #item }),
            WriteMut(path) | ReadWrite(path) => {
                let path = &path.tokens;
                Some(quote! { &mut #item #path })
            }
            Read(path) => {
                let path = &path.tokens;
                Some(quote! { & #item #path })
            }
            Write(_) => None,
        }
    }

    fn as_output(&self, item: &Ident) -> Option<TokenStream> {
        match self {
            Modify::Write(Path { tokens, .. }) => Some(quote! { #item #tokens = }),
            _ => None,
        }
    }

    fn get_dynamic(&self) -> Option<(&Ident, &Ident)> {
        match self {
            Modify::DynamicReadWrite(read, write, _) => Some((read, write)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Modifiers(Vec<Modify>);
impl Modifiers {
    /// The field in the item to write to: `item.path.to.field =`
    ///
    /// Returns empty stream if write modify is passed by reference.
    fn write_field(&self, item: &Ident) -> TokenStream {
        let found = self.0.iter().find_map(|m| m.as_output(item));
        found.unwrap_or(quote!())
    }
    /// The call site: `item.path = fn_name(field1, field2, &item.input1, &mut item.inout)`
    pub fn call<'a>(
        &self,
        fn_name: &Ident,
        ctx: &Ident,
        item: &Ident,
        inputs: impl Iterator<Item = (bool, &'a Ident)>,
    ) -> TokenStream {
        let parameters = inputs.map(|(must_deref, param)| {
            let deref = if must_deref { quote!(*) } else { quote!() };
            match self.0.iter().find(|m| m.has_ident(param)) {
                Some(provided) => provided
                    .call_param(ctx, item)
                    .expect("m.has_ident guarentees call_param always returns some"),
                // From the data structure itself
                None => quote!(#deref #param),
            }
        });
        let write_field = self.write_field(item);
        quote! {
            #write_field #fn_name ( #( #parameters ),* )
        }
    }
    /// Returns `Err` when `function` is invalid.
    ///
    /// It is invalid when:
    /// - Any of the argument is not in the form `foo` or `mut foo`.
    /// - There is a `Modify` with an identifier not present in arguments
    /// - There isn't an output modify attribute (ie: it does nothing)
    /// - There is an invalid attribute
    pub fn validate(&self, function: &ItemFn) -> syn::Result<()> {
        let fn_name = &function.sig.ident;
        macro_rules! bail {
            ($arg:expr) => {
                return Err(syn::Error::new(
                    function.span(),
                    format!("modify function `{fn_name}` {}", $arg),
                ))
            };
        }
        if !self.0.iter().any(|m| m.is_write()) {
            bail!("doesn't have an output, it does nothing!");
        }
        let mut found: Box<[_]> = self.0.iter().map(|m| !m.requires_identifier()).collect();

        for input in &function.sig.inputs {
            let Some(ident) = input.get_ident() else {
                bail!(format!("has a non-identifier input: '{input:?}', not supported"));
            };
            let Some(index) = self.0.iter().position(|m| m.has_ident(ident)) else {
                continue;
            };
            found[index] = true;
        }
        if let Some(index) = found.iter().position(|found| !*found) {
            let modify = &self.0[index];
            let ident = modify
                .ident()
                .expect("we alredy checked this modify requires an identifier");
            bail!(format!(
                "has attribute {modify:?} which isn't present in the argument list \
                add '{ident}' as argument to the function"
            ));
        }
        Ok(())
    }
    /// `true` if `arg` is independent from modify item, it should be state internal
    /// to the modify function.
    ///
    /// # Panics
    ///
    /// If `arg` comes from a function that didn't pass the [`Self::validate`] check.
    pub fn is_constructor_input(&self, arg: &syn::FnArg) -> bool {
        let Some(ident) = arg.get_ident() else { unreachable!() };
        !self.0.iter().any(|m| m.has_ident(ident))
    }

    /// Create self while removing modify attributes from `attrs`.
    pub fn from_attrs(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Self> {
        let mut inner = Vec::new();
        attrs.retain_mut(|attr| {
            let old_len = inner.len();
            inner.push(Modify::parse(attr));
            old_len == inner.len()
        });
        let inner: Vec<_> = inner.into_iter().collect::<syn::Result<_>>()?;
        let inner = inner
            .into_iter()
            .flatten()
            .map(Modify::parse_individual)
            .collect::<syn::Result<_>>()?;
        Ok(Self(inner))
    }
    pub(crate) fn dynamic_fields(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.0
            .iter()
            .filter_map(|m| m.get_dynamic())
            .flat_map(|(read, write)| [read, write])
    }
    pub(crate) fn dynamic_field(&self, rw: Mode) -> Option<&Ident> {
        self.0
            .iter()
            .find_map(|m| m.get_dynamic())
            .map(|m| rw.pick(m))
    }

    pub(crate) fn used_fields(&self, rw: Mode) -> impl Iterator<Item = &Ident> + '_ {
        let rw = rw.is_mode();
        self.0.iter().filter(rw).filter_map(|m| m.field_enum_name())
    }

    pub(crate) fn access_idents(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.0.iter().filter_map(|m| m.field_enum_name())
    }
}
