use arbitrary::Arbitrary;

use crate::{FuzzBumpPropsPrepareDown, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpPropsPrepareDown,
}

impl Fuzz {
    pub fn run(self) {
        from_bump_scope::bumping::bump_prepare_down(self.props.0);
    }
}
