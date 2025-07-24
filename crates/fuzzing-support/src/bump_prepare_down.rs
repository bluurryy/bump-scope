use arbitrary::Arbitrary;

use crate::{FuzzBumpProps, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_prepare().for_down().to();
        from_bump_scope::bumping::bump_prepare_down(props);
    }
}
