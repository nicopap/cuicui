//! A slice of `u32` accessed on the bit level.

#[cfg(test)]
mod tests;

use std::{fmt, iter, ops::Range};

use sorted_iter::sorted_iterator::SortedByItem;

use crate::div_ceil;

trait BlockT {
    const BITS64: usize;
}
impl BlockT for u32 {
    const BITS64: usize = u32::BITS as usize;
}

/// A slice of `u32` accessed on the bit level, see [wikipedia][bitset].
///
/// # Usage
///
/// `Bitset` is parametrized on the storage type, to let you chose whether
/// this needs to be a reference, a `Box`, a `Vec` etc.
///
/// Mutable methods are only available when the underlying storage allows
/// mutable access.
///
/// ```rust
/// use cuicui_datazoo::Bitset;
/// let bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
///
/// let as_array: Bitset<[u32; 3]> = Bitset(bunch_of_bits);
/// let mut as_vec: Bitset<Vec<u32>> = Bitset(bunch_of_bits.to_vec());
/// let as_slice: Bitset<&[u32]> = Bitset(&bunch_of_bits);
/// let as_box: Bitset<Box<[u32]>> = Bitset(Box::new(bunch_of_bits));
///
/// assert_eq!(
///     as_array.ones_in_range(5..91),
///     as_vec.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_slice.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_slice.ones_in_range(5..91),
///     as_box.ones_in_range(5..91),
/// );
/// assert_eq!(
///     as_box.ones_in_range(5..91),
///     as_array.ones_in_range(5..91),
/// );
/// ```
///
/// To use mutable methods ([`Bitset::enable_bit`] is currently the only one),
/// the backing storage `B` must be mutable. Otherwise, you just can't use them.
///
/// ```compile_fail
/// # use cuicui_datazoo::Bitset;
/// # let bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
/// let as_slice: Bitset<&[u32]> = Bitset(&bunch_of_bits);
///
/// as_slice.enable_bit(11);
/// ```
///
/// `Vec<_>` and `&mut [_]` do support mutable access, so the following works:
///
/// ```
/// # use cuicui_datazoo::Bitset;
/// # let mut bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
/// let mut as_vec: Bitset<Vec<u32>> = Bitset(bunch_of_bits.to_vec());
/// let mut as_mut_slice: Bitset<&mut [u32]> = Bitset(&mut bunch_of_bits);
///
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// as_vec.enable_bit(11);
///
/// assert_ne!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// as_mut_slice.enable_bit(11);
///
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_mut_slice.ones_in_range(5..91),
/// );
/// ```
///
/// [bitset]: https://en.wikipedia.org/wiki/Bit_array
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitset<B: AsRef<[u32]>>(pub B);

impl Bitset<Vec<u32>> {
    /// Enables bit at position `bit`, extending the vector if necessary.
    ///
    /// When [`Bitset::bit(bit)`] will be called next, it will always be `true`.
    ///
    /// # Example
    ///
    /// ```
    /// # use cuicui_datazoo::Bitset;
    /// let mut as_vec = Bitset(vec![]);
    /// assert!(as_vec.enable_bit(64).is_none());
    /// assert_eq!(as_vec.0.len(), 0);
    ///
    /// as_vec.enable_bit_extending(73);
    ///
    /// assert!(as_vec.bit(73));
    /// assert!(as_vec.enable_bit(64).is_some());
    /// assert!(as_vec.bit(64));
    /// assert_eq!(as_vec.0.len(), 3);
    /// ```
    pub fn enable_bit_extending(&mut self, bit: usize) {
        let block = bit / u32::BITS64;
        let offset = bit % u32::BITS64;

        if block >= self.0.len() {
            let extra_blocks = block - self.0.len() + 1;
            self.0.extend(iter::repeat(0).take(extra_blocks));
        }
        self.0[block] |= 1 << offset;
    }
}
impl<B: AsRef<[u32]> + AsMut<[u32]>> Bitset<B> {
    /// Enables bit at position `bit`.
    ///
    /// Returns `None` and does nothing if `bit` is out of range.
    ///
    /// When [`Bitset::bit(bit)`] will be called next, it will be `true`
    /// if this returned `Some`.
    #[inline]
    pub fn enable_bit(&mut self, bit: usize) -> Option<()> {
        let block = bit / u32::BITS64;
        let offset = bit % u32::BITS64;

        self.0.as_mut().get_mut(block).map(|block| {
            *block |= 1 << offset;
        })
    }
}
impl<B: AsRef<[u32]>> Bitset<B> {
    #[inline]
    pub fn bit_len(&self) -> usize {
        self.0.as_ref().len() * u32::BITS64
    }
    /// True if bit at `at` is enabled, false if out of bound or disabled.
    #[inline]
    pub fn bit(&self, at: usize) -> bool {
        let block = at / u32::BITS64;
        let offset = (at % u32::BITS64) as u32;
        let offset = 1 << offset;
        let Some(block) = self.0.as_ref().get(block) else { return false };

        block & offset == offset
    }
    pub fn ones_in_range(&self, range: Range<usize>) -> Ones {
        let Range { start, end } = range;
        assert!(start <= self.bit_len());
        assert!(end <= self.bit_len());

        // the offset to "crop" the bits at the edges of the [u32]
        let crop = Range {
            start: (start % u32::BITS64) as u32,
            end: (end % u32::BITS64) as u32,
        };
        // The indices of Blocks of [u32] (ie: NOT bits) affected by range
        let range = Range {
            start: start / u32::BITS64,
            end: div_ceil(end, u32::BITS64),
        };
        let all_blocks = &self.0.as_ref()[range.clone()];

        let (mut bitset, remaining_blocks) = all_blocks
            .split_first()
            .map_or((0, all_blocks), |(b, r)| (*b, r));

        bitset &= ((1 << crop.start) - 1) ^ u32::MAX;
        if remaining_blocks.is_empty() && crop.end != 0 {
            bitset &= (1 << crop.end) - 1;
        }
        Ones {
            block_idx: u32::try_from(range.start).unwrap(),
            crop: crop.end,

            bitset,
            remaining_blocks,
        }
    }
}
impl<B: AsRef<[u32]>> fmt::Debug for Bitset<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, block) in self.0.as_ref().iter().enumerate() {
            if i != 0 {
                write!(f, "_")?;
            }
            write!(f, "{block:08x}")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
impl<'a, B: AsRef<[u32]>> IntoIterator for &'a Bitset<B> {
    type Item = u32;
    type IntoIter = Ones<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.ones_in_range(0..self.bit_len())
    }
}
impl Extend<u32> for Bitset<Vec<u32>> {
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        for bit in iter {
            self.enable_bit_extending(bit as usize)
        }
    }
}
impl Extend<usize> for Bitset<Vec<u32>> {
    fn extend<T: IntoIterator<Item = usize>>(&mut self, iter: T) {
        for bit in iter {
            self.enable_bit_extending(bit)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ones<'a> {
    /// Index in u32 of `bitset`.
    block_idx: u32,
    /// How many bits to keep in the last block.
    crop: u32,

    bitset: u32,
    remaining_blocks: &'a [u32],
}
impl Iterator for Ones<'_> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.bitset == 0 {
            let Some((&bitset, remaining_blocks)) =  self.remaining_blocks.split_first() else {
                return None;
            };
            self.bitset = bitset;
            self.remaining_blocks = remaining_blocks;

            if self.remaining_blocks.is_empty() && self.crop != 0 {
                self.bitset &= (1 << self.crop) - 1;
            }
            self.block_idx += 1;
        }
        let t = self.bitset & 0_u32.wrapping_sub(self.bitset);
        let r = self.bitset.trailing_zeros();
        self.bitset ^= t;
        Some(self.block_idx * u32::BITS + r)
    }
}
impl SortedByItem for Ones<'_> {}
