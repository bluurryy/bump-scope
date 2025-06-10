#![no_main]

use fuzzing_support::bump_prepare_down::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| fuzz.run());
