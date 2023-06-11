//! Define [`Ref`], a custom version of bevy's [`Ref`](BRef) that can be
//! constructed from `EntityRef` and mapped over.
use std::ops::Deref;

use bevy::prelude::{DetectChanges, Ref as BRef};

/// A custom version of bevy's [`Ref`](BRef) that can be
/// constructed from `EntityRef` and mapped over.
///
/// Due to a limitation in bevy, it's impossible to use the bevy `Ref` for this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ref<'w, T: ?Sized> {
    pub(crate) value: &'w T,
    pub(crate) is_added: bool,
    pub(crate) is_changed: bool,
}
impl<'w, T: ?Sized> Ref<'w, T> {
    /// Was this added since last time the system this is called from ran?
    #[must_use]
    pub const fn is_added(&self) -> bool {
        self.is_added
    }
    /// Was this changed since last time the system this is called from ran?
    #[must_use]
    pub const fn is_changed(&self) -> bool {
        self.is_changed
    }
    /// Return the inner reference with its full lifetime.
    ///
    /// Rust's [`Deref`] trait can't produce references with a lifetime longer
    /// than that of the `Ref` itself (with `&'a Ref<'w>`, the lifetime will
    /// always be `'a`).
    ///
    /// This can become an issue in certain scenarios, this is why this method
    /// exists.
    ///
    /// Note that since `Ref` is `Copy`, this **doesn't** consume the value, you
    /// can keep using it.
    #[must_use]
    pub const fn into_inner(self) -> &'w T {
        self.value
    }
    /// Apply a function `f` returning a result to the inner value,
    /// and get a `Ref` with the return value of that function.
    /// Returning `Err` if `f` returns `Err`.
    ///
    /// # Errors
    /// Returns `Err` if `f` returns `Err`.
    pub fn map_failable<E, U: ?Sized>(
        self,
        f: impl FnOnce(&T) -> Result<&U, E>,
    ) -> Result<Ref<'w, U>, E> {
        Ok(Ref {
            value: f(self.value)?,
            is_added: self.is_added,
            is_changed: self.is_changed,
        })
    }
    /// Apply a function `f` to the inner value,
    /// and get a `Ref` with the return value of that function.
    #[must_use]
    pub fn map<U: ?Sized>(self, f: impl FnOnce(&T) -> &U) -> Ref<'w, U> {
        Ref {
            value: f(self.value),
            is_added: self.is_added,
            is_changed: self.is_changed,
        }
    }
    /// Convert a bevy [`Ref`](BRef) into a `reflect_query` `Ref`, applying
    /// a function while converting it.
    ///
    /// You can pass `|i| i` as function if you don't wish to convert the value.
    #[must_use]
    pub fn map_from<U: ?Sized>(bevy: BRef<'w, U>, f: impl FnOnce(&U) -> &T) -> Self {
        Ref {
            is_added: bevy.is_added(),
            is_changed: bevy.is_changed(),
            value: f(bevy.into_inner()),
        }
    }
}
impl<'w, T: ?Sized> From<BRef<'w, T>> for Ref<'w, T> {
    fn from(value: BRef<'w, T>) -> Self {
        Ref::map_from(value, |i| i)
    }
}
impl<'w, T: ?Sized> AsRef<T> for Ref<'w, T> {
    fn as_ref(&self) -> &T {
        self.value
    }
}
impl<'w, T: ?Sized> Deref for Ref<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
impl<'w, 'a, T> IntoIterator for &'a Ref<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}
