use arbitrary::Arbitrary;

use crate::{FuzzBumpPropsDown, from_bump_scope};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    props: FuzzBumpPropsDown,
}

impl Fuzz {
    pub fn run(self) {
        from_bump_scope::bumping::bump_down(self.props.0);
    }
}
