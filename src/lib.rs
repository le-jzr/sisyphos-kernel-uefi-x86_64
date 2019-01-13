#![feature(lang_items)]

#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(repr_align)]
#![feature(attr_literals)]
#![feature(const_fn)]
#![feature(dynamic_sys)]

// FIXME: remove
#![allow(unreachable_code)]
#![allow(dead_code)]
#![allow(unused_variables)]

#[allow(unused_extern_crates)]
extern crate rlibc;

extern crate alloc;

extern crate efi_app;
extern crate x86_64;
extern crate spin;

#[macro_use]
extern crate bitflags;

mod relocate;
mod memory;
//pub mod panic;
//pub mod rt_stubs;
mod sys;

pub mod cdefs;

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

    sys::init();

    // TODO: This change to mapping needs to be reverted if we return without calling ExitBootServices().
    // The remapping we do is currently infringing on UEFI's turf (we don't own the memory map yet,
    // so we shouldn't touch it). It should be fine if we end up calling `ExitBootServices()`, since UEFI itself
    // should only have identity mapping, so the top half should be empty. If we don't ExitBootServices(), though,
    // we have to return things into original state.

    // TODO: Relocate everything into high memory, including current RIP and RSP.

    ctx.console_out().clear_screen().ok();

    //print!("{:?}", unsafe { memory::paging::L4Table::current() });

    println!("Hello, EFI world!");


    panic!("Cannot return from main until we fix mapping.");
    //return efi_app::Status::success();
}


