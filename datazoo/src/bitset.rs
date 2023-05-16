use std::ops::Range;

use super::div_ceil;

trait BlockT {
    const BIT_COUNT: usize;
}
impl BlockT for u32 {
    const BIT_COUNT: usize = u32::BITS as usize;
}
type Block = u32;

#[derive(Debug, Clone, Copy, Default)]
pub struct Bitset<T: AsRef<[u32]> + AsMut<[u32]>>(pub T);

impl<T: AsRef<[u32]> + AsMut<[u32]>> Bitset<T> {
    pub fn bit_len(&self) -> usize {
        self.0.as_ref().len() * Block::BIT_COUNT
    }
    /// Returns `None` if `bit` is out of range
    pub fn enable_bit(&mut self, bit: usize) -> Option<()> {
        let block = bit / Block::BIT_COUNT;
        let offset = bit % Block::BIT_COUNT;

        self.0.as_mut().get_mut(block).map(|block| {
            *block |= 1 << offset;
        })
    }
    pub fn ones_in_range(&self, range: Range<usize>) -> Ones {
        let Range { start, end } = range;
        assert!(start <= self.bit_len());
        assert!(end <= self.bit_len());

        // the offset to "crop" the bits at the edges of the [u32]
        let crop = Range {
            // TODO(perf): verify that this unwrap is always elided,
            // We `% 32` just before, so it should be fine.
            start: u32::try_from(start % Block::BIT_COUNT).unwrap(),
            end: u32::try_from(end % Block::BIT_COUNT).unwrap(),
        };
        // The indices of Blocks of [u32] (ie: NOT bits) affected by range
        let range = Range {
            start: start / Block::BIT_COUNT,
            end: div_ceil(end, Block::BIT_COUNT),
        };
        let all_blocks = &self.0.as_ref()[range.clone()];

        let (mut bitset, remaining_blocks) = all_blocks
            .split_first()
            .map_or((0, all_blocks), |(b, r)| (*b, r));

        bitset &= ((1 << crop.start) - 1) ^ Block::MAX;
        if remaining_blocks.is_empty() && crop.end != 0 {
            bitset &= (1 << crop.end) - 1;
        }
        Ones {
            crop: crop.end,
            remaining_blocks,
            bitset,
            block_idx: u32::try_from(range.start).unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ones<'a> {
    bitset: Block,
    /// Index in Block of `bitset`.
    block_idx: u32,
    /// How many bits to keep in the last block.
    crop: u32,
    remaining_blocks: &'a [Block],
}
impl<'a> Iterator for Ones<'a> {
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
        Some(self.block_idx * Block::BITS + r)
    }
}
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
