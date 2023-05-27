use std::{collections::BTreeSet, mem};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{meta::ParseNestedMeta, spanned::Spanned, Visibility};

use crate::{
    extensions::{GetIdentExt, IntoSynErrorsExt},
    modify_fn::ModifyFn,
};

fn modify_fn_or_add_context(
    item: syn::ImplItem,
    ctx: &mut Option<(syn::Type, syn::Generics)>,
) -> syn::Result<Option<ModifyFn>> {
    use syn::ImplItem::{Fn, Type};
    let msg = "modify_impl accepts only `type Context` as associated type. \
        Other items in the #[modify_impl] block MUST be functions, \
        the modify functions.";
    match item {
        Type(maybe_context) => {
            if maybe_context.ident != "Context" {
                return Err(syn::Error::new(maybe_context.span(), msg));
            }
            *ctx = Some((maybe_context.ty, maybe_context.generics));
            Ok(None)
        }
        Fn(fn_item) => {
            let fn_item = syn::ItemFn {
                attrs: fn_item.attrs,
                vis: fn_item.vis,
                sig: fn_item.sig,
                block: Box::new(fn_item.block),
            };
            ModifyFn::new(fn_item).map(Some)
        }
        item => Err(syn::Error::new(item.span(), msg)),
    }
}

/// Get `Something` in `impl Modify<Something> for ModifyFoo`.
fn get_target(item: &syn::ItemImpl) -> syn::Result<&Ident> {
    use syn::{GenericArgument::Type, Path};
    let msg_missing = "#[impl_modify] block must be: `impl Modify<Something> for ModifySomething";
    let err_inner = |span| syn::Error::new(span, msg_missing);
    let err = |span| Err(err_inner(span));
    match &item.trait_ {
        Some((_, Path { segments, .. }, _)) => {
            let len = segments.len();
            if len != 1 {
                return err(segments.span());
            }
            let last = segments.last().expect("already check len > 0");
            if last.ident != "Modify" {
                return err(last.span());
            }
            match &last.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    let args = &args.args;
                    let len = args.len();
                    if len != 1 {
                        return err(args.span());
                    }
                    match args.last() {
                        Some(Type(ty)) => ty.get_ident().ok_or_else(|| err_inner(ty.span())),
                        kind => err(kind.span()),
                    }
                }
                args => err(args.span()),
            }
        }
        None => err(item.span()),
    }
}

pub(crate) struct Config {
    fab_path: syn::Path,
    enumset_crate: Ident,
    visibility: syn::Visibility,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            fab_path: syn::parse_quote!(::cuicui_fab),
            enumset_crate: Ident::new("enumset", Span::call_site()),
            visibility: Visibility::Public(syn::token::Pub { span: Span::call_site() }),
        }
    }
}
const CONFIG_ATTR_DESCR: &str = "\
- `cuicui_fab_path = alternate::path`: specify which path to use for the `cuicui_fab` crate
  by default, it is `::cuicui_fab`
- `enumset_crate = identifier`: specify which path to use for the `enumset` crate
  by default, it is `enumset`
- `visibility = [pub(crate)]`: specify the visibility for the generated enums.
  by default, it is `pub`\n";
impl Config {
    pub(crate) fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        match () {
            () if meta.path.is_ident("cuicui_fab_path") => {
                let value = meta.value()?;
                self.fab_path = value.parse()?;
            }
            () if meta.path.is_ident("enumset_crate") => {
                let value = meta.value()?;
                self.enumset_crate = value.parse()?;
            }
            () if meta.path.is_ident("visibility") => {
                let value = meta.value()?;
                self.visibility = value.parse()?;
            }
            () => {
                let msg = format!("Unrecognized impl_modify meta attribute\n{CONFIG_ATTR_DESCR}");
                return Err(meta.error(msg));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Block {
    attributes: Vec<syn::Attribute>,
    impl_target: Ident,
    context: syn::Type,
    context_generics: syn::Generics,
    functions: Vec<ModifyFn>,
    fields: BTreeSet<Ident>,
    // TODO(feat): allow generics
    modify_ty: Ident,
    fab_path: syn::Path,
    enumset_ident: Ident,
    visibility: Visibility,
}
impl Block {
    pub fn parse(config: Config, mut input: syn::ItemImpl) -> syn::Result<Self> {
        let mut ctx = None;
        let mut errors = Vec::new();
        let read_fn = |item| match modify_fn_or_add_context(item, &mut ctx) {
            Err(err) => {
                errors.push(err);
                None
            }
            Ok(valid) => valid,
        };
        let msg = "Modify derive with generic parameter not supported yet";
        let err = syn::Error::new(input.span(), msg);
        let modify_ty = input.self_ty.get_ident().ok_or(err)?.clone();
        let impl_target = get_target(&input)?.clone();
        let attributes = mem::take(&mut input.attrs);
        let functions: Vec<_> = input.items.drain(..).filter_map(read_fn).collect();
        let fields = functions.iter().flat_map(ModifyFn::access_idents).collect();

        if let Some(error) = errors.into_syn_errors() {
            return Err(error);
        }
        let Some((context, context_generics)) = ctx else {
            let msg = "modify_impl MUST declare a `type Context` associated type. \
                If you are not using it, use `type Context = ();`";
            return Err(syn::Error::new(input.span(), msg));
        };
        Ok(Block {
            attributes,
            impl_target,
            context,
            context_generics,
            functions,
            fields,
            modify_ty,
            fab_path: config.fab_path,
            enumset_ident: config.enumset_crate,
            visibility: config.visibility,
        })
    }
    pub fn generate_impl(self) -> TokenStream {
        let enset = &self.enumset_ident;
        let enset_string = syn::LitStr::new(&enset.to_string(), enset.span());
        let fab = &self.fab_path;
        let vis = &self.visibility;

        let context = &self.context;
        let context_generics = &self.context_generics;
        let ctx = Ident::new("ctx", Span::call_site());
        let item = Ident::new("item", Span::call_site());

        let modify_ty = &self.modify_ty;
        let field_ty = format_ident!("{modify_ty}Field");
        let field_variants = &self.fields;
        let impl_target = &self.impl_target;

        let fns = || self.functions.iter();
        let ty_variants = fns().map(|f| f.ty_variant(enset, &field_ty));
        let ty_matcher = fns().map(ModifyFn::ty_matcher);
        let changes_arms = fns().map(|f| f.changes_arm(enset, &field_ty));
        let depends_arms = fns().map(|f| f.depends_arm(enset, &field_ty));
        let ty_constructors = fns().map(|m| &m.constructor);
        let ty_function_defs = fns().map(|m| &m.declaration);
        let ty_function_calls = fns().map(|m| m.call(&ctx, &item));

        let ty_attributes = &self.attributes;
        quote! {
            /// Fields accessed by [`
            #[doc = stringify!(#modify_ty)]
            /// `].
            #[derive( ::#enset::EnumSetType, ::std::fmt::Debug )]
            #[enumset(crate_name = #enset_string)]
            #vis enum #field_ty {
                #( #field_variants ),*
            }
            #( #ty_attributes )*
            #vis enum #modify_ty {
                #( #ty_variants ),*
            }
            impl #modify_ty {
                #( #ty_constructors )*
            }
            #[allow(clippy::ptr_arg)]
            impl Modify<#impl_target> for #modify_ty {
                type Field = #field_ty;
                type Context #context_generics = #context;

                fn apply(
                    &self,
                    #ctx: &Self::Context<'_>,
                    #item: &mut #impl_target,
                ) -> #fab::__private::anyhow::Result<()> {
                    match self {
                        #(
                            Self::#ty_matcher => {
                                #ty_function_defs
                                #ty_function_calls;
                            }
                        ),*
                    }
                    Ok(())
                }

                fn depends(&self) -> ::#enset::EnumSet<Self::Field> {
                    match self {
                        #( #depends_arms ),*
                    }
                }

                fn changes(&self) -> ::#enset::EnumSet<Self::Field> {
                    match self {
                        #( #changes_arms ),*
                    }
                }
            }
        }
    }
}
