mod claim;

macro_rules! up_and_down {
    ($($tt:tt)*) => {
        mod upwards {
            #[allow(unused_imports)]
            use super::*;
            #[allow(unused_imports)]
            use $crate::{config::{BumpConfig, BumpConfigBuilder}, extra, vec};

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(true)
                .build();

            $($tt)*
        }

        mod downwards {
            #[allow(unused_imports)]
            use super::*;
            #[allow(unused_imports)]
            use $crate::{config::{BumpConfig, BumpConfigBuilder}, extra, vec};

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(false)
                .build();

            $($tt)*
        }
    };
}

pub(crate) use up_and_down;
