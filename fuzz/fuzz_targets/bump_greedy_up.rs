#![no_main]

use fuzzing_support::bump_greedy_up::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| fuzz.run());
