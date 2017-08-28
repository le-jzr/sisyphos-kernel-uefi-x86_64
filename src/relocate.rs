
use efi_app;
use core::ptr;

#[repr(C)]
pub struct Elf64_Dyn {
	d_tag: u64,
	d_val: u64,
}

#[repr(C)]
pub struct Elf64_Rela {
	r_offset: u64,
	r_info: u64,
	r_addend: u64,
}

const R_X86_64_RELATIVE: u64 = 8;
const DT_NULL: u64 = 0;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_RELAENT: u64 = 9;

pub unsafe fn relocate(ldbase: u64, dyn: *const u8) -> efi_app::Status {

    let dyn = dyn as *const Elf64_Dyn;

    let mut relsz: isize = 0;
    let mut relent: isize = 0;
    let mut rel: *const u8 = ptr::null();
    let mut dyn = dyn;

    while (*dyn).d_tag != DT_NULL {
        match (*dyn).d_tag {
            DT_RELA => { rel = ((*dyn).d_val + ldbase) as usize as *const u8; },
            DT_RELASZ => { relsz = (*dyn).d_val as isize; },
            DT_RELAENT => { relent = (*dyn).d_val as isize; },
            _ => {},
        }
        dyn = dyn.offset(1);
    }

    if rel.is_null() && relent == 0 {
        return efi_app::Status::success();
    }

    if rel.is_null() || relent == 0 {
        return efi_app::Status::load_error();
    }

    // Apply the relocs.

	while relsz > 0 {
	    let rela = &*(rel as *const Elf64_Rela);

		if rela.r_info == R_X86_64_RELATIVE {
			let addr = rela.r_offset.wrapping_add(ldbase) as usize as *mut u64;
			*addr = rela.r_addend.wrapping_add(ldbase);
		}

		rel = rel.offset(relent);
		relsz -= relent;
	}

    efi_app::Status::success()
}

