//! # 位图模块
//!
//! 这个模块提供了一个通用的 [`Bitmap`] 结构，用于高效地进行位操作。
//!
//! [`Bitmap`] 由一个实现了 [`BitStorage`] trait 的泛型整数类型（如 `u8`, `u16`, `u32`, `u64`, `u128`）支持。
//! 它封装了底层的位运算，提供了创建、修改、查询位图以及在置位 (1) 和未置位 (0) 的位上进行迭代的功能。
//!
//! ## 主要功能
//!
//! - **泛型实现**: 可以使用任何常见的无符号整数作为底层存储。
//! - **完整的位运算**: 支持 `&`, `|`, `^`, `!` 等所有标准位运算符。
//! - **迭代器**: 提供 [`PositiveIter`] 和 [`NegativeIter`]，分别用于遍历值为 1 和 0 的位的索引。
//! - **丰富的 API**: 包含 [`set`](Bitmap::set), [`get`](Bitmap::get), [`count_ones`](Bitmap::count_ones), [`any`](Bitmap::any), [`all`](Bitmap::all), [`none`](Bitmap::none) 等常用方法。
//!
//! ## 示例
//!
//! ```
//! use crab_vault_utils::bitmap::{Bitmap, BitStorage};
//!
//! // 使用 u32 作为存储，创建一个 32 位的位图
//! let mut artists = Bitmap::<u32>::new();
//!
//! // 将索引为 2, 8, 9 的位设置为 1
//! artists.set(2, true);
//! artists.set(8, true);
//! artists.set(9, true);
//!
//! // 检查索引为 8 的位是否为 1
//! assert!(artists.get(8));
//! // 检查索引为 5 的位是否为 0
//! assert!(!artists.get(5));
//!
//! // 计算有多少个位被设置了
//! assert_eq!(artists.count_ones(), 3);
//!
//! // 使用迭代器收集所有值为 1 的位的索引
//! let set_bits: Vec<usize> = artists.iter().collect();
//! assert_eq!(set_bits, vec![2, 8, 9]);
//!
//! // 创建另一个位图并进行合并
//! let mut other_artists = Bitmap::<u32>::new();
//! other_artists.set(9, true);
//! other_artists.set(15, true);
//!
//! let all_artists = artists | other_artists;
//! let expected_bits: Vec<usize> = all_artists.iter().collect();
//! assert_eq!(expected_bits, vec![2, 8, 9, 15]);
//! ```

use std::fmt::Debug;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, Shr};

pub trait BitStorage:
    Copy
    + Default
    + Debug
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Not<Output = Self>
    + From<u8>
    + Shl<usize, Output = Self>
    + Shr<usize, Output = Self>
    + PartialEq
    + Eq
{
    const BITS: usize;
    fn trailing_zeros(self) -> u32;
    fn count_ones(self) -> u32;
    fn count_zeros(self) -> u32;
}

macro_rules! impl_bit_storage_for_type {
    ($($t:ty),*) => {
        $(
            impl BitStorage for $t {
                const BITS: usize = std::mem::size_of::<$t>() * 8;

                #[inline]
                fn trailing_zeros(self) -> u32 {
                    self.trailing_zeros()
                }

                #[inline]
                fn count_ones(self) -> u32 {
                    self.count_ones()
                }

                #[inline]
                fn count_zeros(self) -> u32 {
                    self.count_zeros()
                }
            }
        )*
    };
}

impl_bit_storage_for_type!(u8, u16, u32, u64, u128);

/// 一个通用的位图结构，由一个实现了 [`BitStorage`] 的类型支持。
///
/// # 示例
/// ```
/// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
/// // 使用 u16 作为存储类型，总共 16 位
/// let mut bitmap = Bitmap::<u16>::new();
/// bitmap.set(3, true);
/// bitmap.set(5, true);
///
/// assert!(bitmap.get(3));
/// assert!(!bitmap.get(4));
///
/// let positions: Vec<usize> = bitmap.iter().collect();
/// assert_eq!(positions, vec![3, 5]);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitmap<T: BitStorage> {
    val: T,
}

/// 一个迭代器，用于遍历位图中所有值为 1 (positive) 的位索引。
pub struct PositiveIter<T: BitStorage> {
    bitmap: Bitmap<T>,
}

/// 一个迭代器，用于遍历位图中所有值为 0 (negative) 的位索引。
pub struct NegativeIter<T: BitStorage> {
    bitmap: Bitmap<T>,
}

impl<T: BitStorage> From<NegativeIter<T>> for PositiveIter<T> {
    fn from(value: NegativeIter<T>) -> Self {
        let NegativeIter { bitmap } = value;
        Self { bitmap }
    }
}

impl<T: BitStorage> From<PositiveIter<T>> for NegativeIter<T> {
    fn from(value: PositiveIter<T>) -> Self {
        let PositiveIter { bitmap } = value;
        Self { bitmap }
    }
}

impl<T: BitStorage> Iterator for PositiveIter<T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitmap.val == T::from(0) {
            return None;
        }

        let next_bit_pos = self.bitmap.val.trailing_zeros() as usize;

        // 清除刚刚找到的位，以便下一次迭代
        let mask = T::from(1) << next_bit_pos;
        self.bitmap.val &= !mask;
        Some(next_bit_pos)
    }
}

impl<T: BitStorage> Iterator for NegativeIter<T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // 如果所有位都为 1，则没有 0 可以迭代
        if self.bitmap.val == !T::from(0) {
            return None;
        }

        let next_bit_pos = (!self.bitmap.val).trailing_zeros() as usize;

        // 设置刚刚找到的位为 1，以便下一次迭代
        let mask = T::from(1) << next_bit_pos;
        self.bitmap.val |= mask;
        Some(next_bit_pos)
    }
}

impl<T: BitStorage> PositiveIter<T> {
    /// 将正迭代器（遍历 1）转换为负迭代器（遍历 0）。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(1, true);
    /// bitmap.set(3, true);
    ///
    /// let positive_iter = bitmap.iter();
    /// let negative_iter = positive_iter.invert();
    /// let zeros: Vec<usize> = negative_iter.collect();
    ///
    /// assert_eq!(zeros, vec![0, 2, 4, 5, 6, 7]);
    /// ```
    #[inline]
    pub fn invert(self) -> NegativeIter<T> {
        self.into()
    }
}

impl<T: BitStorage> NegativeIter<T> {
    /// 将负迭代器（遍历 0）转换为正迭代器（遍历 1）。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new_full();
    /// bitmap.set(1, false);
    /// bitmap.set(3, false);
    ///
    /// let negative_iter = bitmap.iter_zeros();
    /// let positive_iter = negative_iter.invert();
    /// let ones: Vec<usize> = positive_iter.collect();
    ///
    /// assert_eq!(ones, vec![0, 2, 4, 5, 6, 7]);
    /// ```
    #[inline]
    pub fn invert(self) -> PositiveIter<T> {
        self.into()
    }
}

impl<'a, T: BitStorage> IntoIterator for &'a Bitmap<T> {
    type Item = usize;
    type IntoIter = PositiveIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: BitStorage> IntoIterator for Bitmap<T> {
    type Item = usize;
    type IntoIter = PositiveIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: BitStorage> From<T> for Bitmap<T> {
    #[inline]
    fn from(val: T) -> Self {
        Self { val }
    }
}

impl<T: BitStorage> Bitmap<T> {
    /// 创建一个所有位都为 0 的空位图。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let bitmap = Bitmap::<u16>::new_empty();
    /// assert!(bitmap.none());
    /// assert_eq!(bitmap.count_ones(), 0);
    /// ```
    #[inline]
    pub fn new_empty() -> Self {
        Self { val: T::from(0) }
    }

    /// 创建一个所有位都为 1 的全满位图。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let bitmap = Bitmap::<u8>::new_full();
    /// assert!(bitmap.all());
    /// assert_eq!(bitmap.count_ones(), 8);
    /// ```
    #[inline]
    pub fn new_full() -> Self {
        Self { val: !T::from(0) }
    }

    /// 创建一个空的位图，是 `new_empty` 的别名。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let bitmap = Bitmap::<u32>::new();
    /// assert!(bitmap.none());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self::new_empty()
    }

    /// 返回一个迭代器，用于遍历所有值为 1 的位的索引。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(2, true);
    /// bitmap.set(6, true);
    /// let ones: Vec<usize> = bitmap.iter().collect();
    /// assert_eq!(ones, vec![2, 6]);
    /// ```
    #[inline]
    pub fn iter(&self) -> PositiveIter<T> {
        PositiveIter { bitmap: *self }
    }

    /// 返回一个迭代器，用于遍历所有值为 0 的位的索引。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(0, true);
    /// bitmap.set(1, true);
    /// bitmap.set(2, true);
    /// bitmap.set(3, true);
    /// // 内部值为 ...00001111
    /// let zeros: Vec<usize> = bitmap.iter_zeros().collect();
    /// assert_eq!(zeros, vec![4, 5, 6, 7]);
    /// ```
    #[inline]
    pub fn iter_zeros(&self) -> NegativeIter<T> {
        NegativeIter { bitmap: *self }
    }

    /// 设置指定索引的位。
    ///
    /// `true` 表示设置为 1，`false` 表示设置为 0。
    ///
    /// # Panics
    ///
    /// 如果 `idx` 超出位图的范围（`idx >= T::BITS`），在调试模式下会触发 panic。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(5, true);
    /// assert!(bitmap.get(5));
    /// bitmap.set(5, false);
    /// assert!(!bitmap.get(5));
    /// ```
    #[inline]
    pub fn set(&mut self, idx: usize, set: bool) {
        debug_assert!(idx < T::BITS, "Index out of bounds");
        let mask = T::from(1) << idx;
        if set {
            self.val |= mask;
        } else {
            self.val &= !mask;
        }
    }

    /// 获取指定索引的位的值。
    ///
    /// 返回 `true` 如果该位为 1，否则返回 `false`。
    ///
    /// # Panics
    ///
    /// 如果 `idx` 超出位图的范围（`idx >= T::BITS`），在调试模式下会触发 panic。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(7, true);
    /// assert_eq!(bitmap.get(7), true);
    /// assert_eq!(bitmap.get(0), false);
    /// ```
    #[inline]
    pub fn get(&self, idx: usize) -> bool {
        debug_assert!(idx < T::BITS, "Index out of bounds");
        let mask = T::from(1) << idx;
        (self.val & mask) != T::from(0)
    }

    /// 检查指定索引的位是否为 1。`get` 的别名。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(1, true);
    /// assert!(bitmap.is_true_on(1));
    /// ```
    #[inline]
    pub fn is_true_on(&self, idx: usize) -> bool {
        self.get(idx)
    }

    /// 检查指定索引的位是否为 0。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(1, true);
    /// assert!(bitmap.is_false_on(0));
    /// ```
    #[inline]
    pub fn is_false_on(&self, idx: usize) -> bool {
        !self.get(idx)
    }

    /// 将两个位图进行合并（并集），等同于 `|` 按位或操作。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut b1 = Bitmap::<u8>::new();
    /// b1.set(1, true); // 00000010
    /// let mut b2 = Bitmap::<u8>::new();
    /// b2.set(2, true); // 00000100
    ///
    /// let merged = b1.merge(b2); // 00000110
    /// assert!(merged.get(1));
    /// assert!(merged.get(2));
    /// ```
    #[inline]
    pub fn merge(self, rhs: Bitmap<T>) -> Bitmap<T> {
        self | rhs
    }

    /// 计算值为 1 的位的数量。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(0, true);
    /// bitmap.set(2, true);
    /// bitmap.set(4, true);
    /// assert_eq!(bitmap.count_ones(), 3);
    /// ```
    #[inline]
    pub fn count_ones(&self) -> u32 {
        self.val.count_ones()
    }

    /// 计算值为 0 的位的数量。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u8>::new();
    /// bitmap.set(0, true);
    /// bitmap.set(2, true);
    /// bitmap.set(4, true);
    /// // u8 有 8 位，3 个是 1，所以 5 个是 0
    /// assert_eq!(bitmap.count_zeros(), 5);
    /// ```
    #[inline]
    pub fn count_zeros(&self) -> u32 {
        T::BITS as u32 - self.val.count_ones()
    }

    /// 检查位图中是否至少有一个位是 1。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut b1 = Bitmap::<u8>::new();
    /// b1.set(3, true);
    /// assert!(b1.any());
    ///
    /// let b2 = Bitmap::<u8>::new();
    /// assert!(!b2.any());
    /// ```
    #[inline]
    pub fn any(&self) -> bool {
        self.val != T::from(0)
    }

    /// 检查位图中是否所有位都是 1。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let b1 = Bitmap::<u8>::new_full();
    /// assert!(b1.all());
    ///
    /// let mut b2 = Bitmap::<u8>::new_full();
    /// b2.set(4, false);
    /// assert!(!b2.all());
    /// ```
    #[inline]
    pub fn all(&self) -> bool {
        self.val == !T::from(0)
    }

    /// 检查位图中是否所有位都是 0。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let b1 = Bitmap::<u8>::new();
    /// assert!(b1.none());
    ///
    /// let mut b2 = Bitmap::<u8>::new();
    /// b2.set(0, true);
    /// assert!(!b2.none());
    /// ```
    #[inline]
    pub fn none(&self) -> bool {
        self.val == T::from(0)
    }

    /// 查找第一个值为 1 的位的索引。
    ///
    /// 如果所有位都为 0，则返回 `None`。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut bitmap = Bitmap::<u16>::new();
    /// bitmap.set(5, true);
    /// bitmap.set(10, true);
    /// assert_eq!(bitmap.first_one(), Some(5));
    ///
    /// let empty_bitmap = Bitmap::<u16>::new();
    /// assert_eq!(empty_bitmap.first_one(), None);
    /// ```
    #[inline]
    pub fn first_one(&self) -> Option<usize> {
        if self.none() {
            None
        } else {
            Some(self.val.trailing_zeros() as usize)
        }
    }
}

impl<T: BitStorage> BitAnd for Bitmap<T> {
    type Output = Self;
    /// 按位与（&）。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut b1 = Bitmap::<u8>::from(0b__0000_1101); // 位 0, 2, 3
    /// let mut b2 = Bitmap::<u8>::from(0b__0000_1011); // 位 0, 1, 3
    /// let result = b1 & b2;
    /// let expected = Bitmap::<u8>::from(0b__0000_1001); // 位 0, 3
    /// assert_eq!(result, expected);
    /// ```
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            val: self.val & rhs.val,
        }
    }
}

impl<T: BitStorage> BitOr for Bitmap<T> {
    type Output = Self;
    /// 按位或（|）。
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut b1 = Bitmap::<u8>::from(0b__0000_1101); // 位 0, 2, 3
    /// let mut b2 = Bitmap::<u8>::from(0b__0000_1011); // 位 0, 1, 3
    /// let result = b1 | b2;
    /// let expected = Bitmap::<u8>::from(0b__0000_1111); // 位 0, 1, 2, 3
    /// assert_eq!(result, expected);
    /// ```
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            val: self.val | rhs.val,
        }
    }
}

impl<T: BitStorage> BitXor for Bitmap<T> {
    type Output = Self;
    /// 按位异或（^）
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let mut b1 = Bitmap::<u8>::from(0b__0000_1101); // 位 0, 2, 3
    /// let mut b2 = Bitmap::<u8>::from(0b__0000_1011); // 位 0, 1, 3
    /// let result = b1 ^ b2;
    /// let expected = Bitmap::<u8>::from(0b__0000_0110); // 位 1, 2
    /// assert_eq!(result, expected);
    /// ```
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            val: self.val ^ rhs.val,
        }
    }
}

impl<T: BitStorage> Not for Bitmap<T> {
    type Output = Self;
    /// 按位取反（!）
    ///
    /// # 示例
    /// ```
    /// # use crab_vault_utils::bitmap::{Bitmap, BitStorage};
    /// let b = Bitmap::<u8>::from(0b__1111_0000);
    /// let result = !b;
    /// let expected = Bitmap::<u8>::from(0b__0000_1111);
    /// assert_eq!(result, expected);
    /// ```
    fn not(self) -> Self::Output {
        Self { val: !self.val }
    }
}
