use std::fmt;
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;

use x86_64::PhysicalAddress;
use x86_64::registers::control_regs;

// Start of the upper virtual memory half on current processors with 4-level page tables and 48-bit virtual addresses.
pub const FLAT_MEMORY_START: usize = 0xffff800000000000;

bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

#[derive(Copy, Clone)]
pub struct Entry(u64);

impl Entry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn frame_address(&self) -> Option<PhysicalAddress> {
        if self.flags().contains(PRESENT) {
            Some(PhysicalAddress(self.0 & 0x000f_ffff_ffff_f000))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: PhysicalAddress, flags: EntryFlags) {
        assert!((frame.0 & !0x000f_ffff_ffff_f000) == 0);
        self.0 = frame.0 | flags.bits();
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.flags().contains(PRESENT) {
            f.write_fmt(format_args!("({:x},{:x},{:?})", self.0, self.frame_address().unwrap().0, self.flags()))
        } else {
            f.write_str("0")
        }
    }
}

pub const ENTRY_COUNT: usize = 512;


pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}

#[repr(C, align(4096))]
pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L: TableLevel> fmt::Debug for Table<L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(f.write_fmt(format_args!("Table ")));
        <[Entry] as fmt::Debug>::fmt(&self.entries, f)
    }
}

impl<L> Table<L> where L: TableLevel {
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

#[inline]
unsafe fn flat_mapped_ref<'a, T>(addr: PhysicalAddress) -> &'a T {
    &*((addr.0 as usize + FLAT_MEMORY_START) as *const T)
}

#[inline]
unsafe fn flat_mapped_mut<'a, T>(addr: PhysicalAddress) -> &'a mut T {
    &mut *((addr.0 as usize + FLAT_MEMORY_START) as *mut T)
}

impl<L> Table<L> where L: HierarchicalLevel
{
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index).map(|address| unsafe { flat_mapped_ref(address) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index).map(|address| unsafe { flat_mapped_mut(address) })
    }

    fn next_table_address(&self, index: usize) -> Option<PhysicalAddress> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(PRESENT) && !entry_flags.contains(HUGE_PAGE) {
            self[index].frame_address()
        } else {
            None
        }
    }
}

impl Table<Level4> {
    /// Assumes that physical memory is currently identity-mapped.
    pub unsafe fn initialize_high_mapping() {
        let l4 = &mut *(control_regs::cr3().0 as usize as *mut Table<Level4>);
        for i in 0..256 {
            assert!(l4[i+256].is_unused());
            l4[i+256] = l4[i];
        }
    }

    pub unsafe fn current<'a>() -> &'a Self {
        flat_mapped_ref(control_regs::cr3())
    }

    pub unsafe fn current_mut<'a>() -> &'a mut Self {
        flat_mapped_mut(control_regs::cr3())
    }
}

pub type L4Table = Table<Level4>;

impl<L> Index<usize> for Table<L> where L: TableLevel {
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}


