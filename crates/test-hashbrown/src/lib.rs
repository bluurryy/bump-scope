#![cfg(test)]
#![expect(clippy::approx_constant)]

use bump_scope::alloc::Global;
use hashbrown::HashMap;

type Bump = bump_scope::Bump<Global, 1, true, true, true>;

#[test]
fn test() {
    let bump = Bump::new();
    let mut map = HashMap::new_in(&bump);
    map.insert("tau", 6.283);
}
