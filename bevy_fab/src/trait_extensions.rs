//! Extensions to the `App` to
use bevy::{app::App, reflect::Reflect};
use fab::binding::Entry;
use fab_parse::Styleable;

use crate::{BevyModify, Styles, WorldBindings};

// Initially, I wanted to pass the styles to `RichTextPlugin` and insert them
// at initialization. But it's impossible due to `Styles` containing `Box<dyn FnMut>`,
// which cannot be wrought to implement `Clone`. I temporarilly considered using
// `Arc<dyn FnMut>` instead. But then it is impossible to call the functions, since
// `Arc` is immutable. So I opted to add the following

/// Extension trait to add `alias` and `chop` modifiers to the string format parser.
pub trait AppStylesExtension<M: BevyModify> {
    /// Insert a new style before all others.
    fn overwrite_style<F>(&mut self, style: F) -> &mut Self
    where
        F: FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static;
    /// Add a new style after existing ones.
    fn add_style<F>(&mut self, style: F) -> &mut Self
    where
        F: FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static;
}
impl<M: BevyModify> AppStylesExtension<M> for App {
    fn overwrite_style<F>(&mut self, style: F) -> &mut Self
    where
        F: FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static,
    {
        let Some(mut styles) = self.world.get_resource_mut::<Styles<M>>() else { return self; };
        styles.overwrite(style);
        self
    }
    fn add_style<F>(&mut self, style: F) -> &mut Self
    where
        F: FnMut(Styleable<M>) -> Styleable<M> + Send + Sync + 'static,
    {
        let Some(mut styles) = self.world.get_resource_mut::<Styles<M>>() else { return self; };
        styles.add(style);
        self
    }
}

pub trait AppFormattersExtension<M: BevyModify> {
    fn with_formatter<T: Reflect>(
        &mut self,
        name: impl Into<String>,
        formatter: impl Fn(&T, Entry<M>) + Send + Sync + 'static,
    ) -> &mut Self;
}
impl<M: BevyModify> AppFormattersExtension<M> for App {
    fn with_formatter<T: Reflect>(
        &mut self,
        name: impl Into<String>,
        formatter: impl Fn(&T, Entry<M>) + Send + Sync + 'static,
    ) -> &mut Self {
        let Some(mut world_bindings) = self.world.get_resource_mut::<WorldBindings<M>>() else {
            return self;
        };
        world_bindings.add_formatter(name, formatter);
        self
    }
}
