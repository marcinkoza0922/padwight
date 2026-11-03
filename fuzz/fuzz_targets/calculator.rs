#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| padwight_fuzz::fuzz_targets::calculator(data));
