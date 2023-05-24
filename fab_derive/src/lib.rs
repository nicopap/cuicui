use std::mem;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    parse::Parse, parse::Parser, parse_macro_input, punctuated::Punctuated, Attribute, Field,
    ItemFn, ItemImpl, Meta, MetaList, Token,
};

/// `impl_modify` is an attribute macro to define `Modify<I>` with correct
/// change tracking without too much headache.
///
/// # Syntax
///
/// See the next section for a detailed explanation with semantic information.
///
/// `#[impl_modify]` only works on `impl` block, it should be formatted as a
/// trait implementation for `Modify<I>` as follow:
///
/// ```
/// #[impl_modify]
/// impl Modify<TextSection> for CustomModify {
///     // ...
/// }
/// ```
///
/// The first item (declaration) within that `impl` block should be a type
/// definition for `Context`. The type definition accepts either 1 or 0 lifetime
/// parameters:
///
/// ```
/// #[impl_modify]
/// impl Modify<TextSection> for CustomModify {
///     type Context<'a> = GetFont<'a>;
///     // ...
/// }
/// ```
///
/// All other items are function declarations. They can be documented. You can
/// decorate them with the `modify` attributes.
///
/// ```
/// #[impl_modify]
/// impl Modify<TextSection> for CustomModify {
///     type Context<'a> = GetFont<'a>;
///
///     #[modify(read_write(it.style.color))]
///     fn shift_hue(hue_offset: f32, color: &mut Color) {
///         let mut hsl = color.as_hsla_f32();
///         hsl[0] = (hsl[0] + hue_offset) % 360.0;
///         *color = Color::hsla(hsl[0], hsl[1], hsl[2], hsl[3]);
///     }
///     // ...
/// }
/// ```
///
/// ## Attributes
///
/// All the `modify` attributes are:
///
/// - `#[modify(context(value))]`: Pass the context to the parameter named `value`.
///    It must be of the same type as `type Context`.
/// - `#[modify(read(it.path.to.value))]`: Which field of the item (`it` stands
///    for item) to read. The last field name (here `value`) is the argument
///    used to pass thie value to the function.
/// - `#[modify(write(it.path.to.value))]`: which field of the item  to update
///    with the return value of this function, a function can only have a
///    single `write` attribute, and this excludes the use of `read_write`.
/// - `#[modify(read_write(it.path.to.value))]`:
/// - `#[modify(write_mut(it.path.to.value))]`: This works like `read_write`
///    (the argument to modify is passed as a `&mut`) but we assume that you do
///    not read or use the content of the value in the return value. This is
///    typically useful when you want to overwrite a string value without
///    allocating anything.
///
/// ```
/// #[modify(read(it.path.to.value))]
/// fn some_change(value: f32) {
///     // ...
/// }
/// ```
///
/// You might want to rename the field name before passing it as argument.
/// To do so, use the following syntax:
///
/// ```
/// #[modify(read(unique_value_name = it.path.to.value))]
/// fn some_change(unique_value_name: f32) {
///     // ...
/// }
/// ```
///
/// # How does it look?
///
/// You define a `Modify` operating on an arbitrary item, here, we will use the
/// bevy `TextSection`, element of a `Text` component.
///
/// `Modify` itself is a trait, you need a type on which to implement it. Here,
/// we chose to name our `Modify` type `CustomModify` (ik very creative).
///
/// **Do not define `CustomModify` yourself**, `impl_modify` will implement it
/// for you. See the next section to learn how `CustomModify` looks like.
///
/// `CustomModify` operations are defined as functions in the `impl` block.
/// Those functions have two kinds of arguments:
///
/// 1. Internal parameters: will be constructors of `CustomModify`.
/// 2. Passed parameters: they are fields of the modiify item `I`,
///    here `TextSection`
/// ```
/// #[impl_modify]
/// impl Modify<TextSection> for CustomModify {
///     type Context<'a> = GetFont<'a>;
///     
/// }
/// ```
///
/// # How does it work?
///
/// `impl_modify` creates two enums:
///
/// - `CustomModify`: One variant per free function defined in the `impl_modify`
///   block.
/// - `CustomModifyField`: each accessed field, implements `EnumSetType`,
///   to be used as `Modify::Field` of `CustomModify`.
///
/// `CustomModify` implements `Modify<TextSection>` and has one constructor per
/// defined function in `impl_modify`. In our last example, we defined:
///
/// - `shift_hue(offset: f32)`
/// - `color(set_to: Color)`
/// - `font(name: String)`
///
/// Therefore, our `CustomModify` will look as follow:
///
/// ```
/// enum CustomModify {
///   ShiftHue { offset: f32 },
///   Color { set_to: Color },
///   Font { name: String },
/// }
/// impl CustomModify {
///   const fn shift_hue(offset: f32) -> Self {
///     Self::ShiftHue { offset }
///   }
///   const fn color(set_to: Color) -> Self {
///     Self::Color { set_to }
///   }
///   pub const fn font(name: String) -> Self {
///     Self::Font { name }
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn impl_modify(attr: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as ItemImpl);
    let block = Block::parse(input);
    TokenStream1::from(block.generate_impl())
}
struct Path {
    ident: Ident,
    tokens: TokenStream,
}
impl Path {
    fn to_field_enum_name(&self) -> Ident {
        todo!()
    }
}
impl Parse for Path {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let is_assignment = input.peek(syn::Ident) && input.peek2(Token![=]);
        if is_assignment {
            let ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;
            let tokens = input.cursor().token_stream();
            Ok(Path { ident, tokens })
        } else {
            let tokens = input.fork().cursor().token_stream();
            let ident = input.cursor().token_stream().into_iter().last();
            let Some(ident) = ident else {
                // span is `input`
                panic!("Modify path not declared");
            };
            let TokenTree::Ident(ident) = ident else {
                // span is `ident`
                panic!("Implicit name not ident");
            };
            Ok(Path { ident, tokens })
        }
    }
}
struct ReadAndWrite {
    read: Ident,
    write: Ident,
}
impl Parse for ReadAndWrite {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let read = input.parse()?;
        let _ = input.parse::<Token![,]>()?;
        let write = input.parse()?;
        Ok(ReadAndWrite { read, write })
    }
}
enum Modify {
    Context(Ident),
    Write(Path),
    WriteMut(Path),
    Read(Path),
    ReadWrite(Path),
    DynamicReadWrite(Ident, Ident),
}
fn iter_attrs(tokens: TokenStream) -> Punctuated<MetaList, Token![,]> {
    let parser = Punctuated::<MetaList, Token![,]>::parse_separated_nonempty;
    parser.parse2(tokens).unwrap()
}
impl Modify {
    fn parse_individual(input: MetaList) -> syn::Result<Self> {
        use syn::parse2 as parse;

        let is = |ident: &str, path: &syn::Path| path.get_ident().map_or(false, |i| i == ident);
        let parsed = match &input.path {
            path if is("context", &path) => Modify::Context(parse(input.tokens)?),
            path if is("write", &path) => Modify::Write(parse(input.tokens)?),
            path if is("write_mut", &path) => Modify::WriteMut(parse(input.tokens)?),
            path if is("read", &path) => Modify::Read(parse(input.tokens)?),
            path if is("read_write", &path) => Modify::ReadWrite(parse(input.tokens)?),
            path if is("dynamic_read_write", &path) => {
                let ReadAndWrite { read, write } = parse(input.tokens)?;
                Modify::DynamicReadWrite(read, write)
            }
            _ => {
                panic!("noeadf")
            }
        };
        Ok(parsed)
    }
    fn parse(attr: &mut Attribute) -> impl Iterator<Item = syn::Result<Modify>> {
        let is_modify = |path: &syn::Path| path.get_ident().map_or(false, |i| i == "modify");
        let list = match &mut attr.meta {
            Meta::List(MetaList { path, tokens, .. }) if is_modify(&path) => {
                iter_attrs(mem::take(tokens))
            }
            _ => Punctuated::new(),
        };
        list.into_iter().map(Self::parse_individual)
    }
}
struct Block {
    attributes: Vec<Attribute>,
    impl_target: syn::Type,
    context: syn::Type,
    functions: Vec<ModifyFn>,
    access_paths: Vec<Path>,
    // TODO(feat): allow generics
    ty_name: Ident,
}
impl Block {
    fn parse(input: ItemImpl) -> Self {
        todo!()
    }
    fn generate_impl(self) -> TokenStream {
        // TODO(bug): handle different crate export names
        let enumset_crate = Ident::new("enumset", Span::call_site());
        let fab_crate = Ident::new("cuicui_fab", Span::call_site());

        let ty_name = &self.ty_name;
        let ty_name_field = format_ident!("{ty_name}Field");
        let field_variants = self.access_paths.iter().map(Path::to_field_enum_name);
        let impl_target = &self.impl_target;

        let ty_variants: Vec<_> = self.functions.iter().map(ModifyFn::ty_variant).collect();
        let changes_arms = self.functions.iter().map(ModifyFn::changes_arm);
        let depends_arms = self.functions.iter().map(ModifyFn::depends_arm);
        let ty_constructors = self.functions.iter().map(|m| &m.constructor);
        let ty_function_defs = self.functions.iter().map(ModifyFn::def);
        let ty_function_calls = self.functions.iter().map(ModifyFn::call);

        let ty_attributes = &self.attributes;
        let context = &self.context;
        quote! {
            /// Fields accessed by [`
            #[doc = #ty_name]
            /// `].
            #[derive(#enumset_crate::EnumSetType)]
            enum #ty_name_field {
                #( #field_variants ),*
            }
            #( #ty_attributes )*
            enum #ty_name {
                #( #ty_variants ),*
            }
            impl #ty_name {
                #( #ty_constructors )*
            }
            impl Modify<#impl_target> for #ty_name {
                type Field = #ty_name_field;
                type Context<'a> = #context;

                fn apply(
                    &self,
                    context: &Self::Context<'_>,
                    item: &mut #impl_target,
                ) -> #fab_crate::anyhow::Result<()> {
                    match self {
                        #(
                            Self::#ty_variants => {
                                #ty_function_defs
                                #ty_function_calls
                            }
                        ),*
                        todo!()
                    }
                }

                fn depends(&self) -> #enumset_crate::EnumSet<Self::Field> {
                    match self {
                        #( #depends_arms ),*
                    }
                }

                fn changes(&self) -> #enumset_crate::EnumSet<Self::Field> {
                    match self {
                        #( #changes_arms ),*
                    }
                }
            }

        }
    }
}
type MyArm = TokenStream;
type MyVariant = TokenStream;
type MyItemFn = TokenStream;
type MyExprCall = TokenStream;
struct ModifyFn {
    name: Ident,
    fields: Vec<Field>,
    function: ItemFn,
    constructor: ItemFn,
    reads: Vec<Path>,
    writes: Vec<Path>,
}
impl ModifyFn {
    fn new(input: ItemFn) -> Self {
        ModifyFn {
            name: input.sig.ident.clone(),
            fields: Vec::new(),
            function: input.clone(),
            constructor: input,
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }
    fn call(&self) -> MyExprCall {
        todo!()
    }
    fn def(&self) -> MyItemFn {
        todo!()
    }
    fn ty_name(&self) -> Ident {
        todo!()
    }
    fn ty_variant(&self) -> MyVariant {
        let ty_name = self.ty_name();
        let fields = &self.fields;
        let attrs = &self.function.attrs;
        quote! {
            #( #attrs )*
            #ty_name { #( #fields ),* }
        }
    }
    fn depends_arm(&self) -> MyArm {
        let ty_name = self.ty_name();
        let reads = self.reads.iter().map(Path::to_field_enum_name);
        quote! {
            Self::#ty_name { .. } => EnumSet::EMPTY #( | #reads )*
        }
    }
    fn changes_arm(&self) -> MyArm {
        let ty_name = self.ty_name();
        let writes = self.writes.iter().map(Path::to_field_enum_name);
        quote! {
            Self::#ty_name { .. } => EnumSet::EMPTY #( | #writes )*
        }
    }
    /// Take all attributes marked with `modify` and drain them.
    fn modify_args(&mut self) -> Vec<syn::Result<Modify>> {
        let mut ret = Vec::new();
        self.function.attrs.retain_mut(|attr| {
            let old_len = ret.len();
            ret.extend(Modify::parse(attr));
            old_len == ret.len()
        });
        ret
    }
    fn apply_modify_args(&mut self, args: Vec<Modify>) {
        self.function.sig.inputs;
        todo!()
    }
}
