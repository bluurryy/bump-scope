use crate::{Bump, BumpVec, MutBumpVec, MutBumpVecRev};

#[test]
fn grow_vec() {
    let bump: Bump = Bump::new();
    let mut vec = BumpVec::new_in(&bump);
    let mut numbers = number_strings();

    vec.push(numbers.next().unwrap());

    let ptr = vec.as_ptr();

    while vec.as_ptr() == ptr {
        vec.push(numbers.next().unwrap());
    }

    assert!(vec.iter().cloned().eq(number_strings().take(vec.len())));
}

#[test]
fn grow_mut_vec() {
    let mut bump: Bump = Bump::new();
    let mut vec = MutBumpVec::new_in(&mut bump);
    let mut numbers = number_strings();

    vec.push(numbers.next().unwrap());

    let ptr = vec.as_ptr();

    while vec.as_ptr() == ptr {
        vec.push(numbers.next().unwrap());
    }

    assert!(vec.iter().cloned().eq(number_strings().take(vec.len())));
}

#[test]
fn grow_mut_vec_rev() {
    let mut bump: Bump = Bump::new();
    let mut vec = MutBumpVecRev::new_in(&mut bump);
    let mut numbers = number_strings();

    vec.push(numbers.next().unwrap());

    let ptr = vec.as_ptr();

    while vec.as_ptr() == ptr {
        vec.push(numbers.next().unwrap());
    }

    assert!(vec
        .iter()
        .cloned()
        .eq(number_strings().take(vec.len()).collect::<Vec<_>>().into_iter().rev()));
}

fn number_strings() -> impl Iterator<Item = String> {
    (0..).map(|i| i.to_string())
}
