use std::ops::Range;

trait BlockT {
    const BIT_COUNT: usize;
}
impl BlockT for u32 {
    const BIT_COUNT: usize = u32::BITS as usize;
}
type Block = u32;

/// Integer division rounded up.
const fn div_ceil(lhf: usize, rhs: usize) -> usize {
    (lhf + rhs - 1) / rhs
}

pub trait BitSetExtensions {
    fn ones_in_range(&self, range: Range<usize>) -> Ones;
}
impl BitSetExtensions for [Block] {
    fn ones_in_range(&self, range: Range<usize>) -> Ones {
        let Range { start, end } = range;
        // TODO(perf): this can probably be reduced sill;
        let crop = Range {
            start: (start % Block::BIT_COUNT) as u32,
            end: (end % Block::BIT_COUNT) as u32,
        };
        let range = Range {
            start: start / Block::BIT_COUNT + 1,
            end: div_ceil(end, Block::BIT_COUNT),
        };
        let remaining_blocks = &self[range.clone()];

        let mut bitset = self[range.start - 1];
        bitset &= ((1 << crop.start) - 1) ^ Block::MAX;
        if remaining_blocks.is_empty() && crop.end != 0 {
            bitset &= (1 << crop.end) - 1;
        }
        Ones {
            crop: crop.end,
            remaining_blocks,
            bitset,
            block_idx: u32::try_from(range.start - 1).unwrap(),
        }
    }
}

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
        Some(self.block_idx * (Block::BIT_COUNT as u32) + r)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;
    //                           16v  32v     48v  64v     80v  96v
    const BLOCKS: &[u32] = &[0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f];

    #[test]
    fn both_unaligned() {
        let range = 24..76;
        let blocks: Vec<_> = BLOCKS.iter().map(|i| i.reverse_bits()).collect();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (24..44).chain(60..76).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn first_unaligned() {
        let range = 24..64;
        let blocks: Vec<_> = BLOCKS.iter().map(|i| i.reverse_bits()).collect();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (24..44).chain(60..64).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn last_unaligned() {
        let range = 32..76;
        let blocks: Vec<_> = BLOCKS.iter().map(|i| i.reverse_bits()).collect();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (32..44).chain(60..76).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn none_unaligned() {
        let range = 32..64;
        let blocks: Vec<_> = BLOCKS.iter().map(|i| i.reverse_bits()).collect();
        let actual: Vec<_> = blocks.ones_in_range(range).collect();
        let expected: Vec<u32> = (32..44).chain(60..64).collect();
        assert_eq!(&expected, &actual);
    }
    #[test]
    fn full_range() {
        let range = 0..96;
        let blocks: Vec<_> = BLOCKS.iter().map(|i| i.reverse_bits()).collect();
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
