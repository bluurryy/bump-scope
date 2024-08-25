use arbitrary::Arbitrary;

use crate::{from_bump_scope, FuzzBumpProps};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_down().to();
        unsafe { from_bump_scope::bumping::bump_down(props) };
    }
}