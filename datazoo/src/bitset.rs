use std::ops::Range;

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
/// # let bunch_of_bits = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];
/// let mut as_vec: Bitset<Vec<u32>> = Bitset(bunch_of_bits.to_vec());
/// let as_mut_slice: Bitset<&mut [u32]> = Bitset(&mut bunch_of_bits);
///
/// assert_eq!(
///     as_vec.ones_in_range(5..91),
///     as_slice.ones_in_range(5..91),
/// );
/// as_vec.enable_bit(11);
/// // They aren't equal anymore.
/// assert_ne!(
///     as_vec.ones_in_range(5..91),
///     as_slice.ones_in_range(5..91),
/// );
/// ```
///
/// [bitset]: https://en.wikipedia.org/wiki/Bit_array
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitset<B: AsRef<[u32]>>(pub B);

impl<B: AsRef<[u32]> + AsMut<[u32]>> Bitset<B> {
    /// Returns `None` if `bit` is out of range
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
    #[inline]
    pub fn bit(&self, at: usize) -> bool {
        let block = at / u32::BITS64;
        let offset = u32::try_from(at % u32::BITS64).unwrap();
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
            // TODO(perf): verify that this unwrap is always elided,
            // We `% 32` just before, so it should be fine.
            start: u32::try_from(start % u32::BITS64).unwrap(),
            end: u32::try_from(end % u32::BITS64).unwrap(),
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

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;
    //                            16v  32v     48v  64v     80v  96v
    const BLOCKS: [u32; 3] = [0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];

    fn blocks() -> Bitset<[u32; 3]> {
        Bitset(BLOCKS.map(|i| i.reverse_bits()))
    }

    #[test]
    fn empty_empty() {
        let blocks = Bitset([]);
        let actual: Vec<_> = blocks.ones_in_range(0..0).collect();
        let expected: &[u32] = &[];
        assert_eq!(expected, actual);
    }
    #[test]
    fn empty_range() {
        let blocks = blocks();

        let actual: Vec<_> = blocks.ones_in_range(17..17).collect();
        let expected: &[u32] = &[];
        assert_eq!(expected, actual);

        let actual: Vec<_> = blocks.ones_in_range(32..32).collect();
        assert_eq!(expected, actual);

        let actual: Vec<_> = blocks.ones_in_range(0..0).collect();
        assert_eq!(expected, actual);
    }
    #[test]
    fn same_block() {
        let blocks = blocks();

        let actual: Vec<_> = blocks.ones_in_range(16..31).collect();
        let expected: Vec<u32> = (24..31).collect();
        assert_eq!(&expected, &actual);

        let actual: Vec<_> = blocks.ones_in_range(16..32).collect();
        let expected: Vec<u32> = (24..32).collect();
        assert_eq!(&expected, &actual);

        let actual: Vec<_> = blocks.ones_in_range(64..80).collect();
        let expected: Vec<u32> = (64..76).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn both_unaligned() {
        let range = 24..76;
        let blocks = blocks();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (24..44).chain(60..76).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn first_unaligned() {
        let range = 24..64;
        let blocks = blocks();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (24..44).chain(60..64).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn last_unaligned() {
        let range = 32..76;
        let blocks = blocks();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (32..44).chain(60..76).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn none_unaligned() {
        let range = 32..64;
        let blocks = blocks();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (32..44).chain(60..64).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn full_range() {
        let range = 0..96;
        let blocks = blocks();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (0..4)
            .chain(8..12)
            .chain(24..44)
            .chain(60..76)
            .chain(84..88)
            .chain(92..96)
            .collect();
        assert_eq!(&expected, &actual);
    }
}
