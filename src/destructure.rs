/// Allows you to destructure structs that have a drop implementation.
///
/// The drop implementation will not be called for `$ty` nor for any field that is not bound.
macro_rules! destructure {
    (let $ty:ty {
        $($field:ident $(: $field_alias:ident)?),* $(,)?
    } = $value:expr) => {
        let value: $ty = $value;
        let value = ::core::mem::ManuallyDrop::new(value);

        const _: () = assert!(!$crate::destructure::has_duplicates(&[$(stringify!($field)),*]), "you can't destructure a field twice");

        $(
            #[allow(unused_unsafe)] // we might or might not already be in an unsafe context
            let $crate::destructure::or!($($field_alias)? $field) = unsafe { ::core::ptr::read(&value.$field) };
        )*
    };
}

pub(crate) use destructure;

macro_rules! or {
    ($this:ident $that:ident) => {
        $this
    };
    ($that:ident) => {
        $that
    };
}

pub(crate) use or;

pub(crate) const fn has_duplicates(strings: &[&str]) -> bool {
    let mut x = 0;

    while x < strings.len() {
        let mut y = x + 1;

        while y < strings.len() {
            if str_eq(strings[x], strings[y]) {
                return true;
            }

            y += 1;
        }

        x += 1;
    }

    false
}

const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();

    if a.len() != b.len() {
        return false;
    }

    let mut i = 0;

    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }

        i += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    use std::string::String;

    #[test]
    fn example() {
        pub struct Foo {
            bar: String,
            baz: String,
        }

        impl Drop for Foo {
            fn drop(&mut self) {
                unreachable!()
            }
        }

        let foo = Foo {
            bar: "bar".into(),
            baz: "baz".into(),
        };

        // won't compile
        // let Foo { bar: qux, baz } = foo;

        destructure!(let Foo { bar: qux, baz } = foo);

        assert_eq!(qux, "bar");
        assert_eq!(baz, "baz");
    }
}
