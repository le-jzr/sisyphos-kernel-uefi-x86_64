use spin;

use alloc::allocator::{Alloc, AllocErr, Layout};
use efi_app;

use memory::list_alloc_simple;

pub struct HeapAllocator {
    inner: spin::Mutex<efi_app::Allocator<list_alloc_simple::ListAlloc>>,
}

impl HeapAllocator {
    pub const fn new() -> Self {
        HeapAllocator { inner: spin::Mutex::new(efi_app::Allocator::new(list_alloc_simple::ListAlloc::new())) }
    }
}

unsafe impl<'a> Alloc for &'a HeapAllocator {
    #[inline]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        self.inner.lock().alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.inner.lock().dealloc(ptr, layout)
    }
}


