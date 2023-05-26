#![doc = include_str!("../README.md")]

// Architecture:
//
// `block` (`Block`) parses the whole item block, inlcuding the `impl Modify<Foo> for Bar`.
// It also generates most of the code.
//
// `modify_fn` (`ModifyFn`) is a single function item in the `impl_modify` block.
// It processes the single declared function and its `#[modify(…)]` attributes
// and provides methods to create the generated code based on those.
//
// `modifiers` (`Modifiers`) is a list of `ModifyFn` modifiers. Modifiers are
// declaration of what fields the function reads and writes, and which parameter
// of the modify function those fields correspond to.
// It provides methods to generate the fields and modify enums
// and argument lists for the modify functions.
//
// `path` (`Path`) is a modifier path into the item. Provides method to create
// the fields enum.

mod block;
mod extensions;
mod modifiers;
mod modify_fn;
mod path;

use block::Config;
use proc_macro::TokenStream as TokenStream1;
use syn::{parse_macro_input, ItemImpl};

use crate::block::Block;

#[doc = include_str!("../README.md")]
#[proc_macro_attribute]
pub fn impl_modify(attrs: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let mut config = Config::default();

    if !attrs.is_empty() {
        let config_parser = syn::meta::parser(|meta| config.parse(meta));
        parse_macro_input!(attrs with config_parser);
    }
    let input = parse_macro_input!(input as ItemImpl);
    match Block::parse(config, input) {
        Err(errors) => errors.into_compile_error().into(),
        Ok(block) => block.generate_impl().into(),
    }
}
