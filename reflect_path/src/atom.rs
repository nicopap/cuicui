use crate::Path;

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
#[derive(PartialEq, Debug, Clone, Copy)]
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

/// A list of atomic accessors.
///
/// Used to atomize accessors, for proper dependency management.
///
/// An accessor is *atomic* when it cannot be split into more fine-grained
/// accessors.
///
/// Consider the following accessors:
/// ```text
/// .zooba
/// .gee
/// .gee.wooz
/// .gee.zaboo
/// ```
/// If we have mutable access to `.gee`, it is possible to also access `.gee.wooz`,
/// so we should forbid access to `.gee.wooz` when providing access to `.gee`.
///
/// See the `./design_doc/fab/track_nested_field.md` file for details.
pub(crate) struct AtomicAccess<'a>(Vec<Path<'a>>);

impl Path<'_> {
    /// The [`Atom`] relationship between self and other.
    ///
    /// It should be read "self `<return value>` of other", for example, if this
    /// method returns `Atom::Parent`, we get:
    ///
    /// > "self parent of other"
    fn atom_of(&self, other: &Path) -> Atom {
        let self_comps = &self.0;
        let other_comps = &other.0;
        let mut zipped = self_comps.iter().zip(other_comps.iter());
        let all_equal = zipped.all(|(l, r)| l == r);
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

impl<'z> AtomicAccess<'z> {
    pub fn new() -> Self {
        AtomicAccess(Vec::new())
    }
    pub fn with_capacity(capacity: usize) -> Self {
        AtomicAccess(Vec::with_capacity(capacity))
    }
    fn atomized<'a>(&'a self, rel: &'a Path<'a>) -> impl Iterator<Item = &'_ Path<'z>> + 'a {
        self.0
            .iter()
            .filter(|a| a.atom_of(rel) != Atom::Diverges)
            .filter(|a| a.atom_of(rel) != Atom::Parent)
    }
    fn index_of_subset(&self, rel: &Path) -> Option<Found> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(i, a)| a.atom_of(rel).found(i))
    }
    /// Accumulates `accessors` into `Self`, removing duplicates & parents
    /// of existing children.
    pub fn from_non_atomic(paths: impl IntoIterator<Item = Path<'z>>) -> Self {
        let mut atomic = Self::new();

        for path in paths {
            match atomic.index_of_subset(&path) {
                Some(Found::Parent(index)) => atomic.0[index] = path,
                Some(Found::Child(_) | Found::Equal) => {}
                None => atomic.0.push(path),
            }
        }
        atomic
    }
    /// Try to add `path` to the list of accessor. If it conflicts, then
    /// returns `true`. `false` otherwise.
    pub fn add_path(&mut self, path: Path<'z>) -> bool {
        match self.index_of_subset(&path) {
            Some(_) => return true,
            None => self.0.push(path),
        }
        false
    }
}
