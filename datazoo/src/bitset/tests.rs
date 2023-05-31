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
