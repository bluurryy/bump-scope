#![no_main]

use fuzzing_support::allocator_api::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| {
    _ = env_logger::try_init();
    fuzz.run()
});
