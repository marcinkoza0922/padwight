#![no_main]

use libfuzzer_sys::fuzz_target;

// `PADWIGHT_FUZZ_SEEDS=dir cargo fuzz run config_load -- -runs=0` writes the seed corpus.
fuzz_target!(init: {
    if let Some(dir) = std::env::var_os("PADWIGHT_FUZZ_SEEDS") {
        padwight_fuzz::fuzz_targets::write_config_seeds(std::path::Path::new(&dir));
        std::process::exit(0);
    }
}, |data: &[u8]| padwight_fuzz::fuzz_targets::config_load(data));
