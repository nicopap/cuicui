use std::mem;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{meta::ParseNestedMeta, spanned::Spanned, Visibility};

use crate::{
    extensions::{GetIdentExt, IntoSynErrorsExt},
    modifiers::AtomicAccessors,
    modify_fn::ModifyFn,
};

const BAD_ASSOC_TYPE: &str = "Modify as a trait requires the following \
    associated types: \n\
    - `type Context<'a>`: The immutable context external to `Item` passed to \
      to the modify through the `#[modify(ctx)]` attribute\n\
    - `type Item`: The item on which Modify operates.\n\
    - `type Items`: The collection of items passed to `Resolve` to run the \
      modifiers specifed in this macro on.\n\
    \n\
    It also optionally supports:\n\
    - `type Resolver`: The resolver to use for this Modify. By default, it is \
    the `DepsResolver` with modify change dependency detection. You may chose \
    `MinimalResolver` instead. It doesn't have change dependency detection, but \
    it is much faster to build and run.";

struct Generic {
    ty: syn::Type,
    gens: syn::Generics,
}
impl From<syn::ImplItemType> for Generic {
    fn from(value: syn::ImplItemType) -> Self {
        Generic { ty: value.ty, gens: value.generics }
    }
}
/// Store encountered associated types used in the `Modify` definition.
#[derive(Default)]
struct AssociatedTypes {
    /// `Modify::Context`, type being the right side of `=` and geneirc the
    /// context's lifetime parameter.
    context: Option<Generic>,

    /// `Modify::Item`.
    item: Option<Generic>,

    /// `Modify::Items`.
    items: Option<Generic>,

    /// the resolver to use for this `Modify`
    resolver: Option<syn::Type>,

    /// The `Modify::MakeItem`. By default, same as `item`.
    make_item: Option<syn::Type>,
}
fn read_item(item: syn::ImplItem, assoc: &mut AssociatedTypes) -> syn::Result<Option<ModifyFn>> {
    use syn::ImplItem::{Fn, Type};
    match item {
        Type(assoc_type) if assoc_type.ident == "Context" => {
            assoc.context = Some(assoc_type.into());
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "Item" => {
            assoc.item = Some(assoc_type.into());
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "Items" => {
            assoc.items = Some(assoc_type.into());
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "Resolver" => {
            assoc.resolver = Some(assoc_type.ty);
            Ok(None)
        }
        Type(assoc_type) if assoc_type.ident == "MakeItem" => {
            assoc.make_item = Some(assoc_type.ty);
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
- `cuicui_fab_path = alternate::path`: specify which path to use for the \
  `cuicui_fab` crate by default, it is `::cuicui_fab`
- `enumset_crate = identifier`: specify which path to use for the `enumset` \
  crate by default, it is `enumset`
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

pub(crate) struct Block {
    attributes: Vec<syn::Attribute>,
    item: Generic,
    items: Generic,
    context: Generic,
    resolver: Option<syn::Type>,
    make_item: Option<syn::Type>,
    functions: Vec<ModifyFn>,
    field_accessors: AtomicAccessors,
    // TODO(feat): allow generics
    modify_ty: Ident,
    fab_path: syn::Path,
    enumset_ident: Ident,
    visibility: Visibility,
    debug_derive: bool,
    clone_derive: bool,
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
        let mut functions: Vec<_> = input.items.drain(..).filter_map(read_fn).collect();
        let field_accessors = AtomicAccessors::from_non_atomic(
            functions
                .iter()
                .flat_map(|f| f.modifiers.non_atomic_paths()),
        );
        functions
            .iter_mut()
            .for_each(|f| f.atomize_accessors(&field_accessors));

        if let Some(error) = errors.into_syn_errors() {
            return Err(error);
        }
        let Some(context) = assocs.context else {
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
            resolver: assocs.resolver,
            make_item: assocs.make_item,
            functions,
            field_accessors,
            modify_ty,
            fab_path: config.fab_path,
            enumset_ident: config.enumset_crate,
            visibility: config.visibility,
            debug_derive: !config.no_debug_derive,
            clone_derive: !config.no_clone_derive,
        })
    }
    pub fn generate_impl(self) -> TokenStream {
        let Self {
            attributes,
            item: Generic { ty: item_ty, gens: item_gens },
            items: Generic { ty: items_ty, gens: items_gens },
            context: Generic { ty: context_ty, gens: context_gens },
            resolver,
            field_accessors,
            modify_ty,
            fab_path,
            enumset_ident,
            visibility,
            debug_derive,
            clone_derive,
            make_item,
            ..
        } = &self;

        let enset_string = syn::LitStr::new(&enumset_ident.to_string(), enumset_ident.span());

        let ctx = Ident::new("ctx", Span::call_site());
        let item_param = Ident::new("item", Span::call_site());

        let field_accessors = field_accessors.all_variants();
        let field_ty = format_ident!("{modify_ty}Field");

        let fns = || self.functions.iter();
        let ty_variants = fns().map(|f| f.ty_variant(enumset_ident, &field_ty));
        let ty_matcher = fns().map(ModifyFn::ty_matcher);
        let changes_arms = fns().map(|f| f.changes_arm(enumset_ident, &field_ty));
        let depends_arms = fns().map(|f| f.depends_arm(enumset_ident, &field_ty));
        let field_assoc_fns = fns().map(|f| f.fields_assoc_fns(enumset_ident, &field_ty));
        let ty_constructors = fns().map(|m| &m.constructor);
        let ty_function_defs = fns().map(|m| &m.declaration);
        let ty_function_calls = fns().map(|m| m.call(&ctx, &item_param));
        let debug_derive = debug_derive.then(|| quote!(#[derive( ::std::fmt::Debug )]));
        let clone_derive = clone_derive.then(|| quote!(#[derive( ::std::clone::Clone )]));
        let resolver = resolver.as_ref().map_or_else(
            || {
                let enumset_private = quote!(::#enumset_ident::__internal::EnumSetTypePrivate);
                let bit_width = quote!((<Self::Field as #enumset_private>::BIT_WIDTH - 1) as usize);
                quote!(#fab_path::resolve::DepsResolver::<Self, {#bit_width} >)
            },
            |ty| quote!(#ty),
        );
        let make_item = make_item
            .as_ref()
            .map_or_else(|| quote!(#item_ty), |ty| quote!(#ty));

        quote! {
            #[doc = concat!("Fields accessed by [`", stringify!(#modify_ty), "`].")]
            #[doc = "\n\n"]
            #[doc = concat!(
                "Fields may be members of [`",
                stringify!(#item_ty),
                "`], the Item of sections modified by [`",
                stringify!(#modify_ty),
                "`], or fields of the context [`",
                stringify!(#modify_ty),
                "::Context`].\n",
            )]
            #[derive( ::#enumset_ident::EnumSetType, ::std::fmt::Debug )]
            #[enumset(crate_name = #enset_string)]
            #visibility enum #field_ty {
                #( #field_accessors ),*
            }
            #( #attributes )*
            #debug_derive
            #clone_derive
            #visibility enum #modify_ty {
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
                type Context #context_gens = #context_ty;
                // TODO: add &'a mut if Item not declared with <'a>
                type Item #item_gens = #item_ty;
                type Items #items_gens = #items_ty;
                type MakeItem = #make_item;
                type Resolver = #resolver;

                fn apply #item_gens(
                    &self,
                    #ctx: &Self::Context<'_>,
                    #item_param: #item_ty,
                ) -> #fab_path::__private::anyhow::Result<()> {
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
                fn depends(&self) -> ::#enumset_ident::EnumSet<Self::Field> {
                    match self {
                        #( #depends_arms ),*
                    }
                }

                #[inline]
                fn changes(&self) -> ::#enumset_ident::EnumSet<Self::Field> {
                    match self {
                        #( #changes_arms ),*
                    }
                }
            }
        }
    }
}
