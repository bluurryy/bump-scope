#![no_main]

use fuzzing_support::alloc_static_layout::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| fuzz.run());
