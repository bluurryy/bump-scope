#![allow(clippy::pedantic)]
#![allow(clippy::toplevel_ref_arg)]

/// This is nightly's `Iterator::partition_in_place`.
pub(crate) fn partition_in_place<'a, T: 'a, P>(
    mut iter: impl DoubleEndedIterator<Item = &'a mut T>,
    ref mut predicate: P,
) -> usize
where
    P: FnMut(&T) -> bool,
{
    // FIXME: should we worry about the count overflowing? The only way to have more than
    // `usize::MAX` mutable references is with ZSTs, which aren't useful to partition...

    // These closure "factory" functions exist to avoid genericity in `Self`.

    #[inline]
    fn is_false<'a, T>(
        predicate: &'a mut impl FnMut(&T) -> bool,
        true_count: &'a mut usize,
    ) -> impl FnMut(&&mut T) -> bool + 'a {
        move |x| {
            let p = predicate(&**x);
            *true_count += p as usize;
            !p
        }
    }

    #[inline]
    fn is_true<T>(predicate: &mut impl FnMut(&T) -> bool) -> impl FnMut(&&mut T) -> bool + '_ {
        move |x| predicate(&**x)
    }

    // Repeatedly find the first `false` and swap it with the last `true`.
    let mut true_count = 0;
    while let Some(head) = iter.find(is_false(predicate, &mut true_count)) {
        if let Some(tail) = iter.rfind(is_true(predicate)) {
            crate::mem::swap(head, tail);
            true_count += 1;
        } else {
            break;
        }
    }
    true_count
}
