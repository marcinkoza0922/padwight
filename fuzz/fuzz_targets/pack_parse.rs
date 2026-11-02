#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(init: padwight_fuzz::fuzz_targets::allow_caught_font_panics(), |data: &[u8]| padwight_fuzz::fuzz_targets::pack_parse(data));
