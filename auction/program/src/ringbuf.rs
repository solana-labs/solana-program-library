//! Taken from ringbuf, the code is short Apache/MIT licensed. 
//! Changes:
//! - Adds Borsh serialization on top.
//! - Program is single-threaded so atomic operations removed.
//! - Ditch MaybeUninit and allocate with Option

use alloc::{sync::Arc, vec::Vec};
use borsh::{BorshDeserialize, BorshSerialize};
use core::{
    cmp::min,
    mem::{self},
    ptr::{self, copy},
};

use crate::{consumer::Consumer, producer::Producer};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct RingBuffer<T: Sized> {
    pub data: Vec<T>,
    pub head: usize,
    pub tail: usize,
}

impl<T: Sized> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity + 1);
        Self {
            data: data,
            head: 0,
            tail: 0,
        }
    }

    pub fn split(&mut self) -> (Producer<T>, Consumer<T>) {
        (
            Producer { rb: &mut self },
            Consumer { rb: &mut self }
        )
    }

    pub fn capacity(&self) -> usize {
        self.data.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        (self.tail + 1) % (self.capacity() + 1) == self.head
    }

    pub fn len(&self) -> usize {
        (self.tail + self.capacity() + 1 - self.head) % (self.capacity() + 1)
    }

    pub fn remaining(&self) -> usize {
        self.capacity() - self.len()
    }
}

struct SlicePtr<T: Sized> {
    pub ptr: *mut T,
    pub len: usize,
}

impl<T> SlicePtr<T> {
    fn null() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }
    fn new(slice: &mut [T]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
        }
    }
    unsafe fn shift(&mut self, count: usize) {
        self.ptr = self.ptr.add(count);
        self.len -= count;
    }
}

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn move_items<T>(src: &mut Consumer<T>, dst: &mut Producer<T>, count: Option<usize>) -> usize {
    unsafe {
        src.pop_access(|src_left, src_right| -> usize {
            dst.push_access(|dst_left, dst_right| -> usize {
                let n = count.unwrap_or_else(|| {
                    min(
                        src_left.len() + src_right.len(),
                        dst_left.len() + dst_right.len(),
                    )
                });
                let mut m = 0;
                let mut src = (SlicePtr::new(src_left), SlicePtr::new(src_right));
                let mut dst = (SlicePtr::new(dst_left), SlicePtr::new(dst_right));

                loop {
                    let k = min(n - m, min(src.0.len, dst.0.len));
                    if k == 0 {
                        break;
                    }
                    copy(src.0.ptr, dst.0.ptr, k);
                    if src.0.len == k {
                        src.0 = src.1;
                        src.1 = SlicePtr::null();
                    } else {
                        src.0.shift(k);
                    }
                    if dst.0.len == k {
                        dst.0 = dst.1;
                        dst.1 = SlicePtr::null();
                    } else {
                        dst.0.shift(k);
                    }
                    m += k
                }

                m
            })
        })
    }
}
