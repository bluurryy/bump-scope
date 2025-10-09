/// Allows you to destructure structs that have a drop implementation.
///
/// The drop implementation will not be called for `$ty` nor for any field that is not bound.
macro_rules! destructure {
    (let $ty:path {
        $($field:ident $(: $field_alias:ident)?),* $(,)?
    } = $value:expr) => {
        let value: $ty = $value;

        // errors if there are duplicates
        let $ty { $($field: _,)* .. } = &value;

        let value = ::core::mem::ManuallyDrop::new(value);

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

        // won't compile
        // destructure!(let Foo { bar, bar } = foo);

        destructure!(let Foo { bar: qux, baz } = foo);

        assert_eq!(qux, "bar");
        assert_eq!(baz, "baz");
    }
}
