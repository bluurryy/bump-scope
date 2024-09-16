use crate::{from_bump_scope, FuzzBumpProps};
use arbitrary::Arbitrary;

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_up().to();
        from_bump_scope::bumping::bump_up(props);
    }
}
