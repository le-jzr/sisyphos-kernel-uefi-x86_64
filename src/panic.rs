
use efi_app;
use core::fmt;

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(args: fmt::Arguments, s: &'static str, line: u32) -> ! {
    let out: &mut fmt::Write = unsafe { efi_app::__fixme_temporary_out() };
    out.write_fmt(format_args!("Panic in \'{}\' (line {}):\n", s, line));
    out.write_fmt(args);
    loop{}
}

