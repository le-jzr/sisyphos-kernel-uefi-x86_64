
use x86_64::registers::flags;
use x86_64::instructions::interrupts;

pub fn uninterruptible<T, F> (func: F) -> T
    where F: FnOnce() -> T
{
    let if_enabled = flags::flags().contains(flags::Flags::IF);

    unsafe {
        x86_64::instructions::interrupts::disable();
    }

    let result = func();

    if if_enabled {
        unsafe {
            x86_64::instructions::interrupts::enable();
        }
    }

    result
}


