use std::mem;

use heck::AsUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, spanned::Spanned, ItemFn, Token};

use crate::modifiers::{Mode, Modifiers};

type Tokens = TokenStream;

enum Site {
    Constructor,
    Arm,
}
fn mk_tyname(snake_name: &Ident) -> Ident {
    let ident = AsUpperCamelCase(snake_name.to_string());
    Ident::new(&ident.to_string(), snake_name.span())
}
fn mk_variant_type(ty: Box<syn::Type>) -> Box<syn::Type> {
    use syn::{Type::Reference, TypeReference};
    match *ty {
        Reference(TypeReference { lifetime: None, mutability: None, elem, .. }) => elem,
        any_else => Box::new(any_else),
    }
}
/// `#name { [field: Type],* }` used in `enum ModifyFoo` declaration.
///
/// - Removes inputs appearing in `mods`
/// - Remove prefix `&` that do not have a lifetime from the input type.
/// - Add the `dynamic_field` `read` and `write` fields with type `dynamic_ty`
fn mk_variant(mods: &Modifiers, name: &Ident, inputs: &[syn::FnArg], dynamic_ty: Tokens) -> Tokens {
    // TODO: document variant with path & modify target.
    let fields = inputs
        .iter()
        .filter(|i| mods.is_constructor_input(i))
        .filter_map(|f| match f {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => Some(pat),
        })
        .map(|f| {
            let mut modified = f.clone();
            modified.ty = mk_variant_type(modified.ty);
            modified
        });
    let dynamic_fields = mods.dynamic_fields().map(|f| quote!( #f : #dynamic_ty ));
    quote! {
        #name { #( #fields, )* #( #dynamic_fields ),* }
    }
}
/// `#name { [field],* [..]? }` used in `ModifyFoo` constructors (`Site::Constructor`)
/// or in match arms for `Site::Arm`.
///
/// - Removes inputs appearing in `mods`
/// - Only keep the name of the input.
fn mk_matcher(mods: &Modifiers, name: &Ident, inputs: &[syn::FnArg], site: Site) -> Tokens {
    let mk_pattern_only = |i| match i {
        &syn::FnArg::Receiver(_) => None,
        syn::FnArg::Typed(ty) => Some(&ty.pat),
    };
    let dots = match site {
        Site::Constructor => quote!(),
        Site::Arm => quote!(..),
    };
    let fields = inputs
        .iter()
        .filter(|i| mods.is_constructor_input(i))
        .filter_map(mk_pattern_only);
    quote! {
        #name { #( #fields, )* #dots }
    }
}
/// Convert `function` into a constructor.
///
/// Constructors are functions of the same name as the modify function, used
/// to create each individual variants of the modify enum.
///
/// - Removes inputs appearing in `mods`
/// - Remove prefix `&` that do not have a lifetime from the input type.
/// - Add the `dynamic_field` `read` and `write` fields with type `dynamic_ty`
fn mk_constructor(modifiers: &Modifiers, mut function: ItemFn, body: Box<syn::Block>) -> ItemFn {
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
            let syn::FnArg::Typed(mut input) = input else { continue; };
            input.ty = mk_variant_type(input.ty);

            new_inputs.push(syn::FnArg::Typed(input));
        }
    }
    function.sig.inputs = new_inputs;
    function.block = body;
    function
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
    pub constructor: Option<ItemFn>,
    modifiers: Modifiers,
}
impl ModifyFn {
    pub fn new(mut input: ItemFn) -> syn::Result<Self> {
        let name = input.sig.ident.clone();
        let ty_name = mk_tyname(&name);

        let modifiers = Modifiers::from_attrs(&mut input.attrs)?;
        modifiers.validate(&input)?;

        let inputs: Vec<_> = input.sig.inputs.clone().into_iter().collect();
        let matcher = mk_matcher(&modifiers, &ty_name, &inputs, Site::Constructor);
        let block = quote!({ Self :: #matcher });

        let constructor = if modifiers.dynamic_field(Mode::Read).is_none() {
            let block = syn::parse2(block)?;
            Some(mk_constructor(&modifiers, input.clone(), Box::new(block)))
        } else {
            None
        };

        let mut declaration = input;
        mk_declaration(&mut declaration);

        Ok(ModifyFn { name, inputs, declaration, constructor, modifiers })
    }
    pub fn call(&self, ctx: &Ident, item: &Ident) -> Tokens {
        let name = &self.name;
        let arguments = self.declaration.sig.inputs.iter().filter_map(|i| match i {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(ident) => {
                    let must_deref = !matches!(&*pat.ty, &syn::Type::Reference(_));
                    Some((must_deref, &ident.ident))
                }
                _ => None,
            },
        });
        self.modifiers.call(name, ctx, item, arguments)
    }
    fn ty_name(&self) -> Ident {
        mk_tyname(&self.name)
    }
    pub fn ty_variant(&self, root: &Ident, field_ty_name: &Ident) -> Tokens {
        let field_ty = quote!(::#root::EnumSet<#field_ty_name>);
        mk_variant(&self.modifiers, &self.ty_name(), &self.inputs, field_ty)
    }
    pub fn ty_matcher(&self) -> Tokens {
        mk_matcher(&self.modifiers, &self.ty_name(), &self.inputs, Site::Arm)
    }
    fn arm(&self, root: &Ident, field_ty_name: &Ident, mode: Mode) -> Tokens {
        let ty_name = self.ty_name();
        let ty = quote!(Self::#ty_name);
        let fields = self.modifiers.used_fields(mode);
        match self.modifiers.dynamic_field(mode) {
            Some(dynamic) => quote! { #ty { #dynamic , .. } => *#dynamic },
            None => quote! { #ty { .. } => ::#root::EnumSet::EMPTY #(| #field_ty_name::#fields)* },
        }
    }
    pub fn depends_arm(&self, root: &Ident, field_ty_name: &Ident) -> Tokens {
        self.arm(root, field_ty_name, Mode::Read)
    }
    pub fn changes_arm(&self, root: &Ident, field_ty_name: &Ident) -> Tokens {
        self.arm(root, field_ty_name, Mode::Write)
    }
    pub fn access_idents(&self) -> impl Iterator<Item = Ident> + '_ {
        self.modifiers.access_idents().cloned()
    }
}
