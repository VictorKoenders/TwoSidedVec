use std::{mem, iter, slice, vec};
use super::TwoSidedVec;


/// The two-sided counterpart to `Extend`,
/// which supports extending both the front and back of the
pub trait TwoSidedExtend<T> {
    fn extend_back<I: IntoIterator<Item=T>>(&mut self, iter: I);
    fn extend_front<I: IntoIterator<Item=T>>(&mut self, iter: I);
}

impl<T> TwoSidedExtend<T> for TwoSidedVec<T> {
    #[inline]
    fn extend_back<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        SpecExtend::<T, I::IntoIter>::extend_back(self, iter.into_iter())
    }
    #[inline]
    fn extend_front<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        SpecExtend::<T, I::IntoIter>::extend_front(self, iter.into_iter())
    }
}

impl<'a, T: Copy + 'a> TwoSidedExtend<&'a T> for TwoSidedVec<T> {
    #[inline]
    fn extend_back<I: IntoIterator<Item=&'a T>>(&mut self, iter: I) {
        SpecExtend::<&'a T, I::IntoIter>::extend_back(self, iter.into_iter())
    }
    #[inline]
    fn extend_front<I: IntoIterator<Item=&'a T>>(&mut self, iter: I) {
        SpecExtend::<&'a T, I::IntoIter>::extend_front(self, iter.into_iter())
    }
}



/// The internal trait used to specialize `TwoSidedExtend` into more efficient implementations.
trait SpecExtend<T, I> {
    fn extend_back(&mut self, iter: I);
    fn extend_front(&mut self, iter: I);
}

impl<T, I: Iterator<Item=T>> SpecExtend<T, I> for TwoSidedVec<T> {
    #[inline]
    default fn extend_back(&mut self, iter: I) {
        self.default_extend_back(iter)
    }
    #[inline]
    default fn extend_front(&mut self, iter: I) {
        self.default_extend_front(iter)
    }
}
/// Specialize for iterators whose length we trust.
impl<T, I: iter::TrustedLen<Item=T>> SpecExtend<T, I> for TwoSidedVec<T> {
    default fn extend_back(&mut self, iter: I) {
        let (low, high) = iter.size_hint();
        if let Some(additional) = high {
            debug_assert_eq!(additional, low);
            self.reserve_back(additional);
            debug_assert!(additional <= isize::max_value() as usize);
            let mut ptr = self.start_ptr();
            for value in iter {
                unsafe {
                    ptr = ptr.sub(1);
                    ptr.write(value);
                    /*
                     * Remember we need to be panic safe,
                     * so we must set this at each step.
                     * Overflow is impossible,
                     * since we can never allocate more than `isize::max_value` bytes.
                     * It appears this actually won't be optimized away by LLVM
                     * as described in rust-lang/rust#32155
                     * @bluss found a got workaround but it's kind of overkill for us
                     * rust-lang/rust#36355
                     */
                    self.start_index -= 1;
                }
            }
        } else {
            self.default_extend_back(iter);
        }
    }

    default fn extend_front(&mut self, iter: I) {
        let (low, high) = iter.size_hint();
        if let Some(additional) = high {
            debug_assert_eq!(additional, low);
            self.reserve_front(additional);
            debug_assert!(additional <= isize::max_value() as usize);
            let mut ptr = self.end_ptr();
            for value in iter {
                unsafe {
                    ptr.write(value);
                    ptr = ptr.add(1);
                }
                /*
                 * Remember we need to be panic safe,
                 * so we must set this at each step.
                 * See above for more info on optimization and overflow.
                 */
                self.end_index += 1;
            }
        } else {
            self.default_extend_front(iter);
        }
    }
}

impl<T> SpecExtend<T, vec::IntoIter<T>> for TwoSidedVec<T> {
    #[inline]
    fn extend_back(&mut self, iter: vec::IntoIter<T>) {
        // This is specialized to reuse the allocation, so it should be free
        let elements = iter.collect::<Vec<T>>();
        unsafe { self.raw_extend_back(elements.as_ptr(), elements.len()); }
        mem::forget(elements);
    }

    #[inline]
    fn extend_front(&mut self, iter: vec::IntoIter<T>) {
        // This is specialized to reuse the allocation, so it should be free
        let elements = iter.collect::<Vec<T>>();
        unsafe { self.raw_extend_front(elements.as_ptr(), elements.len()); }
        mem::forget(elements);
    }
}
impl<'a, T: Copy + 'a, I: Iterator<Item=&'a T>> SpecExtend<&'a T, I> for TwoSidedVec<T> {
    #[inline]
    default fn extend_back(&mut self, iter: I) {
        TwoSidedExtend::extend_back(self, iter.cloned())
    }

    #[inline]
    default fn extend_front(&mut self, iter: I) {
        TwoSidedExtend::extend_front(self, iter.cloned());
    }
}
/// Specialize `Copy`able slices to directly `memcpy` their bytes.
impl<'a, T: Copy + 'a> SpecExtend<&'a T, slice::Iter<'a, T>> for TwoSidedVec<T> {
    #[inline]
    fn extend_back(&mut self, iter: slice::Iter<'a, T>) {
        let target = iter.as_slice();
        unsafe {
            self.raw_extend_back(target.as_ptr(), target.len());
        }
    }

    #[inline]
    fn extend_front(&mut self, iter: slice::Iter<'a, T>) {
        let target = iter.as_slice();
        unsafe {
            self.raw_extend_front(target.as_ptr(), target.len());
        }
    }
}

