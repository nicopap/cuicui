use core::fmt;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use super::{
    path::{Components, Path},
    Mode, Modifiers,
};

/// See [`Atom`], this also includes a `usize` for the position in a array
/// of the child/parent.
enum Found {
    Child(usize),
    Parent(usize),
    Equal,
}
/// Relationship between two [`Path`]s.
///
/// Considering relationship between `lhs` (left-hand side) and `rhs`
/// (right-hand side).
enum Atom {
    /// `rhs` accesses a sub-field of `lhs`, meaning that `lhs`
    /// may read/write to `lhs`
    Child,
    /// Same as `Child`, but with `rhs` and `lhs` swapped.
    Parent,
    /// `lhs == rhs`
    Equal,
    /// `rhs` and `lhs` do not overlap.
    Diverges,
}
impl Atom {
    fn found(self, i: usize) -> Option<Found> {
        match self {
            Atom::Child => Some(Found::Child(i)),
            Atom::Equal => Some(Found::Equal),
            Atom::Parent => Some(Found::Parent(i)),
            Atom::Diverges => None,
        }
    }
}
struct Accessor {
    span: Span,
    comps: Components,
}
impl Accessor {
    fn new(span: Span, components: Components) -> Self {
        Accessor { span, comps: components }
    }
    /// The [`Atom`] relationship between self and other.
    ///
    /// It should be read "self `<return value>` of other", for example, if this
    /// method returns `Atom::Parent`, we get:
    ///
    /// > "self parent of other"
    fn atom_of(&self, other: &Path) -> Atom {
        let self_comps = &self.comps.path;
        let other_comps = &other.components.path;
        let mut zipped = self_comps.iter().zip(other_comps.iter());
        let same_source = self.comps.source == other.components.source;
        let all_equal = zipped.all(|(l, r)| l == r) && same_source;
        let self_more_components = self_comps.len() > other_comps.len();
        let self_less_components = self_comps.len() < other_comps.len();

        match all_equal {
            false => Atom::Diverges,
            true if self_more_components => Atom::Child,
            true if self_less_components => Atom::Parent,
            true => Atom::Equal,
        }
    }
}

/// List of all the atomic accessors used by all the modify functions.
///
/// Used to atomize accessors, for proper dependency management.
///
/// An accessor is *atomic* when it cannot be split into more fine-grained
/// accessors.
///
/// Consider the following accessors:
///
/// ```text
/// #[modify(read(.zooba))]
/// fn read_zooba {}
///
/// #[modify(write(.gee))]
/// fn write_gee {}
///
/// #[modify(read(.gee.wooz))]
/// fn read_gee_wooz {}
///
/// #[modify(read(.gee.zaboo))]
/// fn read_gee_zaboo {}
/// ```
///
/// Suppose a `read_gee_zaboo` is nested in a `write_gee` section.
/// We need to trigger `read_gee_zaboo` if `write_gee` is triggered.
/// To do so, we need to know that `write_gee` does indeed write to `.gee.zaboo`.
/// Splitting `.gee` into its "atomic parts" let us now what modify functions
/// to trigger when `.gee` is triggered.
///
/// See the `./design_doc/fab/track_nested_field.md` file for details.
pub struct AtomicAccessors(Vec<Accessor>);
impl AtomicAccessors {
    fn new() -> Self {
        AtomicAccessors(Vec::new())
    }
    fn atomized<'a, 'b: 'a>(&'b self, rel: &'a Path) -> impl Iterator<Item = &'b Accessor> + 'a {
        self.0
            .iter()
            .filter(|a| matches!(a.atom_of(rel), Atom::Child | Atom::Equal))
    }
    fn index_of_subset(&self, rel: &Path) -> Option<Found> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(i, a)| a.atom_of(rel).found(i))
    }
    /// Accumulates `accessors` into `Self`, removing duplicates & parents
    /// of existing children.
    pub fn from_non_atomic<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Self {
        let mut atomic = Self::new();

        for path in paths {
            match atomic.index_of_subset(path) {
                Some(Found::Parent(index)) => {
                    atomic.0[index] = Accessor::new(path.span, path.components.clone())
                }
                Some(Found::Child(_) | Found::Equal) => {}
                None => atomic
                    .0
                    .push(Accessor::new(path.span, path.components.clone())),
            }
        }
        atomic
    }

    pub(crate) fn all_variants(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let to_variant = |a: &Accessor| {
            let literal = syn::LitStr::new(&a.comps.doc_string(), a.span);
            let attributes = quote!(#[doc = #literal]);
            let variant = a.comps.variant_ident(a.span);
            quote!(#attributes #variant)
        };
        self.0.iter().map(to_variant)
    }
}
/// List of atomic accessors
pub struct FnAtomicAccessors {
    read: Vec<(Span, Components)>,
    write: Vec<(Span, Components)>,
}
impl FnAtomicAccessors {
    /// Create `Self`, accessors of `modifiers` (ie: their modify attributes)
    /// where each of them is atomic.
    pub fn new(modifiers: &Modifiers, accessors: &AtomicAccessors) -> Self {
        let mut read = Vec::new();
        let mut write = Vec::new();

        for modifier in &modifiers.mods {
            let add_to_buffer = |b: &mut Vec<_>| {
                accessors
                    .atomized(&modifier.path)
                    .for_each(|atomized| b.push((atomized.span, atomized.comps.clone())))
            };
            if modifier.is_write() {
                add_to_buffer(&mut write);
            }
            if modifier.is_read() {
                add_to_buffer(&mut read);
            }
        }
        FnAtomicAccessors { read, write }
    }
    pub fn access_doc_string(&self) -> AccessorDoc {
        AccessorDoc(self)
    }
    pub fn variant_idents(&self, mode: Mode) -> impl ExactSizeIterator<Item = Ident> + '_ {
        let buffer = match mode {
            Mode::Read => &self.read,
            Mode::Write => &self.write,
        };
        buffer.iter().map(|(span, comp)| comp.variant_ident(*span))
    }
}
pub struct AccessorDoc<'a>(&'a FnAtomicAccessors);
impl fmt::Display for AccessorDoc<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reads = !self.0.read.is_empty();
        let writes = !self.0.write.is_empty();
        if reads {
            write!(f, "* Reads: ")?;
            let mut reads = self.0.read.iter();
            if let Some(first) = reads.next() {
                write!(f, "`{}`", first.1.pretty_fmt())?;
            }
            for read in reads {
                write!(f, ", `{}`", read.1.pretty_fmt())?;
            }
            writeln!(f)?;
        }

        if writes {
            write!(f, "* Writes to: ")?;
            let mut writes = self.0.write.iter();
            if let Some(first) = writes.next() {
                write!(f, "`{}`", first.1.pretty_fmt())?;
            }
            for write in writes {
                write!(f, ", `{}`", write.1.pretty_fmt())?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
