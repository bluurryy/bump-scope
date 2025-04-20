#![cfg(test)]

use bump_scope::Bump;
use hashbrown::HashMap;

#[test]
fn test() {
    let bump: Bump = Bump::new();
    let mut map = HashMap::new_in(&bump);
    map.insert("tau", 6.283);
}
