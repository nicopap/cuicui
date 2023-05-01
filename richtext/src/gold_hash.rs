use std::hash::{BuildHasher, Hasher};

use bevy::utils::hashbrown;

pub type GoldMap<K, V> = hashbrown::HashMap<K, V, GoldHash>;

/// A [`BuildHasher`] that results in a [`GoldHasher`].
#[derive(Default)]
pub struct GoldHash;

impl BuildHasher for GoldHash {
    type Hasher = GoldHasher;

    fn build_hasher(&self) -> Self::Hasher {
        GoldHasher::default()
    }
}

/// The golden ratio
const PHI: f64 = 1.618033988749895;
const UPHI: u64 = ((u64::MAX as f64) / PHI) as u64 - 1;
const SHIFT: u32 = 32;

/// A hash that only works on `u64`s, multiplying by the golden ratio.
///
/// A passthrough hash is fine but will have poor performance on smaller tables
/// due to collisions, especially if the lower bits do not tend to change,
/// As the hash is cut to fit the size of the hash table, generally discarding
/// the upper bits.
///
/// Shufling about the bits help reduce collision when discarding upper bits.
///
/// See https://probablydance.com/2018/06/16/fibonacci-hashing-the-optimization-that-the-world-forgot-or-a-better-alternative-to-integer-modulo/
#[derive(Debug, Default)]
pub struct GoldHasher {
    hash: u64,
}

impl Hasher for GoldHasher {
    fn write(&mut self, _bytes: &[u8]) {
        panic!("can only hash u64 using GoldHasher");
    }

    #[inline]
    fn write_u64(&mut self, mut i: u64) {
        i ^= i >> SHIFT;
        self.hash = self.hash.wrapping_mul(UPHI) ^ i.wrapping_mul(UPHI) >> SHIFT;
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.write_u64((i & (u64::MAX as u128)) as u64);
        self.write_u64((i << u64::BITS & (u64::MAX as u128)) as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}
