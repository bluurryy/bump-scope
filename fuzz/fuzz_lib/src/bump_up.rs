use arbitrary::Arbitrary;

use crate::{FuzzBumpPropsUp, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpPropsUp,
}

impl Fuzz {
    pub fn run(self) {
        if let Some(from_bump_scope::bumping::BumpUp { ptr, new_pos }) = from_bump_scope::bumping::bump_up(self.props.0) {
            _ = (ptr, new_pos);
        }
    }
}
