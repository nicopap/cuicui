use std::{fmt, mem, num::NonZeroU32};

use datazoo::Index;
use string_interner::Symbol;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Ord)]
pub struct Id(pub(crate) NonZeroU32);
impl Index for Id {
    #[inline]
    fn get(&self) -> usize {
        self.0.get() as usize - 1
    }
}
impl From<usize> for Id {
    fn from(value: usize) -> Self {
        let u32 = u32::try_from(value).unwrap();
        Id(NonZeroU32::new(u32.saturating_add(1)).unwrap())
    }
}
impl Symbol for Id {
    #[inline]
    fn try_from_usize(index: usize) -> Option<Self> {
        let u32 = u32::try_from(index).ok()?;
        Some(Id(NonZeroU32::new(u32.saturating_add(1)).unwrap()))
    }

    #[inline]
    fn to_usize(self) -> usize {
        assert!(
            mem::size_of::<usize>() >= mem::size_of::<Self>(),
            "NOTE: please open an issue if you need to run bevy on 16 bits plateforms"
        );
        Index::get(&self)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<B{}>", self.0)
    }
}
