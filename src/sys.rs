
use std::os::dynamic;
use std::io;
use std::vec;

use std::cell::RefCell;

use std::ptr::null_mut;

use std::time::Duration;

use efi_app;

struct Console;

impl dynamic::Stdout for Console {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        let out = unsafe { efi_app::__fixme_temporary_out() };
        let _r = out.output_bytes(data);
        Ok(())
    }
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

struct DummyMutex;

impl dynamic::Mutex for DummyMutex {
    fn new(&self) -> dynamic::MutexHandle {
        dynamic::MutexHandle::uninitialized()
    }

    unsafe fn destroy(&self, m: dynamic::MutexHandle) {
    }

    unsafe fn lock(&self, m: dynamic::MutexHandle) {
    }

    unsafe fn try_lock(&self, m: dynamic::MutexHandle) -> bool {
        true
    }

    unsafe fn unlock(&self, m: dynamic::MutexHandle) {
    }
}

struct DummyRwLock;

impl dynamic::RwLock for DummyRwLock {
    fn new(&self) -> dynamic::RwLockHandle {
        dynamic::RwLockHandle::uninitialized()
    }

    unsafe fn destroy(&self, m: dynamic::RwLockHandle) {}
    unsafe fn read(&self, m: dynamic::RwLockHandle) {}
    unsafe fn try_read(&self, m: dynamic::RwLockHandle) -> bool { true }
    unsafe fn read_unlock(&self, m: dynamic::RwLockHandle) {}
    unsafe fn write(&self, m: dynamic::RwLockHandle) {}
    unsafe fn try_write(&self, m: dynamic::RwLockHandle) -> bool { true }
    unsafe fn write_unlock(&self, m: dynamic::RwLockHandle) {}
}

struct DummyCvar;

impl dynamic::Condvar for DummyCvar {
    fn new(&self) -> dynamic::CondvarHandle {
        dynamic::CondvarHandle::uninitialized()
    }

    unsafe fn destroy(&self, cv: dynamic::CondvarHandle) {}
    fn notify_one(&self, cv: dynamic::CondvarHandle) {}
    fn notify_all(&self, cv: dynamic::CondvarHandle) {}
    fn wait(&self, cv: dynamic::CondvarHandle, m: dynamic::MutexHandle) {}
    fn wait_timeout(&self, cv: dynamic::CondvarHandle, m: dynamic::MutexHandle, dur: Duration) -> bool { true }
}

struct DummyTls {
    data: RefCell<vec::Vec<*mut u8>>,
}

impl DummyTls {
    fn new() -> Self {
        DummyTls { data: RefCell::new(vec::Vec::new()) }
    }
}

// Not really, but we are assuming single-threaded operation here.
unsafe impl Sync for DummyTls {}

impl dynamic::ThreadLocal for DummyTls {
    unsafe fn create(&self, dtor: Option<unsafe extern fn(*mut u8)>) -> dynamic::ThreadLocalKey {
        let mut vec = self.data.borrow_mut();
        let key = vec.len();
        vec.push(null_mut());
        key
    }

    unsafe fn set(&self, key: dynamic::ThreadLocalKey, value: *mut u8) {
        self.data.borrow_mut()[key] = value;
    }

    unsafe fn get(&self, key: dynamic::ThreadLocalKey) -> *mut u8 {
        self.data.borrow()[key]
    }

    unsafe fn destroy(&self, key: dynamic::ThreadLocalKey) {
        // meh
    }
}

#[allow(unused_must_use)]
pub fn init() {
    dynamic::STDERR.initialize(Box::new(Console));
    dynamic::STDOUT.initialize(Box::new(Console));
    dynamic::MUTEX.initialize(Box::new(DummyMutex));
    dynamic::REENTRANT_MUTEX.initialize(Box::new(DummyMutex));
    dynamic::THREAD_LOCAL.initialize(Box::new(DummyTls::new()));
    dynamic::RWLOCK.initialize(Box::new(DummyRwLock));
    dynamic::CONDVAR.initialize(Box::new(DummyCvar));
}
