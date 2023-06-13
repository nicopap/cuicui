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
#[test]
fn u32_at() {
    let bitset = Bitset(&[0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f]);

    let (at, expected) = (bitset.u32_at(0).unwrap(), 0xf0f0_00ff);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.u32_at(4).unwrap(), 0xff0f_000f);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.u32_at(16).unwrap(), 0x000f_f0f0);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.u32_at(64).unwrap(), 0xfff0_0f0f);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    assert_eq!(bitset.u32_at(65).ok(), None);
    assert_eq!(bitset.u32_at(96).ok(), None);

    let bitset = Bitset(&[u32::MAX, u32::MAX, u32::MAX]);

    assert_eq!(bitset.u32_at(0).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(1).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(2).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(7).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(16).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(64).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(31).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(32).unwrap(), u32::MAX);
    assert_eq!(bitset.u32_at(33).unwrap(), u32::MAX);

    assert_eq!(bitset.u32_at(65).ok(), None);
    assert_eq!(bitset.u32_at(96).ok(), None);
}
#[test]
fn n_at() {
    // =======
    // 32 bits
    // =======

    let bitset = Bitset(&[0xf0f0_00ff, 0xfff0_000f, 0xfff0_0f0f]);

    let (at, expected) = (bitset.n_at(32, 0).unwrap(), 0xf0f0_00ff);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.n_at(32, 4).unwrap(), 0xff0f_000f);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.n_at(32, 16).unwrap(), 0x000f_f0f0);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    let (at, expected) = (bitset.n_at(32, 64).unwrap(), 0xfff0_0f0f);
    assert_eq!(at, expected, "left: {at:08x}, right: {expected:08x}");

    assert_eq!(bitset.n_at(32, 65), None);
    assert_eq!(bitset.n_at(32, 96), None);

    let bitset = Bitset(&[u32::MAX, u32::MAX, u32::MAX]);

    assert_eq!(bitset.n_at(32, 0).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 1).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 2).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 7).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 16).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 64).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 31).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 32).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(32, 33).unwrap(), u32::MAX);

    assert_eq!(bitset.n_at(32, 65), None);
    assert_eq!(bitset.n_at(32, 96), None);

    // ================
    // more interesting
    // ================

    assert_eq!(bitset.n_at(96, 0).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(95, 1).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(90, 5).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(90, 7), None);
    assert_eq!(bitset.n_at(65, 31).unwrap(), u32::MAX);
    assert_eq!(bitset.n_at(64, 32).unwrap(), u32::MAX);
}
#[test]
fn disable_range() {
    use std::ops::Not;
    let mut bitset = Bitset(vec![0xffff_ffff, 0xffff_ffff, 0xffff_ffff]);

    assert!(bitset.bit(0));
    assert!(bitset.bit(10));
    assert!(bitset.bit(15));
    assert!(bitset.bit(16));
    assert!(bitset.bit(32));
    assert!(bitset.bit(35));
    assert!(bitset.bit(53));
    assert!(bitset.bit(54));
    assert!(bitset.bit(64));
    assert!(bitset.bit(73));

    bitset.disable_range(0..16);
    bitset.disable_range(35..54);

    assert!(bitset.bit(0).not());
    assert!(bitset.bit(10).not());
    assert!(bitset.bit(15).not());
    assert!(bitset.bit(16));
    assert!(bitset.bit(32));
    assert!(bitset.bit(35).not());
    assert!(bitset.bit(53).not());
    assert!(bitset.bit(54));
    assert!(bitset.bit(64));
    assert!(bitset.bit(73));
}
