#![no_main]

use fuzzing_support::allocator_api::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| {
    env_logger::init();
    fuzz.run()
});
