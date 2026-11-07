#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod claim;

macro_rules! up_and_down {
    ($($tt:tt)*) => {
        mod prelude {
            #[allow(unused_imports)]
            pub(crate) use super::*;
            #[allow(unused_imports)]
            pub(crate) use $crate::{
                config::{BumpConfig, BumpConfigBuilder},
                bump_imp::{BumpImpTrait, AsBumpImp},
                extra,
            };
        }

        mod upwards {
            #[allow(unused_imports)]
            use super::prelude::*;

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(true)
                .build();

            $($tt)*
        }

        mod downwards {
            #[allow(unused_imports)]
            use super::prelude::*;

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(false)
                .build();

            $($tt)*
        }
    };
}

pub(crate) use up_and_down;
