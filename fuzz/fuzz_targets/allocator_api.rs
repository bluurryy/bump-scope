#![no_main]

use fuzz_lib::allocator_api::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| {
    _ = env_logger::try_init();
    fuzz.run()
});
