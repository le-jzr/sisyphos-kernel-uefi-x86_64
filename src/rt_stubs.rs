/*
#[lang = "eh_personality"]
extern "C" fn eh_personality() {
    // TODO
}
*/

#[no_mangle]
pub extern "C" fn __floatundisf(_a: u64) -> f32 {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn __mulsf3(_a: f32, _b: f32) -> f32 {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn __muldf3(_a: f64, _b: f64) -> f64 {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn __divsf3(_a: f32, _b: f32) -> f32 {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn __divdf3(_a: f64, _b: f64) -> f64 {
    unimplemented!()
}

