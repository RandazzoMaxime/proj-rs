#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| proj_rs_fuzz::grid_reader(data));
