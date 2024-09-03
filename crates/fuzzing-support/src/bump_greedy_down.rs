use arbitrary::Arbitrary;

use crate::{from_bump_scope, FuzzBumpGreedyProps};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpGreedyProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_down().to();
        from_bump_scope::bumping::bump_greedy_down(props);
    }
}
