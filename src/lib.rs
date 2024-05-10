use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
use std::{iter, ptr, slice};

pub mod raw;
#[cfg(feature = "serde")]
mod serialize;
/// The prelude of traits and objects which are generally useful.
pub mod prelude {
    pub use super::TwoSidedVec;
}

use self::raw::{Capacity, CapacityRequest, RawTwoSidedVec};

/// Internal macro used to count the number of expressions passed to the `two_sided_vec!` macro.
#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! count_exprs {
    () => (0);
    ($one:expr) => (1);
    ($first:expr, $($value:expr),*) => ($crate::count_exprs!($($value),*) + 1)
}
/// Creates a `TwoSidedVec` from the specified elements.
///
/// ## Examples
/// ````
/// # #[macro_use] extern crate two_sided_vec;
/// # fn main() {
/// let example = two_sided_vec![1, 2, 3; 4, 5, 6];
/// assert_eq!(example.back(), &[1, 2, 3]);
/// assert_eq!(example.front(), &[4, 5, 6]);
/// let example = two_sided_vec![4, 5, 6];
/// assert_eq!(example.back(), &[]);
/// assert_eq!(example.front(), &[4, 5, 6]);
/// # }
/// ````
#[macro_export]
macro_rules! two_sided_vec {
    () => (TwoSidedVec::new());
    ($($element:expr),*) => (two_sided_vec![; $($element),*]);
    ($($back:expr),*; $($front:expr),*) =>  {{
        let mut result = $crate::TwoSidedVec::with_capacity(
            $crate::count_exprs!($($back),*),
            $crate::count_exprs!($($front),*)
        );
        $(result.push_back($back);)*
        if !result.back().is_empty() {
            result.back_mut().reverse();
        }
        $(result.push_front($front);)*
        result
    }}
}

impl<T> Default for RawTwoSidedVec<T> {
    fn default() -> Self {
        RawTwoSidedVec::new()
    }
}
/// A simple 'two sided' vector, that can grow both forwards and backwards.
///
/// The front and the back can be viewed as seperate and independent vectors,
/// with negative indexing accessing the back and positive indexing accessing the front.
/// This allows you to **append to the back without modifying positive indexes**.
/// Unless you actually need pushing to the back to appear to shift the front forward,
/// like `VecDeque` does, this negative index system will probably be better for your situation.
///
/// Internally this allows a much simpler and faster implementation,
/// since there's only a single pointer to the middle that grows up and down.
/// Internally, we have to reallocate the buffer if we run out of capacity in either the
/// negative or positive difes grow separately and
/// Although bounds checks are _slightly_ slower since they involve two comparisons,
/// the access itself should be just as fast.
pub struct TwoSidedVec<T> {
    memory: RawTwoSidedVec<T>,
    start_index: isize,
    end_index: isize,
}
impl<T> TwoSidedVec<T> {
    #[inline]
    pub fn new() -> Self {
        unsafe { TwoSidedVec::from_raw(RawTwoSidedVec::new()) }
    }
    #[inline]
    pub fn with_capacity(back: usize, front: usize) -> Self {
        unsafe { TwoSidedVec::from_raw(RawTwoSidedVec::with_capacity(Capacity { back, front })) }
    }
    #[inline]
    unsafe fn from_raw(memory: RawTwoSidedVec<T>) -> Self {
        TwoSidedVec {
            memory,
            start_index: 0,
            end_index: 0,
        }
    }
    /// Take a slice of the front of this queue
    #[inline]
    pub fn front(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.middle_ptr(), self.len_front()) }
    }
    /// Take a slice of the back of this queue
    #[inline]
    pub fn back(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.start_ptr(), self.len_back()) }
    }
    /// Take a mutable slice of the front of this queue
    #[inline]
    pub fn front_mut(&mut self) -> &mut [T] {
        self.split_mut().1
    }
    /// Take a mutable slice of the back of this queue
    #[inline]
    pub fn back_mut(&mut self) -> &mut [T] {
        self.split_mut().0
    }
    /// Take seperate slices of the back and the front of the vector respectively.
    #[inline]
    pub fn split(&self) -> (&[T], &[T]) {
        (self.back(), self.front())
    }
    /// Take seperate mutable slices of the back and front of the vector respectively.
    #[inline]
    pub fn split_mut(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            (
                slice::from_raw_parts_mut(self.start_ptr(), self.len_back()),
                slice::from_raw_parts_mut(self.middle_ptr(), self.len_front()),
            )
        }
    }
    #[inline]
    pub fn push_front(&mut self, value: T) {
        self.reserve_front(1);
        unsafe {
            ptr::write(self.end_ptr(), value);
            self.end_index += 1;
        }
    }
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        if self.end_index > 0 {
            self.end_index -= 1;
            unsafe { Some(ptr::read(self.end_ptr())) }
        } else {
            None
        }
    }
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        if self.start_index < 0 {
            self.start_index += 1;
            unsafe { Some(ptr::read(self.middle_ptr().offset(self.start_index - 1))) }
        } else {
            None
        }
    }
    /// Push the specified value into the front of this queue,
    /// without modifying its `end` or touching the front of the queue.
    ///
    /// This effectively **preserves all positive indexes**,
    /// which may or may not be useful for your situation.
    #[inline]
    pub fn push_back(&mut self, value: T) {
        self.reserve_back(1);
        unsafe {
            ptr::write(self.start_ptr().offset(-1), value);
            self.start_index -= 1;
        }
    }
    fn default_extend_back<I: Iterator<Item = T>>(&mut self, iter: I) {
        if let Some(hint) = iter.size_hint().1 {
            self.reserve_back(hint)
        };
        for value in iter {
            self.push_back(value);
        }
    }
    fn default_extend_front<I: Iterator<Item = T>>(&mut self, iter: I) {
        if let Some(hint) = iter.size_hint().1 {
            self.reserve_front(hint)
        };
        for value in iter {
            self.push_front(value);
        }
    }

    #[inline]
    pub fn reserve_back(&mut self, amount: usize) {
        debug_assert!(self.check_sanity());
        if !self.can_fit_back(amount) {
            self.grow(0, amount);
        }
        debug_assert!(self.can_fit_back(amount))
    }
    #[inline]
    pub fn reserve_front(&mut self, amount: usize) {
        debug_assert!(self.check_sanity());
        if !self.can_fit_front(amount) {
            self.grow(amount, 0);
        }
        debug_assert!(
            self.can_fit_front(amount),
            "Insufficient capacity {:?} for {} additional elements. end_index = {}, start_index = {}, len = {}",
            self.memory.capacity(), amount, self.end_index, self.start_index, self.len()
        );
    }
    #[cold]
    #[inline(never)]
    fn grow(&mut self, front: usize, back: usize) {
        let request = CapacityRequest {
            used: Capacity {
                back: self.len_back(),
                front: self.len_front(),
            },
            needed: Capacity { front, back },
        };
        self.memory.reserve(request);
    }
    #[inline]
    fn can_fit_front(&self, amount: usize) -> bool {
        let remaining_front = self.capacity_front() - self.len_front();
        remaining_front >= amount
    }
    #[inline]
    fn can_fit_back(&self, amount: usize) -> bool {
        let remaining_back = self.capacity_back() - self.len_back();
        remaining_back >= amount
    }
    #[inline]
    pub fn capacity_back(&self) -> usize {
        self.memory.capacity().back
    }
    #[inline]
    pub fn capacity_front(&self) -> usize {
        self.memory.capacity().front
    }
    /// Return the length of the entire vector, which is the sum of the
    /// lengths of the front and back parts.
    ///
    /// The **length isn't where the vector ends**,
    /// since it could have elements in the back with negative indexes.
    /// Use `vec.start()` and `vec.end()` if you want to know the start and end indexes.
    /// The total length is exactly equivalent to `vec.len_back() + vec.len_front()`
    #[inline]
    pub fn len(&self) -> usize {
        debug_assert!(self.start_index <= self.end_index);
        self.end_index.wrapping_sub(self.start_index) as usize
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start_index == self.end_index
    }
    /// Return the length of the back of the vector.
    #[inline]
    pub fn len_back(&self) -> usize {
        debug_assert!(self.start_index <= 0);
        // NOTE: We perform the cast immediately after the negation to handle overflow properly
        self.start_index.wrapping_neg() as usize
    }
    /// Return the length of the front of the vector
    #[inline]
    pub fn len_front(&self) -> usize {
        debug_assert!(self.end_index >= 0);
        self.end_index as usize
    }
    /// Give the (inclusive) start of the queue's elements.
    /// which may be negative if the queue's back isn't empty
    ///
    /// This is exactly equivelant to `-vec.back().len()`.
    #[inline]
    pub fn start(&self) -> isize {
        self.start_index
    }
    /// Give the (exclusive) end of the queue's elements,
    /// which may be less than the length if the queue's back contains some elements.
    ///
    /// This is exactly equivalent to `vec.front().len()`
    #[inline]
    pub fn end(&self) -> isize {
        self.end_index
    }
    /// Return the `[start, end)` range of the element indices,
    /// equivalent to a tuple of `(queue.start(), queue.end())`.
    #[inline]
    pub fn range(&self) -> Range<isize> {
        self.start_index..self.end_index
    }
    /// Iterate over the entire vector, including both the back and front.
    #[inline]
    pub fn iter_entire(&self) -> slice::Iter<T> {
        self[..].iter()
    }
    #[inline]
    pub fn get<I: TwoSidedIndex<T>>(&self, index: I) -> Option<&I::Output> {
        index.get(self)
    }
    #[inline]
    pub fn get_mut<I: TwoSidedIndex<T>>(&mut self, index: I) -> Option<&mut I::Output> {
        index.get_mut(self)
    }
    /// Get a reference to value at the specified index
    ///
    /// ## Safety
    /// Undefined behavior if the index is out of bounds
    #[inline]
    pub unsafe fn get_unchecked<I: TwoSidedIndex<T>>(&self, index: I) -> &I::Output {
        index.get_unchecked(self)
    }

    /// Get a mutable reference to value at the specified index
    ///
    /// ## Safety
    /// Undefined behavior if the index is out of bounds
    #[inline]
    pub unsafe fn get_unchecked_mut<I: TwoSidedIndex<T>>(&mut self, index: I) -> &mut I::Output {
        index.get_unchecked_mut(self)
    }
    /// Give a raw pointer to the start of the elements
    #[inline]
    pub fn start_ptr(&self) -> *mut T {
        unsafe { self.middle_ptr().offset(self.start_index) }
    }
    /// Give a raw pointer to the middle of the elements
    #[inline]
    pub fn middle_ptr(&self) -> *mut T {
        self.memory.middle()
    }
    #[inline]
    pub fn end_ptr(&self) -> *mut T {
        unsafe { self.middle_ptr().offset(self.end_index) }
    }
    #[inline]
    pub fn split_at(&self, index: isize) -> (&[T], &[T]) {
        (&self[..index], &self[index..])
    }
    fn check_sanity(&self) -> bool {
        assert!(self.start_index <= 0 && self.end_index >= 0);
        // These should be implied by the other checks
        debug_assert!(self.start_ptr() <= self.middle_ptr());
        debug_assert!(self.end_ptr() >= self.middle_ptr());
        true
    }
    pub fn clear(&mut self) {
        while let Some(value) = self.pop_back() {
            drop(value)
        }
        while let Some(value) = self.pop_front() {
            drop(value)
        }
        debug_assert_eq!(self.len(), 0);
    }
    /// Enumerate the indices and values of the elements in the back of the vector.
    ///
    /// The primary advantage over regular enumeration is that it
    /// gives proper negative indices since the elements are in the back.
    #[inline]
    pub fn enumerate_back(&self) -> SignedEnumerate<slice::Iter<T>> {
        SignedEnumerate::new(self.start_index, self.back().iter())
    }
    /// Enumerate the indices and values of the elements in the front of the vector.
    ///
    /// The only possible advantage over regular enumeration is that it
    /// gives positive `isize` indices for consistency with enumeration over the back.
    #[inline]
    pub fn enumerate_front(&self) -> SignedEnumerate<slice::Iter<T>> {
        SignedEnumerate::new(0, self.front().iter())
    }
    /// Enumerate the indices and values of each element in the front and back.
    ///
    /// The primary advantage over regular enumeration is that
    /// it gives proper negative indices for elements that are in the back.
    #[inline]
    pub fn enumerate(&self) -> SignedEnumerate<slice::Iter<T>> {
        SignedEnumerate::new(self.start(), self[..].iter())
    }
    /// Mutably enumerate the indices and values of each element in the front and back.
    ///
    /// The primary advantage over regular enumeration is that
    /// it gives proper negative indices for elements that are in the back.
    #[inline]
    pub fn enumerate_mut(&mut self) -> SignedEnumerate<slice::IterMut<T>> {
        SignedEnumerate::new(self.start(), self[..].iter_mut())
    }
    pub fn truncate_back(&mut self, len: usize) {
        // Not as optimized as `truncate_front` because I'm lazy ;)
        while self.len_back() > len {
            drop(self.pop_back().unwrap())
        }
    }
    pub fn truncate_front(&mut self, len: usize) {
        unsafe {
            // drop any extra elements
            while self.len_front() > len {
                // decrement len before the drop_in_place(), so a panic on Drop
                // doesn't re-drop the just-failed value.
                self.end_index -= 1;
                ptr::drop_in_place(self.middle_ptr().offset(self.end_index));
            }
        }
    }
    pub fn retain<F: FnMut(isize, &mut T) -> bool>(&mut self, mut pred: F) {
        self.retain_back(|index, element| pred(index, element));
        self.retain_front(|index, element| pred(index, element));
    }
    pub fn retain_back<F: FnMut(isize, &mut T) -> bool>(&mut self, mut pred: F) {
        self.drain_filter_back(|index, element| !pred(index, element));
    }
    pub fn retain_front<F: FnMut(isize, &mut T) -> bool>(&mut self, mut pred: F) {
        self.drain_filter_front(|index, element| !pred(index, element));
    }
    pub fn drain_filter_back<F: FnMut(isize, &mut T) -> bool>(
        &mut self,
        pred: F,
    ) -> DrainFilterBack<T, F> {
        let old_len = self.len_back();
        self.back_mut().reverse();
        // Guard against us getting leaked (leak amplification)
        self.start_index = 0;
        DrainFilterBack {
            old_len,
            index: 0,
            del: 0,
            vec: self,
            pred,
        }
    }
    pub fn drain_filter_front<F: FnMut(isize, &mut T) -> bool>(
        &mut self,
        pred: F,
    ) -> DrainFilterFront<T, F> {
        let old_len = self.end_index as usize;
        // Guard against us getting leaked (leak amplification)
        self.end_index = 0;
        DrainFilterFront {
            old_len,
            index: 0,
            del: 0,
            vec: self,
            pred,
        }
    }
}
impl<T: Clone> Clone for TwoSidedVec<T> {
    fn clone(&self) -> Self {
        let mut result = TwoSidedVec::with_capacity(self.len_back(), self.len_front());
        result.default_extend_back(self.back().iter().rev().cloned());
        result.default_extend_front(self.front().iter().cloned());
        result
    }
}
impl<T> Default for TwoSidedVec<T> {
    #[inline]
    fn default() -> Self {
        TwoSidedVec::new()
    }
}
impl<T: Debug> Debug for TwoSidedVec<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TwoSidedVec")
            .field("back", &self.back())
            .field("front", &self.front())
            .finish()
    }
}

pub trait TwoSidedIndex<T>: Sized + Debug {
    type Output: ?Sized;
    /// Use this as an index against the specified vec
    ///
    /// ## Safety
    /// Undefined behavior if the index is out of bounds
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output;
    /// Use this as an index against the specified vec
    ///
    /// ## Safety
    /// Undefined behavior if the index is out of bounds
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output;
    fn check(&self, target: &TwoSidedVec<T>) -> bool;
    #[inline]
    fn get(self, target: &TwoSidedVec<T>) -> Option<&Self::Output> {
        if self.check(target) {
            Some(unsafe { self.get_unchecked(target) })
        } else {
            None
        }
    }
    #[inline]
    fn get_mut(self, target: &mut TwoSidedVec<T>) -> Option<&mut Self::Output> {
        if self.check(target) {
            Some(unsafe { self.get_unchecked_mut(target) })
        } else {
            None
        }
    }
    #[inline]
    fn index(self, target: &TwoSidedVec<T>) -> &Self::Output {
        if self.check(target) {
            unsafe { self.get_unchecked(target) }
        } else {
            self.invalid()
        }
    }
    #[inline]
    fn index_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        if self.check(target) {
            unsafe { self.get_unchecked_mut(target) }
        } else {
            self.invalid()
        }
    }
    #[cold]
    #[inline(never)]
    fn invalid(self) -> ! {
        panic!("Invalid index: {:?}", self)
    }
}
impl<T, I: TwoSidedIndex<T>> Index<I> for TwoSidedVec<T> {
    type Output = I::Output;
    #[inline]
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}
impl<T, I: TwoSidedIndex<T>> IndexMut<I> for TwoSidedVec<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}
impl<T> TwoSidedIndex<T> for isize {
    type Output = T;

    #[inline]
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output {
        debug_assert!(self.check(target));
        &*target.middle_ptr().offset(self)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        &mut *target.middle_ptr().offset(self)
    }

    #[inline]
    fn check(&self, target: &TwoSidedVec<T>) -> bool {
        *self >= target.start_index && *self < target.end_index
    }
}
impl<T> TwoSidedIndex<T> for Range<isize> {
    type Output = [T];

    #[inline]
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output {
        slice::from_raw_parts(
            target.middle_ptr().offset(self.start),
            (self.end - self.start) as usize,
        )
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        slice::from_raw_parts_mut(
            target.middle_ptr().offset(self.start),
            (self.end - self.start) as usize,
        )
    }

    #[inline]
    fn check(&self, target: &TwoSidedVec<T>) -> bool {
        self.start >= target.start_index && self.start <= self.end && self.end <= target.end_index
    }
}

impl<T> TwoSidedIndex<T> for RangeFull {
    type Output = [T];

    #[inline]
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output {
        slice::from_raw_parts(target.middle_ptr().offset(target.start_index), target.len())
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        slice::from_raw_parts_mut(target.middle_ptr().offset(target.start_index), target.len())
    }

    #[inline]
    fn check(&self, _target: &TwoSidedVec<T>) -> bool {
        true
    }
}
impl<T> TwoSidedIndex<T> for RangeFrom<isize> {
    type Output = [T];

    #[inline]
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output {
        slice::from_raw_parts(
            target.middle_ptr().offset(self.start),
            (target.end_index - self.start) as usize,
        )
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        slice::from_raw_parts_mut(
            target.middle_ptr().offset(self.start),
            (target.end_index - self.start) as usize,
        )
    }

    #[inline]
    fn check(&self, target: &TwoSidedVec<T>) -> bool {
        self.start >= target.start_index && self.start <= target.end_index
    }
}
impl<T> TwoSidedIndex<T> for RangeTo<isize> {
    type Output = [T];

    #[inline]
    unsafe fn get_unchecked(self, target: &TwoSidedVec<T>) -> &Self::Output {
        slice::from_raw_parts(
            target.middle_ptr().offset(target.start_index),
            (self.end - target.start_index) as usize,
        )
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, target: &mut TwoSidedVec<T>) -> &mut Self::Output {
        slice::from_raw_parts_mut(
            target.middle_ptr().offset(target.start_index),
            (self.end - target.start_index) as usize,
        )
    }

    #[inline]
    fn check(&self, target: &TwoSidedVec<T>) -> bool {
        self.end >= target.start_index && self.end <= target.end_index
    }
}

pub struct SignedEnumerate<I> {
    index: isize,
    handle: I,
}
impl<I: Iterator> SignedEnumerate<I> {
    #[inline]
    pub fn new(start: isize, handle: I) -> Self {
        debug_assert!(
            (handle.size_hint().1.unwrap_or(0) as isize)
                .checked_add(start)
                .is_some(),
            "Overflow!"
        );
        SignedEnumerate {
            index: start,
            handle,
        }
    }
}
impl<T, I: Iterator<Item = T>> Iterator for SignedEnumerate<I> {
    type Item = (isize, T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.handle.next() {
            let index = self.index;
            self.index += 1;
            Some((index, value))
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.handle.size_hint()
    }
}
impl<I> iter::DoubleEndedIterator for SignedEnumerate<I>
where
    I: iter::DoubleEndedIterator + iter::ExactSizeIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.handle.next_back().map(|value| {
            let len = self.handle.len();
            // I'm going to pretend this is the caller's responsibility
            debug_assert!(len <= isize::max_value() as usize);
            (self.index + (len as isize), value)
        })
    }
}
impl<I: iter::FusedIterator> iter::FusedIterator for SignedEnumerate<I> {}
impl<I: iter::ExactSizeIterator> iter::ExactSizeIterator for SignedEnumerate<I> {}

impl<T> From<Vec<T>> for TwoSidedVec<T> {
    #[inline]
    fn from(mut original: Vec<T>) -> Self {
        let ptr = original.as_mut_ptr();
        let capacity = original.capacity();
        let len = original.len();
        TwoSidedVec {
            memory: unsafe {
                RawTwoSidedVec::from_raw_parts(
                    ptr,
                    Capacity {
                        back: 0,
                        front: capacity,
                    },
                )
            },
            end_index: len as isize,
            start_index: 0,
        }
    }
}
impl<T: PartialEq<U>, U> PartialEq<TwoSidedVec<U>> for TwoSidedVec<T> {
    fn eq(&self, other: &TwoSidedVec<U>) -> bool {
        if self.start() == other.start() && self.end() == other.end() {
            for (first, second) in self.back().iter().zip(other.back()) {
                if first != second {
                    return false;
                }
            }
            for (first, second) in self.front().iter().zip(other.front()) {
                if first != second {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}
impl<T: Eq> Eq for TwoSidedVec<T> {}
impl<T: Hash> Hash for TwoSidedVec<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        /*
         * NOTE: We also need to take their start into account,
         * since otherwise [; 1, 2, 3] and [1 ; 2, 3] would hash the same.
         */
        state.write_isize(self.start());
        T::hash_slice(self.back(), state);
        T::hash_slice(self.front(), state);
    }
}

pub struct DrainFilterBack<'a, T: 'a, F: FnMut(isize, &mut T) -> bool> {
    vec: &'a mut TwoSidedVec<T>,
    index: usize,
    del: usize,
    old_len: usize,
    pred: F,
}
impl<'a, T: 'a, F: FnMut(isize, &mut T) -> bool> Iterator for DrainFilterBack<'a, T, F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        // Because we reversed the memory this is almost the exact same as `DrainFilterFront`
        unsafe {
            while self.index != self.old_len {
                let i = self.index;
                self.index += 1;
                let v = slice::from_raw_parts_mut(
                    self.vec.middle_ptr().sub(self.old_len),
                    self.old_len,
                );
                let actual_index = -((i + 1) as isize);
                if (self.pred)(actual_index, &mut v[i]) {
                    self.del += 1;
                    return Some(ptr::read(&v[i]));
                } else if self.del > 0 {
                    let del = self.del;
                    let src: *const T = &v[i];
                    let dst: *mut T = &mut v[i - del];
                    // This is safe because self.vec has length 0
                    // thus its elements will not have Drop::drop
                    // called on them in the event of a panic.
                    ptr::copy_nonoverlapping(src, dst, 1);
                }
            }
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.old_len - self.index))
    }
}

impl<'a, T, F: FnMut(isize, &mut T) -> bool> Drop for DrainFilterBack<'a, T, F> {
    fn drop(&mut self) {
        for _ in self.by_ref() {}
        let target_len = self.old_len - self.del;
        unsafe {
            ptr::copy_nonoverlapping(
                self.vec.middle_ptr().sub(self.old_len),
                self.vec.middle_ptr().sub(target_len),
                target_len,
            );
        }
        debug_assert!(target_len <= isize::max_value() as usize);
        self.vec.start_index = -(target_len as isize);
        // Reverse the order so we're back to where we started
        self.vec.back_mut().reverse();
    }
}

pub struct DrainFilterFront<'a, T: 'a, F: FnMut(isize, &mut T) -> bool> {
    vec: &'a mut TwoSidedVec<T>,
    index: usize,
    del: usize,
    old_len: usize,
    pred: F,
}
impl<'a, T: 'a, F: FnMut(isize, &mut T) -> bool> Iterator for DrainFilterFront<'a, T, F> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        unsafe {
            while self.index != self.old_len {
                let i = self.index;
                self.index += 1;
                let v = slice::from_raw_parts_mut(self.vec.middle_ptr(), self.old_len);
                if (self.pred)(i as isize, &mut v[i]) {
                    self.del += 1;
                    return Some(ptr::read(&v[i]));
                } else if self.del > 0 {
                    let del = self.del;
                    let src: *const T = &v[i];
                    let dst: *mut T = &mut v[i - del];
                    // This is safe because self.vec has length 0
                    // thus its elements will not have Drop::drop
                    // called on them in the event of a panic.
                    ptr::copy_nonoverlapping(src, dst, 1);
                }
            }
            None
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.old_len - self.index))
    }
}
impl<'a, T, F: FnMut(isize, &mut T) -> bool> Drop for DrainFilterFront<'a, T, F> {
    fn drop(&mut self) {
        for _ in self.by_ref() {}
        let target_len = (self.old_len - self.del) as isize;
        assert!(target_len >= 0);
        self.vec.end_index = target_len;
    }
}
