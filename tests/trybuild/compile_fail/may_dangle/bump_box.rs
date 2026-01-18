//! When we add the `#[may_dangle]` attribute via the "nightly-dropck-eyepatch" feature
//! we need to make sure `T` is still marked as owned and thus as being dropped by the container.
//! We do this by having a `PhantomData<T>` field.
//! If we didn't do this, then this would compile and cause UB because were
//! accessing a dangling reference (use after free).

use core::fmt::Debug;

use bump_scope::Bump;

struct PrintOnDrop<T: Debug>(T);

impl<T: Debug> Drop for PrintOnDrop<T> {
    fn drop(&mut self) {
        std::println!("dropping: {:?}", self.0);
    }
}

fn dangling_reference(bump: &mut Bump) {
    let mut b = bump.alloc(None);
    let s = String::from("hello");
    *b = Some(PrintOnDrop(&s));
}

fn main() {}
