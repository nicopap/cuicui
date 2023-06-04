mod deps;
mod path;

use std::fmt;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{meta::ParseNestedMeta, parenthesized, parse::Parse, spanned::Spanned, ItemFn, Token};

use crate::extensions::{GetIdentExt, IntoSynErrorsExt};
use path::Path;

pub use deps::{AtomicAccessors, FnAtomicAccessors};

use self::path::Source;

#[derive(Clone, Copy)]
pub enum Mode {
    Read,
    Write,
}
#[derive(Debug)]
struct DynamicReadWrite {
    read: Ident,
    write: Ident,
    param_name: Ident,
}
impl Parse for DynamicReadWrite {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let read = input.parse()?;
        let _ = input.parse::<Token![,]>()?;
        let write = input.parse()?;
        let param_name = if input.parse::<Token![,]>().is_ok() {
            input.parse()?
        } else {
            Ident::new("item", input.span())
        };
        Ok(DynamicReadWrite { read, write, param_name })
    }
}

impl DynamicReadWrite {
    fn field_ident(&self, mode: Mode) -> &Ident {
        use Mode::*;
        match mode {
            Read => &self.read,
            Write => &self.write,
        }
    }
}

const MODIFY_ATTR_DESCR: &str = "\
- `context([ident =] .path.in.context)`: The context declared in `type Context = Foo;`
- `write(.path.in.item)`: path in item to write return value
- `write_mut([ident =] .path.in.item)`: write-only path in item to pass as `&mut ident`
- `read([ident =] .path.in.item)`: read-only path in item to pass as `&ident`
- `read_write([ident =] .path.in.item)`: read/write path in item to pass as `&mut ident`
- `dynamic_read_write(read_ident, write_ident [, ident])`: pass `&mut item` and read
  those fields for checking which paths in item are read from and writen to.
  The thirs optional parameter is which function argument to pass it to
  (by default it is `item`)
";
#[derive(Debug, Clone, Copy)]
enum ModifyType {
    Context,
    Write,
    WriteMut,
    Read,
    ReadWrite,
}
struct Modify {
    ty: ModifyType,
    path: Path,
}
impl fmt::Display for Modify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ModifyType::*;

        let ty = match self.ty {
            Context => "context",
            Write => "write",
            WriteMut => "write_mut",
            Read => "read",
            ReadWrite => "read_write",
        };
        write!(f, "{ty}({})", self.path)
    }
}
impl Modify {
    fn is_write(&self) -> bool {
        use ModifyType::*;
        matches!(self.ty, Write | WriteMut | ReadWrite)
    }
    fn is_read(&self) -> bool {
        use ModifyType::*;
        matches!(self.ty, Read | ReadWrite | Context)
    }

    fn ident(&self) -> &Ident {
        &self.path.ident
    }
    fn requires_identifier(&self) -> bool {
        !matches!(self.ty, ModifyType::Write)
    }
    fn has_ident(&self, ident: &Ident) -> bool {
        self.ident() == ident
    }

    fn call_param(&self, ctx: &Ident, item: &Ident) -> Option<TokenStream> {
        use ModifyType::*;

        match self.ty {
            Context => Some(quote! { & #ctx }),
            WriteMut | ReadWrite => {
                let path = self.path.to_tokens();
                Some(quote! { &mut #item #path })
            }
            Read => {
                let path = self.path.to_tokens();
                Some(quote! { & #item #path })
            }
            Write => None,
        }
    }

    fn as_output(&self, item: &Ident) -> Option<TokenStream> {
        use ModifyType::Write;

        if let Write = self.ty {
            let tokens = self.path.to_tokens();
            Some(quote! { #item #tokens = })
        } else {
            None
        }
    }
}

pub struct Modifiers {
    mods: Vec<Modify>,
    dynamic: Option<DynamicReadWrite>,
}
impl Modifiers {
    pub fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        let non_dynamic_idents = [
            ("context", (ModifyType::Context, Source::Context)),
            ("write", (ModifyType::Write, Source::Item)),
            ("write_mut", (ModifyType::WriteMut, Source::Item)),
            ("read", (ModifyType::Read, Source::Item)),
            ("read_write", (ModifyType::ReadWrite, Source::Item)),
        ];
        let meta_type = non_dynamic_idents
            .iter()
            .find_map(|(name, value)| meta.path.is_ident(name).then_some(*value));
        match meta_type {
            Some((ty, source)) => {
                let path;
                parenthesized!(path in meta.input);
                self.mods
                    .push(Modify { ty, path: source.parse_path(&path)? });
            }
            None if meta.path.is_ident("dynamic_read_write") => {
                let dynamic;
                parenthesized!(dynamic in meta.input);
                self.dynamic = Some(dynamic.parse()?);
            }
            None => {
                return Err(syn::Error::new(meta.input.span(), MODIFY_ATTR_DESCR));
            }
        }
        Ok(())
    }
    /// The field in the item to write to: `item.path.to.field =`
    ///
    /// Returns empty stream if write modify is passed by reference.
    fn write_field(&self, item: &Ident) -> TokenStream {
        let found = self.mods.iter().find_map(|m| m.as_output(item));
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
            match self.mods.iter().find(|m| m.has_ident(param)) {
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
        let has_dynamic = self.dynamic.is_some();

        macro_rules! bail {
            ($arg:expr) => {
                return Err(syn::Error::new(
                    function.span(),
                    format!("modify function `{fn_name}` {}", $arg),
                ))
            };
        }
        if !has_dynamic && !self.mods.iter().any(|m| m.is_write()) {
            bail!("doesn't have an output, it does nothing!");
        }
        let mut found: Box<[_]> = self.mods.iter().map(|m| !m.requires_identifier()).collect();

        for input in &function.sig.inputs {
            let Some(ident) = input.get_ident() else {
                bail!(format!("has a non-identifier input: '{input:?}', not supported"));
            };
            let Some(index) = self.mods.iter().position(|m| m.has_ident(ident)) else {
                continue;
            };
            found[index] = true;
        }
        if let Some(index) = found.iter().position(|found| !*found) {
            let modify = &self.mods[index];
            let ident = &modify.path.ident;
            bail!(format!(
                "has attribute {modify} which isn't present in the argument list. \
                Add '{ident}' as argument to the function"
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
        let Some(ident) = arg.get_ident() else { panic!("this function wasn't validated") };
        let dynamic_ident = self.dynamic.as_ref().map(|d| &d.param_name);
        let dynamic = Some(ident) == dynamic_ident;
        !dynamic && !self.mods.iter().any(|m| m.has_ident(ident))
    }

    /// Create self while removing modify attributes from `attrs`.
    pub fn from_attrs(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Self> {
        let mut ret = Self { mods: Vec::new(), dynamic: None };
        let mut errs = Vec::new();

        attrs.retain_mut(|attr| {
            let is_modify = attr.meta.path().is_ident("modify");
            if is_modify {
                if let Err(err) = attr.parse_nested_meta(|m| ret.parse(m)) {
                    errs.push(err);
                }
            }
            !is_modify
        });
        if !errs.is_empty() {
            return Err(errs.into_syn_errors().unwrap());
        }
        Ok(ret)
    }
    pub fn dynamic_fields(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.dynamic.iter().flat_map(|d| [&d.read, &d.write])
    }
    pub fn dynamic_field(&self, rw: Mode) -> Option<&Ident> {
        self.dynamic.as_ref().map(|d| d.field_ident(rw))
    }

    pub fn non_atomic_paths(&self) -> impl Iterator<Item = &Path> + '_ {
        self.mods.iter().map(|m| &m.path)
    }
}
