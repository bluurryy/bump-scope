#![no_main]

use fuzz_lib::bump_prepare_up::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| fuzz.run());
