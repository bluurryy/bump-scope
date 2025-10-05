use arbitrary::Arbitrary;

use crate::{FuzzBumpProps, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpProps,
}

impl Fuzz {
    pub fn run(self) {
        let props = self.props.for_up().to();
        if let Some(from_bump_scope::bumping::BumpUp { ptr, new_pos }) = from_bump_scope::bumping::bump_up(props) {
            _ = (ptr, new_pos);
        }
    }
}
