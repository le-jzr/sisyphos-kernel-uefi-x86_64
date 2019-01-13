
use x86_64::instructions::interrupts;
use x86_64::instructions::halt;

#[no_mangle]
pub extern "C" fn strlen(s: *const u8) -> usize {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    unsafe {
        interrupts::disable();
        loop {
            halt();
        }
    }
}

#[no_mangle]
pub extern "C" fn sqrt(n: f64) -> f64 {
    unimplemented!()
}
