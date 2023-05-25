use std::mem;

use heck::AsUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, spanned::Spanned, ItemFn, Token};

use crate::modifiers::Modifiers;

type MyArm = TokenStream;
type MyVariant = TokenStream;
type MyMatcher = TokenStream;
type MyExprCall = TokenStream;

fn mk_tyname(snake_name: &Ident) -> Ident {
    let ident = AsUpperCamelCase(snake_name.to_string());
    Ident::new(&ident.to_string(), snake_name.span())
}
fn mk_variant(mods: &Modifiers, name: &Ident, inputs: &[syn::FnArg]) -> MyVariant {
    // TODO: remove reference from input type.
    // TODO: document variant with path & modify target.
    let fields = inputs.iter().filter(|i| mods.is_constructor_input(i));
    quote! {
        #name { #( #fields ),* }
    }
}
fn mk_matcher(mods: &Modifiers, name: &Ident, inputs: &[syn::FnArg]) -> MyMatcher {
    let mk_pattern_only = |i| match i {
        &syn::FnArg::Receiver(_) => None,
        syn::FnArg::Typed(ty) => Some(&ty.pat),
    };
    // TODO: remove reference from input type.
    let fields = inputs
        .iter()
        .filter(|i| mods.is_constructor_input(i))
        .filter_map(mk_pattern_only);
    quote! {
        #name { #( #fields ),* }
    }
}
fn mk_constructor(modifiers: &Modifiers, function: &mut ItemFn, body: Box<syn::Block>) {
    let span = function.sig.span();

    // Constructor always return self
    let path = syn::TypePath { qself: None, path: format_ident!("Self").into() };
    let ty = Box::new(syn::Type::Path(path));
    function.sig.output = syn::ReturnType::Type(Token![->](span), ty);

    // Constructor is `const` and public
    function.vis = syn::Visibility::Public(Token![pub](span));
    function.sig.constness = Some(Token![const](span));

    // Remove modify-dependent arguments
    let mut new_inputs = Punctuated::new();
    for input in mem::take(&mut function.sig.inputs).into_iter() {
        if modifiers.is_constructor_input(&input) {
            new_inputs.push(input);
        }
    }
    function.sig.inputs = new_inputs;
    function.block = body;
}
/// Declaration of modifier to insert into the match arms of the `apply` `Modify` method.
fn mk_declaration(function: &mut ItemFn) {
    function.attrs.clear();
}

#[derive(Debug)]
pub(crate) struct ModifyFn {
    name: Ident,
    inputs: Vec<syn::FnArg>,
    pub declaration: ItemFn,
    pub constructor: ItemFn,
    modifiers: Modifiers,
}
impl ModifyFn {
    pub fn new(mut input: ItemFn) -> syn::Result<Self> {
        let name = input.sig.ident.clone();
        let ty_name = mk_tyname(&name);

        let modifiers = Modifiers::from_attrs(&mut input.attrs)?;
        modifiers.validate(&input)?;

        let inputs: Vec<_> = input.sig.inputs.clone().into_iter().collect();
        let matcher = mk_matcher(&modifiers, &ty_name, &inputs);
        let block = quote!({ Self :: #matcher });

        let mut constructor = input.clone();
        mk_constructor(&modifiers, &mut constructor, Box::new(syn::parse2(block)?));

        let mut declaration = input;
        mk_declaration(&mut declaration);

        Ok(ModifyFn { name, inputs, declaration, constructor, modifiers })
    }
    pub fn call(&self, ctx: &Ident, item: &Ident) -> MyExprCall {
        let name = &self.name;
        let arguments = self.declaration.sig.inputs.iter().filter_map(|i| match i {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(ident) => Some(&ident.ident),
                _ => None,
            },
        });
        self.modifiers.call(name, ctx, item, arguments)
    }
    fn ty_name(&self) -> Ident {
        mk_tyname(&self.name)
    }
    pub fn ty_variant(&self) -> MyVariant {
        mk_variant(&self.modifiers, &self.ty_name(), &self.inputs)
    }
    pub fn ty_matcher(&self) -> MyVariant {
        mk_matcher(&self.modifiers, &self.ty_name(), &self.inputs)
    }
    pub fn depends_arm(&self, root: &Ident, field_ty_name: &Ident) -> MyArm {
        // TODO: dynamic_read_write
        let ty_name = self.ty_name();
        let reads = self.modifiers.reads();
        quote! {
            Self::#ty_name { .. } => #root ::EnumSet::EMPTY
                #( | #field_ty_name :: #reads )*
        }
    }
    pub fn changes_arm(&self, root: &Ident, field_ty_name: &Ident) -> MyArm {
        // TODO: dynamic_read_write
        let ty_name = self.ty_name();
        let writes = self.modifiers.writes();
        quote! {
            Self::#ty_name { .. } =>  #root ::EnumSet::EMPTY
                #( | #field_ty_name :: #writes )*
        }
    }
    pub fn access_idents(&self) -> impl Iterator<Item = Ident> + '_ {
        self.modifiers.access_idents()
    }
}
