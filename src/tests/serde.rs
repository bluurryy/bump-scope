use ::serde::Serialize;

use crate::{bump_format, bump_vec};

use super::*;

fn assert_same<A: Serialize, B: Serialize>(a: &A, b: &B) {
    let a_json = serde_json::to_string(a).unwrap();
    let b_json = serde_json::to_string(b).unwrap();
    assert_eq!(a_json, b_json);
}

#[test]
fn ser() {
    let mut bump: Bump = Bump::new();

    {
        let a = bump.alloc_str("Hello world!");
        let b = "Hello world!";
        assert_same(&a, &b);
    }

    {
        let mut a = bump.alloc_fixed_vec(5);
        a.extend_from_slice_copy(&[1, 2, 3]);
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = bump_vec![in bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_vec![in bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_vec_rev![in bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = bump_format!(in bump, "Hello world!");
        let b = "Hello world!";
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_format!(in bump, "Hello world!");
        let b = "Hello world!";
        assert_same(&a, &b);
    }
}
