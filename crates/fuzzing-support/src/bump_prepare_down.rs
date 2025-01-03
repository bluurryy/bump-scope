use arbitrary::Arbitrary;

use crate::{from_bump_scope, FuzzBumpPrepareProps};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpPrepareProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_down().to();
        from_bump_scope::bumping::bump_prepare_down(props);
    }
}
