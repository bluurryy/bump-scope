#![cfg(test)]
#![expect(clippy::approx_constant)]

use hashbrown::HashMap;

type Bump = bump_scope::Bump;

#[test]
fn test() {
    let bump = Bump::new();
    let mut map = HashMap::new_in(&bump);
    map.insert("tau", 6.283);
}
