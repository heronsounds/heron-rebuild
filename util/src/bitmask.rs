use std::{cmp, ops};

const INDEX_MASKS_U8: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const INDEX_MASKS_U16: [u16; 16] = [
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];
#[rustfmt::skip]
const INDEX_MASKS_U32: [u32; 32] = [
    1, 1 << 1, 1 << 2, 1 << 3, 1 << 4, 1 << 5, 1 << 6, 1 << 7,
    1 << 8, 1 << 9, 1 << 10, 1 << 11, 1 << 12, 1 << 13, 1 << 14, 1 << 15,
    1 << 16, 1 << 17, 1 << 18, 1 << 19, 1 << 20, 1 << 21, 1 << 22, 1 << 23,
    1 << 24, 1 << 25, 1 << 26, 1 << 27, 1 << 28, 1 << 29, 1 << 30, 1 << 31,
];
#[rustfmt::skip]
const INDEX_MASKS_U64: [u64; 64] = [
    1, 1 << 1, 1 << 2, 1 << 3, 1 << 4, 1 << 5, 1 << 6, 1 << 7,
    1 << 8, 1 << 9, 1 << 10, 1 << 11, 1 << 12, 1 << 13, 1 << 14, 1 << 15,
    1 << 16, 1 << 17, 1 << 18, 1 << 19, 1 << 20, 1 << 21, 1 << 22, 1 << 23,
    1 << 24, 1 << 25, 1 << 26, 1 << 27, 1 << 28, 1 << 29, 1 << 30, 1 << 31,
    1 << 32, 1 << 33, 1 << 34, 1 << 35, 1 << 36, 1 << 37, 1 << 38, 1 << 39,
    1 << 40, 1 << 41, 1 << 42, 1 << 43, 1 << 44, 1 << 45, 1 << 46, 1 << 47,
    1 << 48, 1 << 49, 1 << 50, 1 << 51, 1 << 52, 1 << 53, 1 << 54, 1 << 55,
    1 << 56, 1 << 57, 1 << 58, 1 << 59, 1 << 60, 1 << 61, 1 << 62, 1 << 63,
];

/// Trait for types that can be used as the underlying type of a bitmask.
/// In practice, should only be implemented for unsigned int types.
// TODO: make a hierarchical version for sizes beyond 128...
pub trait Bitmask:
    Sized
    + 'static
    + Copy
    + cmp::PartialEq
    + ops::Shr<usize, Output = Self>
    + ops::BitOrAssign<Self>
    + ops::Not<Output = Self>
    + ops::BitAnd<Self, Output = Self>
    + ops::BitAndAssign<Self>
{
    /// Number of bits contained in this type
    const BITS: usize;

    /// Reference to the number one
    const ONE: Self;

    /// return true if the i'th bit is set
    #[inline]
    fn get(&self, i: usize) -> bool {
        (*self >> i) & Self::ONE == Self::ONE
    }

    /// set the i'th bit to true
    // NB this needs to be defined on the types themselves,
    // since we make use of power-of-2 lookup tables.
    fn set(&mut self, i: usize);
}

impl Bitmask for u8 {
    const BITS: usize = u8::BITS as usize;
    const ONE: Self = 1;
    #[inline]
    fn set(&mut self, i: usize) {
        *self |= INDEX_MASKS_U8[i]
    }
}

impl Bitmask for u16 {
    const BITS: usize = u16::BITS as usize;
    const ONE: Self = 1;
    #[inline]
    fn set(&mut self, i: usize) {
        *self |= INDEX_MASKS_U16[i]
    }
}

impl Bitmask for u32 {
    const BITS: usize = u32::BITS as usize;
    const ONE: Self = 1;
    #[inline]
    fn set(&mut self, i: usize) {
        *self |= INDEX_MASKS_U32[i]
    }
}

impl Bitmask for u64 {
    const BITS: usize = u64::BITS as usize;
    const ONE: Self = 1;
    #[inline]
    fn set(&mut self, i: usize) {
        *self |= INDEX_MASKS_U64[i]
    }
}

impl Bitmask for u128 {
    const BITS: usize = u128::BITS as usize;
    const ONE: Self = 1;
    // didn't want to bother w/ an index mask for this one:
    #[inline]
    fn set(&mut self, i: usize) {
        *self |= 1 << i
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_set() {
        let mut mask = 0u8;
        mask.set(1);
        assert_eq!(0b10, mask);
        let mut mask = 0u16;
        mask.set(1);
        assert_eq!(0b10, mask);
        let mut mask = 0u32;
        mask.set(1);
        assert_eq!(0b10, mask);
        let mut mask = 0u64;
        mask.set(1);
        assert_eq!(0b10, mask);
        let mut mask = 0u128;
        mask.set(1);
        assert_eq!(0b10, mask);
    }
    #[test]
    fn test_get() {
        let mask = 0b100u8;
        assert_eq!(mask.get(0), false);
        assert_eq!(mask.get(2), true);
        let mask = 0b100u16;
        assert_eq!(mask.get(0), false);
        assert_eq!(mask.get(2), true);
        let mask = 0b100u32;
        assert_eq!(mask.get(0), false);
        assert_eq!(mask.get(2), true);
        let mask = 0b100u64;
        assert_eq!(mask.get(0), false);
        assert_eq!(mask.get(2), true);
        let mask = 0b100u128;
        assert_eq!(mask.get(0), false);
        assert_eq!(mask.get(2), true);
    }
    #[test]
    fn test_mask_lookups() {
        for i in 0..8 {
            eprintln!("testing power of two {i}");
            assert_eq!(2u8.pow(i as u32), INDEX_MASKS_U8[i]);
            assert_eq!(2u16.pow(i as u32), INDEX_MASKS_U16[i]);
            assert_eq!(2u32.pow(i as u32), INDEX_MASKS_U32[i]);
            assert_eq!(2u64.pow(i as u32), INDEX_MASKS_U64[i]);
        }
        for i in 8..16 {
            eprintln!("testing power of two {i}");
            assert_eq!(2u16.pow(i as u32), INDEX_MASKS_U16[i]);
            assert_eq!(2u32.pow(i as u32), INDEX_MASKS_U32[i]);
            assert_eq!(2u64.pow(i as u32), INDEX_MASKS_U64[i]);
        }
        for i in 16..32 {
            eprintln!("testing power of two {i}");
            assert_eq!(2u32.pow(i as u32), INDEX_MASKS_U32[i]);
            assert_eq!(2u64.pow(i as u32), INDEX_MASKS_U64[i]);
        }
        for i in 32..64 {
            eprintln!("testing power of two {i}");
            assert_eq!(2u64.pow(i as u32), INDEX_MASKS_U64[i]);
        }
    }
}
