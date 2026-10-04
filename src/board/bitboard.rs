use crate::util;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// Returns true if the specified bit is set.
    #[inline(always)]
    pub fn is_set(&self, square: u8) -> bool {
        self.0 & (1 << square) != 0
    }

    /// Sets the specified bit to true.
    #[inline(always)]
    pub const fn set(&mut self, square: u8) {
        self.0 |= 1u64 << square;
    }

    /// Clears all bits in the bitboard.
    #[inline(always)]
    pub fn clear(&mut self, square: u8) {
        self.0 &= !(1 << square);
    }

    /// Finds, clears, and returns the index of the Least Significant Bit set to 1.
    #[inline(always)]
    pub fn pop_lsb(&mut self) -> u8 {
        let index = self.0.trailing_zeros() as u8;
        if index == 64 {
            return 64;
        }
        self.0 &= self.0.wrapping_sub(1);
        index
    }

    /// Returns the index of the Least Significant Bit set to 1.
    #[inline(always)]
    pub fn get_lsb(self) -> u8 {
        self.0.trailing_zeros() as u8
    }

    /// Returns true if the bitboard is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn print(&self) {
        println!("  +-------------------+");
        // Loop from rank 7 down to 0 (Rank 8 down to Rank 1)
        for rank in (0..8).rev() {
            print!("{} | ", rank + 1);
            for file in 0..8 {
                let square = util::rank_file_to_square(rank, file);
                if self.is_set(square) {
                    print!("1 ");
                } else {
                    print!(". ");
                }
            }
            println!("|");
        }
        println!("  +-------------------+");
        println!("    a b c d e f g h\n");
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}
