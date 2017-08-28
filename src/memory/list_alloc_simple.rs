use core::mem;
use core::ptr::null_mut;
use core::cmp;

use alloc::allocator::{Alloc, Layout, AllocErr};
use memory::paging::FLAT_MEMORY_START;

// A little wrapper for a little checking.
macro_rules! d {
    ($expression:expr) => ({
            debug_assert!($expression != null_mut());
            (&mut *$expression)
    })
}

struct SpanList {
    first: *mut SpanHeader,
    length: usize,
}

impl SpanList {
    #[inline]
    const fn new() -> SpanList {
        SpanList { first: null_mut(), length: 0 }
    }

    #[inline]
    fn empty(&self) -> bool {
        self.first == null_mut()
    }

    #[inline]
    unsafe fn push(&mut self, entry: *mut SpanHeader) {
        assert!(entry != null_mut());

        d!(entry).next = self.first;
        self.first = entry;
        self.length += 1;
    }

    #[inline]
    unsafe fn pop(&mut self) -> *mut SpanHeader {
        assert!(self.first != null_mut());

        self.length -= 1;

        let hdr = self.first;
        self.first = d!(hdr).next;
        d!(hdr).next = null_mut();
        hdr
    }

    #[inline]
    fn first_address(&self) -> usize {
        self.first as usize
    }

    #[inline]
    fn len(&self) -> usize {
        self.length
    }

    #[inline]
    fn take(&mut self) -> SpanList {
        let mut nlist = SpanList::new();
        mem::swap(self, &mut nlist);
        nlist
    }

    unsafe fn count(&self) -> usize {
        let mut hdr = self.first;
        let mut cnt = 0;

        while hdr != null_mut() {
            cnt += 1;
            hdr = d!(hdr).next;
        }

        cnt
    }

    unsafe fn check_length(&self) -> bool {
        assert!(self.count() == self.length);
        true
    }

    unsafe fn check_links(&mut self) -> bool {
        let mut hdr = self.first;

        while hdr != null_mut() {
            assert!(!d!(hdr).is_marked(), "Cycle in header list.");
            d!(hdr).mark();
            hdr = d!(hdr).next;
        }

        let mut hdr = self.first;

        while hdr != null_mut() {
            assert!(d!(hdr).is_marked(), "???");
            d!(hdr).unmark();
            hdr = d!(hdr).next;
        }

        true
    }

    unsafe fn merge(mut a: SpanList, mut b: SpanList) -> SpanList {
        if a.length == 0 && b.length == 0 {
            return a;
        }

        let mut head = SpanHeader{size: 0, next: null_mut()};
        let mut tail: *mut SpanHeader = &mut head;
        let mut length = 0;


        loop {
            if a.first_address() > b.first_address() {
                mem::swap(&mut a, &mut b);
            }

            assert!(!b.empty());

            if a.empty() {
                d!(tail).next = b.first;
                length += b.length;
                break;
            }

            let hdr = a.pop();

            assert!(d!(tail).limit() <= d!(hdr).address());

            if d!(tail).limit() == d!(hdr).address() {
                d!(tail).size += d!(hdr).size;
            } else {
                d!(tail).next = hdr;
                tail = hdr;
                length += 1;
            }
        }

        let mut list = SpanList{ first: head.next, length: length };
        debug_assert!(list.check_length());
        debug_assert!(list.check_links());
        list
    }

    unsafe fn split_off(&mut self, n: usize) -> SpanList {
        assert!(self.length > n);

        let other_len = self.length - n;
        self.length = n;

        let mut hdr = self.first;
        for i in 1..n {
            hdr = d!(hdr).next;
        }

        let list = SpanList{ first: d!(hdr).next, length: other_len };
        d!(hdr).next = null_mut();
        list
    }

    unsafe fn sort(&mut self) {
        if self.length <= 1 {
            return;
        }

        let len = self.length;
        let mut other = self.split_off(len / 2);

        self.sort();
        other.sort();

        *self = Self::merge(self.take(), other);
    }
}

#[repr(C)]
struct SpanHeader {
    size: usize,
    next: *mut SpanHeader,
}

const MARK: usize = 0x8000_0000_0000_0000;

impl SpanHeader {
    #[inline]
    fn address(&self) -> usize {
        self as *const SpanHeader as usize
    }

    #[inline]
    fn limit(&self) -> usize {
        self.address() + self.size
    }

    #[inline]
    unsafe fn split_at(&mut self, address: usize) -> *mut SpanHeader {
        assert!(address >= self.address() + mem::size_of::<SpanHeader>());
        assert!(address <= self.limit() - mem::size_of::<SpanHeader>());

        let front_size = address - self.address();
        let back_size = self.size - front_size;

        self.size = front_size;
        let span = address as *mut SpanHeader;
        *span = SpanHeader { size: back_size, next: null_mut() };
        span
    }

    #[inline]
    fn mark(&mut self) {
        self.size |= MARK;
    }

    #[inline]
    fn unmark(&mut self) {
        self.size &= !MARK;
    }

    #[inline]
    fn is_marked(&self) -> bool {
        (self.size & MARK) != 0
    }
}

#[test]
fn test_segment_header_size() {
    assert_eq!(mem::size_of::<SpanHeader>(), 2*mem::size_of::<usize>());
}

pub struct ListAlloc {
    free_list: SpanList,
    garbage_list: SpanList,
    garbage_limit: usize,

    total_bytes: usize,
    current_allocated_bytes: usize,
    current_free_bytes: usize,
    current_garbage_bytes: usize,
}

unsafe impl Send for ListAlloc {}

impl ListAlloc {
    #[inline]
    pub const fn new() -> Self {
        Self {
            free_list: SpanList::new(), garbage_list: SpanList::new(),
            garbage_limit: 1024, total_bytes: 0, current_allocated_bytes: 0,
            current_free_bytes: 0, current_garbage_bytes: 0
        }
    }

    pub unsafe fn provide(&mut self, ptr: *mut u8, size: usize) {
        assert!((ptr as usize) % mem::size_of::<SpanHeader>() == 0);
        assert!(size % mem::size_of::<SpanHeader>() == 0);

        self.total_bytes += size;
        self.current_allocated_bytes += size;
        self.dealloc(ptr, Layout::array::<u8>(size).unwrap());
    }

    #[inline]
    fn align_up(val: usize, align: usize) -> usize {
        debug_assert!(align.is_power_of_two());

        (((val-1)/align)+1)*align
    }

    unsafe fn search_free_list(&mut self, size: usize, align: usize) -> bool {
        while !self.free_list.empty() {
            let address = self.free_list.first as usize;
            let limit = address + d!(self.free_list.first).size;

            let required_top = Self::align_up(address, align) + size;
            if required_top <= limit {
                return true;
            }

            self.garbage_list.push(self.free_list.pop());
        }

        false
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        let size = Self::align_up(layout.size(), mem::size_of::<SpanHeader>());
        let align = cmp::max(layout.align(), mem::size_of::<SpanHeader>());

        if !self.search_free_list(size, align) {
            self.gc();

            if !self.search_free_list(size, align) {
                return Err(AllocErr::Exhausted{request: layout});
            }
        }

        let mut span = self.free_list.pop();

        let alloc_start = Self::align_up(span as usize, align);
        let alloc_end = alloc_start + size;

        assert!(alloc_end <= d!(span).limit());

        if alloc_end < d!(span).limit() {
            let nspan = d!(span).split_at(alloc_end);
            self.free_list.push(nspan);
        }

        if alloc_start > d!(span).address() {
            let nspan = d!(span).split_at(alloc_start);
            self.free_list.push(span);
            span = nspan;
        }

        self.current_allocated_bytes += size;
        self.current_free_bytes -= size;

        debug_assert!(self.debug_check());

        debug_assert!(d!(span).address() == alloc_start);
        debug_assert!(d!(span).limit() == alloc_end);

        Ok(span as *mut u8)
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = Self::align_up(layout.size(), mem::size_of::<SpanHeader>());

        let span = ptr as *mut SpanHeader;
        *span = SpanHeader { size: size, next: null_mut() };
        self.garbage_list.push(span);

        self.current_allocated_bytes -= size;
        self.current_garbage_bytes += size;
        debug_assert!(self.debug_check());

        if self.garbage_list.len() >= self.garbage_limit {
            self.gc();
        }
    }

    unsafe fn gc(&mut self) {
        let mut garbage = self.garbage_list.take();
        garbage.sort();

        self.free_list = SpanList::merge(self.free_list.take(), garbage);

        self.current_free_bytes += self.current_garbage_bytes;
        self.current_garbage_bytes = 0;
        debug_assert!(self.debug_check());
    }

    // Verifies that the lists are sane and that they match the expected byte counts.
    unsafe fn debug_check(&mut self) -> bool {
        // Basics.
        assert_eq!(self.total_bytes, self.current_allocated_bytes + self.current_free_bytes + self.current_garbage_bytes);

        // Verify free list.
            assert!(self.free_list.check_length());
            assert!(self.free_list.check_links());

            let mut counted_bytes: usize = 0;
            let mut previous_limit: usize = 0;
            let mut head = self.free_list.first;

            while head != null_mut() {
                assert!(previous_limit == 0 || previous_limit < d!(head).address());
                counted_bytes += d!(head).size;
                previous_limit = d!(head).limit();
                head = d!(head).next;
            }

            assert_eq!(self.current_free_bytes, counted_bytes);


            // Verify garbage list as well as we are able.
            assert!(self.free_list.check_length());
            assert!(self.free_list.check_links());

            let mut counted_bytes: usize = 0;
            let mut head = self.garbage_list.first;

            while head != null_mut() {
                counted_bytes += d!(head).size;
                head = d!(head).next;
            }

            assert_eq!(self.current_garbage_bytes, counted_bytes);
        true
    }
}

unsafe impl Alloc for ListAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        ListAlloc::alloc(self, layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        ListAlloc::dealloc(self, ptr, layout)
    }
}



// TODO: move elsewhere
use efi_app;
unsafe impl efi_app::FrontAllocator for ListAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        ListAlloc::alloc(self, layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        ListAlloc::dealloc(self, ptr, layout)
    }

    unsafe fn feed_memory(&mut self, addr: efi_app::PhysicalAddress, size: usize) {
        ListAlloc::provide(self, (FLAT_MEMORY_START + addr.0 as usize) as *mut u8, size)
    }
}
