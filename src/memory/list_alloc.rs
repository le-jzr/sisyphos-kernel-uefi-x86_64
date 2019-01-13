use core::mem;
use core::ptr::Unique;
use core::cmp;
use core::marker::PhantomData;

use alloc::allocator::{Alloc, Layout, AllocErr};
use memory::paging::FLAT_MEMORY_START;

struct Span(Unique<SpanHeader>);
struct SpanList(Option<Span>);
struct SpanListLen {
    list: SpanList,
    length: usize,
}

#[repr(C)]
struct SpanHeader {
    size: usize,
    tail: SpanList,
}

const MARK: usize = 0x8000_0000_0000_0000;

impl Span {
    #[inline]
    unsafe fn new(address: usize, size: usize) -> Span {
        debug_assert!(size & MARK == 0);

        let new_span = Span(Unique::new(address as *mut SpanHeader));
        *(new_span.header_mut()) = SpanHeader { size: size, tail: SpanList(None) };
        new_span
    }

    #[inline]
    fn header(&self) -> &SpanHeader {
        unsafe{self.0.as_ref()}
    }

    #[inline]
    unsafe fn header_mut(&mut self) -> &mut SpanHeader {
        self.0.as_mut()
    }

    #[inline]
    fn address(&self) -> usize {
        self.0.as_ptr() as usize
    }

    #[inline]
    fn size(&self) -> usize {
        self.header().size & !MARK
    }

    #[inline]
    unsafe fn set_size(&mut self, size: usize) {
        debug_assert!(size & MARK == 0);
        self.header_mut().size = size;
    }

    #[inline]
    fn mark(&mut self) {
        unsafe {
            self.header_mut().size |= MARK;
        }
    }

    #[inline]
    fn unmark(&mut self) {
        unsafe {
            self.header_mut().size &= !MARK;
        }
    }

    #[inline]
    fn is_marked(&self) -> bool {
        (self.header().size & MARK) != 0
    }

    #[inline]
    fn limit(&self) -> usize {
        self.address() + self.size()
    }

    #[inline]
    fn overlaps(&self, other: &Span) -> bool {
        !(self.limit() <= other.address() || other.limit() <= self.address())
    }

    #[inline]
    fn is_adjacent(&self, next: &Span) -> bool {
        self.limit() == next.address()
    }

    #[inline]
    fn skip(&self, n: usize) -> Option<&Span> {
        let mut s = self;
        for i in 0..n {
            match s.next() {
                None => return None,
                Some(t) => s = t,
            }
        }

        Some(s)
    }

    #[inline]
    fn skip_mut(&mut self, n: usize) -> Option<&mut Span> {
        let mut s = self;
        for i in 0..n {
            match s.next_mut() {
                None => return None,
                Some(t) => s = t,
            }
        }

        Some(s)
    }

    #[inline]
    fn next(&self) -> Option<&Span> {
        self.header().tail.first()
    }

    #[inline]
    fn next_mut(&mut self) -> Option<&mut Span> {
        unsafe { self.header_mut().tail.first_mut() }
    }

    #[inline]
    fn last_mut(&mut self) -> &mut Span {
        let mut s = self;
        while let Some(n) = s.next_mut() {
            s = n;
        }
        s
    }

    #[inline]
    fn split_at(&mut self, address: usize) -> Span {
        assert!(address >= self.address() + mem::size_of::<SpanHeader>());
        assert!(address <= self.limit() - mem::size_of::<SpanHeader>());

        let front_size = address - self.address();
        let back_size = self.size() - front_size;

        self.set_size(front_size);
        Span::new(address, back_size)
    }

    #[inline]
    fn unwrap(self) -> *mut u8 {
        debug_assert!(self.next().is_none());

        let ptr = self.address() as *mut u8;
        mem::forget(self);
        ptr
    }
}

impl SpanList {
    #[inline]
    const fn new() -> SpanList {
        SpanList(None)
    }

    #[inline]
    fn empty(&self) -> bool {
        self.0.is_none()
    }

    #[inline]
    fn first(&self) -> Option<&Span> {
        self.0.as_ref()
    }

    #[inline]
    fn first_mut(&mut self) -> Option<&mut Span> {
        self.0.as_mut()
    }

    fn count(&self) -> usize {
        let mut iter: Option<&mut Span> = self.0.as_mut();
        let mut cnt = 0;

        while let Some(span) = iter {
            cnt += 1;
            iter = span.next_mut();
        }

        cnt
    }

    fn check_links(&mut self) -> bool {
        {
            let mut iter: Option<&mut Span> = self.0.as_mut();

            while let Some(span) = iter {
                assert!(!span.is_marked(), "Cycle in header list.");
                span.mark();
                iter = span.next_mut();
            }
        }
        {
            let mut iter: Option<&mut Span> = self.0.as_mut();

            while let Some(span) = iter {
                assert!(span.is_marked(), "Should never happen.");
                span.unmark();
                iter = span.next_mut();
            }
        }

        true
    }

    #[inline]
    fn take(&mut self) -> SpanList {
        let nlist = SpanList(self.0);
        self.0 = None;
        nlist
    }
}

impl SpanListLen {

    #[inline]
    fn take(&mut self) -> SpanListLen {
        let nlist = SpanListLen { list: self.list.take(), size: self.size };
        self.size = 0;

        nlist
    }

    fn check_length(&self) -> bool {
        assert!(self.list.count() == self.length);
        true
    }

    fn split_at(self, at: usize) -> (SpanListLen, SpanListLen) {
        assert!(at <= self.len());

        if at == 0 {
            return (SpanListLen::new(), self);
        }

        let second: SpanListLen;
        {
            let r = self.list.first_mut().unwrap().skip_mut(at-1).unwrap();
            second = SpanListLen{ list: r.header_mut().tail.take(), length: self.length - at };
        }

        self.length = at;

        debug_assert!(self.check_length());
        debug_assert!(second.check_length());
        (self, second)
    }

    fn merge(a: SpanListLen, b: SpanListLen) -> SpanListLen {
        unimplemented!()

        // TODO
        /*
        let mut dummy = SegmentHeader{size:0, next:None};
        let mut out_list = SpanListLen{
            list: SpanList(Some(Span::new(&mut dummy as usize, mem::size_of_val(&dummy))),
            length: 1,
        };

        let mut length = 0;

        {
            let mut tail = out_list.first_mut().unwrap();

            let mut a = a;
            let mut b = b;

            loop {
                if a.empty() {
                    mem::swap(&mut a, &mut b);
                }

                if a.empty() {
                    break;
                }

                if b.empty() {
                    if let SpanList(Some(span)) = a {
                        if tail.is_adjacent(&span) {
                        } else {

                        }
                    } else {
                        unreachable!()
                    }


                    let s = a.pop().unwrap();
                    if tail.is_adjacent(&s) {
                        tail.extend_with(s);
                    } else {
                        tail.header_mut().tail = SpanList(Some(s));
                        length += 1;
                    }

                    let SomeList(
                }

                match (a.first(), b.first()) {
                    (None, None) => {
                        break;
                    },
                    (Some(s1), None) => {
                        if tail.is_adjacent(s1) {
                            tail.extend(s1);
                        } else {
                            tail.as_mut().next = Some(s1);
                        }

                        return head.next;
                    },
                    (Some(s1), Some(s2)) => {
                        debug_assert!(!s1.as_ref().overlaps(s2.as_ref()));

                        let s = if s1.as_ref().address() < s2.as_ref().address() { l1 = s1.as_ref().next; s1 } else { l2 = s2.as_ref().next; s2 };

                        if tail.as_ref().is_adjacent(s.as_ref()) {
                            tail.as_mut().extend(s.as_ref());
                        } else {
                            tail.as_mut().next = Some(s);
                            tail = s;
                        }
                    },
                    _ => unreachable!()
                }
            }
            */
    }

    fn len(&self) -> usize {
        self.length
    }

    fn sort(&mut self) {
        unimplemented!()
    }
/*
        unsafe fn sort_list(l: Option<Unique<SegmentHeader>>, len: usize) -> Option<Unique<SegmentHeader>> {
        if len <= 1 {
            return l;
        }

        let len1 = len / 2;
        let len2 = len - len1;

        let (l1, l2) = Self::split_list(l, len1);
        let l1 = Self::sort_list(l1, len1);
        let l2 = Self::sort_list(l2, len2);
        Self::merge_lists(l1, l2)
    }
*/
}

#[test]
fn test_segment_header_size() {
    assert_eq!(mem::size_of::<SegmentHeader>(), 2*mem::size_of::<usize>());
}

pub struct ListAlloc {
    free_list: SpanListLen,
    garbage_list: SpanListLen,
    garbage_limit: usize,

    total_bytes: usize,
    current_allocated_bytes: usize,
    current_free_bytes: usize,
    current_garbage_bytes: usize,
}

impl ListAlloc {
    #[inline]
    pub const fn new() -> Self {
        Self {
            free_list: SpanListLen::new(), garbage_list: SpanListLen::new(),
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
        loop {
            if let Some(span) = self.free_list.first() {
                let required_top = Self::align_up(span.address(), align) + size;
                if required_top <= span.limit() {
                    return true;
                }
            } else {
                return false;
            }

            self.garbage_list.push(self.free_list.pop().unwrap());
        }
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

        let mut span = self.free_list.pop().unwrap();

        let alloc_start = Self::align_up(span.address(), align);
        let alloc_end = alloc_start + size;

        assert!(alloc_end <= span.limit());

        if alloc_end < span.limit() {
            let nspan = span.split_at(alloc_end);
            self.free_list.push(nspan);
        }

        if alloc_start > span.address() {
            let nspan = span.split_at(alloc_start);
            self.free_list.push(span);
            span = nspan;
        }

        self.current_allocated_bytes += size;
        self.current_free_bytes -= size;

        debug_assert!(self.debug_check());

        debug_assert!(span.address() == alloc_start);
        debug_assert!(span.limit() == alloc_end);

        Ok(span.unwrap())
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = Self::align_up(layout.size(), mem::size_of::<SpanHeader>());

        self.garbage_list.push(Span::new(ptr as usize, size));

        self.current_allocated_bytes -= size;
        self.current_garbage_bytes += size;
        debug_assert!(self.debug_check());

        if self.garbage_list.len() >= self.garbage_limit {
            self.gc();
        }
    }

    unsafe fn gc(&mut self) {
        let garbage = self.garbage_list.take();
        garbage.sort();

        self.free_list = SpanListLen::merge(self.free_list, garbage);

        self.current_free_bytes += self.current_garbage_bytes;
        self.current_garbage_bytes = 0;
        debug_assert!(self.debug_check());
    }

    // Verifies that the lists are sane and that they match the expected byte counts.
    fn debug_check(&mut self) -> bool {
        // Basics.
        assert_eq!(self.total_bytes, self.current_allocated_bytes + self.current_free_bytes + self.current_garbage_bytes);

        // Verify free list.
            assert!(self.free_list.check_length());
            assert!(self.free_list.check_links());

            let mut counted_bytes: usize = 0;
            let mut previous_limit: usize = 0;
            let mut head = self.free_list.first();

            while let Some(span) = head {
                assert!(previous_limit == 0 || previous_limit < span.address());
                counted_bytes += span.size();
                previous_limit = span.limit();
                head = span.next();
            }

            assert_eq!(self.current_free_bytes, counted_bytes);


            // Verify garbage list as well as we are able.
            assert!(self.free_list.check_length());
            assert!(self.free_list.check_links());

            let mut counted_bytes: usize = 0;
            let mut head = self.garbage_list.first();

            while let Some(span) = head {
                counted_bytes += span.size();
                head = span.next();
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
