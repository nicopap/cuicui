use std::{collections::BTreeSet, mem};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{meta::ParseNestedMeta, spanned::Spanned, Visibility};

use crate::{
    extensions::{GetIdentExt, IntoSynErrorsExt},
    modify_fn::ModifyFn,
};

const BAD_ASSOC_TYPE: &str = "Modify as a trait requires the following \
    associated types: \n\
    - `type Context<'a>`: The immutable context external to `Item` passed to \
      to the modify through the `#[modify(ctx)]` attribute\n\
    - `type Item`: The item on which Modify operates.\n\
    - `type Items`: The collection of items passed to `Resolve` to run the \
      modifiers specifed in this macro on.";

/// Store encountered associated types used in the `Modify` definition.
#[derive(Default)]
struct AssociatedTypes {
    /// `Modify::Context`, type being the right side of `=` and geneirc the
    /// context's lifetime parameter.
    context: Option<(syn::Type, syn::Generics)>,

    /// `Modify::Item`.
    item: Option<syn::Type>,

    /// `Modify::Items`.
    items: Option<syn::Type>,
}
fn read_item(item: syn::ImplItem, assoc: &mut AssociatedTypes) -> syn::Result<Option<ModifyFn>> {
    use syn::ImplItem::{Fn, Type};
    match item {
        Type(assoc_type) if assoc_type.ident == "Context" => {
            assoc.context = Some((assoc_type.ty, assoc_type.generics));
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "Item" => {
            assoc.item = Some(assoc_type.ty);
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "Items" => {
            assoc.items = Some(assoc_type.ty);
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
        item => Err(syn::Error::new(item.span(), BAD_ASSOC_TYPE)),
    }
}

pub(crate) struct Config {
    fab_path: syn::Path,
    enumset_crate: Ident,
    visibility: syn::Visibility,
    no_debug_derive: bool,
    no_clone_derive: bool,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            fab_path: syn::parse_quote!(::cuicui_fab),
            enumset_crate: Ident::new("enumset", Span::call_site()),
            visibility: Visibility::Public(syn::token::Pub { span: Span::call_site() }),
            no_debug_derive: false,
            no_clone_derive: false,
        }
    }
}
const CONFIG_ATTR_DESCR: &str = "\
- `cuicui_fab_path = alternate::path`: specify which path to use for the `cuicui_fab` crate
  by default, it is `::cuicui_fab`
- `enumset_crate = identifier`: specify which path to use for the `enumset` crate
  by default, it is `enumset`
- `no_derive(Debug | Clone)`: Do not automatically implement given trait for Modifier.
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
            () if meta.path.is_ident("no_derive") => meta.parse_nested_meta(|meta| {
                if meta.path.is_ident("Debug") {
                    self.no_debug_derive = true;
                }
                if meta.path.is_ident("Clone") {
                    self.no_debug_derive = true;
                }
                Ok(())
            })?,
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
    item: syn::Type,
    items: syn::Type,
    context: syn::Type,
    context_generics: syn::Generics,
    functions: Vec<ModifyFn>,
    fields: BTreeSet<Ident>,
    // TODO(feat): allow generics
    modify_ty: Ident,
    fab_path: syn::Path,
    enumset_ident: Ident,
    visibility: Visibility,
    no_debug_derive: bool,
    no_clone_derive: bool,
}
impl Block {
    pub fn parse(config: Config, mut input: syn::ItemImpl) -> syn::Result<Self> {
        let mut assocs = AssociatedTypes::default();
        let mut errors = Vec::new();
        let read_fn = |item| match read_item(item, &mut assocs) {
            Err(err) => {
                errors.push(err);
                None
            }
            Ok(valid) => valid,
        };
        let msg = "Modify derive with generic parameter not supported yet";
        let err = syn::Error::new(input.span(), msg);
        let modify_ty = input.self_ty.get_ident().ok_or(err)?.clone();
        let attributes = mem::take(&mut input.attrs);
        let functions: Vec<_> = input.items.drain(..).filter_map(read_fn).collect();
        let fields = functions.iter().flat_map(ModifyFn::access_idents).collect();

        if let Some(error) = errors.into_syn_errors() {
            return Err(error);
        }
        let Some((context, context_generics)) = assocs.context else {
            let msg = "modify_impl MUST declare a `type Context` associated type. \
                If you are not using it, use `type Context = ();`";
            return Err(syn::Error::new(input.span(), msg));
        };
        let Some(item) = assocs.item else {
            let msg = "modify_impl MUST declare a `type Item` associated type.";
            return Err(syn::Error::new(input.span(), msg));
        };
        let Some(items) = assocs.items else {
            let msg = "modify_impl MUST declare a `type Items` associated type.";
            return Err(syn::Error::new(input.span(), msg));
        };
        Ok(Block {
            attributes,
            item,
            items,
            context,
            context_generics,
            functions,
            fields,
            modify_ty,
            fab_path: config.fab_path,
            enumset_ident: config.enumset_crate,
            visibility: config.visibility,
            no_debug_derive: config.no_debug_derive,
            no_clone_derive: config.no_clone_derive,
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
        let item_param = Ident::new("item", Span::call_site());

        let modify_ty = &self.modify_ty;
        let field_ty = format_ident!("{modify_ty}Field");
        let field_variants = &self.fields;
        let item = &self.item;
        let items = &self.items;

        let fns = || self.functions.iter();
        let ty_variants = fns().map(|f| f.ty_variant(enset, &field_ty));
        let ty_matcher = fns().map(ModifyFn::ty_matcher);
        let changes_arms = fns().map(|f| f.changes_arm(enset, &field_ty));
        let depends_arms = fns().map(|f| f.depends_arm(enset, &field_ty));
        let field_assoc_fns = fns().map(|f| f.fields_assoc_fns(enset, &field_ty));
        let ty_constructors = fns().map(|m| &m.constructor);
        let ty_function_defs = fns().map(|m| &m.declaration);
        let ty_function_calls = fns().map(|m| m.call(&ctx, &item_param));
        let debug_derive = if self.no_debug_derive {
            quote!()
        } else {
            quote!(#[derive( ::std::fmt::Debug )])
        };
        let clone_derive = if self.no_clone_derive {
            quote!()
        } else {
            quote!(#[derive( ::std::clone::Clone )])
        };

        let ty_attributes = &self.attributes;
        quote! {
            #[doc = concat!("Fields accessed by [`", stringify!(#modify_ty), "`].")]
            #[doc = "\n\n"]
            #[doc = concat!(
                "Fields may be members of [`",
                stringify!(#item),
                "`], the Item of sections modified by [`",
                stringify!(#modify_ty),
                "`], or fields of the context [`",
                stringify!(#modify_ty),
                "::Context`].\n",
            )]
            #[derive( ::#enset::EnumSetType, ::std::fmt::Debug )]
            #[enumset(crate_name = #enset_string)]
            #vis enum #field_ty {
                #( #field_variants ),*
            }
            #( #ty_attributes )*
            #debug_derive
            #clone_derive
            #vis enum #modify_ty {
                #( #ty_variants ),*
            }
            /// Functions returning which field each modify function changes
            /// and depends on.
            ///
            /// Note that if the modify function in question doesn't depend on
            /// anything, no function is provided.
            impl #modify_ty {
                #(
                    #field_assoc_fns
                )*
            }
            /// Constructors for each individual modify variant.
            impl #modify_ty {
                #(
                    #ty_constructors
                )*
            }
            #[allow(clippy::ptr_arg)]
            impl Modify for #modify_ty {
                type Field = #field_ty;
                type Context #context_generics = #context;
                type Item = #item;
                type Items = #items;

                fn apply(
                    &self,
                    #ctx: &Self::Context<'_>,
                    #item_param: &mut #item,
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

                #[inline]
                fn depends(&self) -> ::#enset::EnumSet<Self::Field> {
                    match self {
                        #( #depends_arms ),*
                    }
                }

                #[inline]
                fn changes(&self) -> ::#enset::EnumSet<Self::Field> {
                    match self {
                        #( #changes_arms ),*
                    }
                }
            }
        }
    }
}
