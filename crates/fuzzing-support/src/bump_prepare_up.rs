use arbitrary::Arbitrary;

use crate::{FuzzBumpPropsPrepareUp, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpPropsPrepareUp,
}

impl Fuzz {
    pub fn run(self) {
        from_bump_scope::bumping::bump_prepare_up(self.props.0);
    }
}
