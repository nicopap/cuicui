//! Extensions to the `App` to
use bevy::{app::App, prelude::Mut, reflect::Reflect};
use fab::binding::Entry;
use fab_parse::Styleable;

use crate::{
    fmt_system::{FmtSystem, IntoFmtSystem},
    track::UserFmt,
    BevyModify, Styles, WorldBindings,
};

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
    fn add_user_fmt(&mut self, name: impl AsRef<str>, fmt: UserFmt<M>) -> &mut Self;

    // Add a formatter that may READ (only) the world.
    fn add_sys_fmt<T: FmtSystem<M>>(
        &mut self,
        name: impl AsRef<str>,
        fmt: impl IntoFmtSystem<M, T>,
    ) -> &mut Self;

    // Add a simple function formatter.
    fn add_dyn_fn_fmt(
        &mut self,
        name: impl AsRef<str>,
        fmt: impl Fn(&dyn Reflect, Entry<M>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_user_fmt(name, UserFmt::from_fn(fmt))
    }

    // Add a simple function formatter, with a known type as input.
    fn add_fn_fmt<T: Reflect>(
        &mut self,
        name: impl AsRef<str>,
        fmt: impl Fn(&T, Entry<M>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_user_fmt(
            name,
            UserFmt::from_fn(move |reflect, e| {
                let value = reflect.downcast_ref().unwrap();
                fmt(value, e)
            }),
        )
    }
}
impl<M: BevyModify> AppFormattersExtension<M> for App {
    fn add_user_fmt(&mut self, name: impl AsRef<str>, fmt: UserFmt<M>) -> &mut Self {
        let mut world_bindings = self.world.resource_mut::<WorldBindings<M>>();
        world_bindings.add_user_fmt(name, fmt);
        self
    }
    fn add_sys_fmt<T: FmtSystem<M>>(
        &mut self,
        name: impl AsRef<str>,
        fmt: impl IntoFmtSystem<M, T>,
    ) -> &mut Self {
        self.world
            .resource_scope(|world, mut bindings: Mut<WorldBindings<M>>| {
                bindings.add_user_fmt(name, UserFmt::from_system(fmt, world));
            });
        self
    }
}
