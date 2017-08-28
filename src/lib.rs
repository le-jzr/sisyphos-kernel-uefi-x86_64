#![feature(lang_items)]
#![no_std]
#![feature(compiler_builtins_lib)]

#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(raw)]
#![feature(shared)]
#![feature(repr_align)]
#![feature(attr_literals)]
#![feature(const_fn)]

// FIXME: remove
#![allow(unreachable_code)]


extern crate rlibc;
extern crate compiler_builtins;
extern crate alloc;

extern crate efi_app;
extern crate x86_64;
extern crate spin;

#[macro_use]
extern crate bitflags;

mod relocate;
mod memory;
pub mod panic;
pub mod rt_stubs;

use alloc::boxed::Box;


#[global_allocator]
static ALLOCATOR: memory::heap::HeapAllocator = memory::heap::HeapAllocator::new();

#[no_mangle]
pub extern "C" fn efi_main(ldbase: u64, dyn: *const u8, arg1: efi_app::Arg1, arg2: efi_app::Arg2) -> efi_app::Status
{
    // First, relocate to identity-mapped region, so asserts etc work.
    unsafe { relocate::relocate(ldbase, dyn); }

    // FIXME: this implicitly initializes globals in efi_app, which is weird.
    let mut ctx = unsafe { efi_app::BootContext::new(arg1, arg2) };

    // Copy the flat memory mapping into the upper half.
    // This is done simply by taking the top-level page table from UEFI
    // and copying existing pointers from bottom half to top half.
    //
    // The upper half is verified to be empty, in order to avoid unknowingly
    // clobbering something important. If the upper half is mapped by UEFI,
    // we just print an error message and make it halt and catch fire. Such a
    // thing happening should be highly unlikely.
    unsafe { memory::paging::L4Table::initialize_high_mapping(); }

    let b = Box::new(5);
    // TODO: Relocate everything into high memory, including current RIP and RSP.

    let o = ctx.console_out();
    o.clear_screen();

    //o.write_fmt(format_args!("{:?}", unsafe { memory::paging::L4Table::current() }));

    o.output_string("Hello, EFI world!\n");

    loop{}
    //return efi_app::Status::success();
}


