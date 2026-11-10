#![no_main]

use fuzz_lib::alloc_static_layout::Fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|fuzz: Fuzz| fuzz.run());
