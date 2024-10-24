/// Allows you to destructure structs that have a drop implementation.
macro_rules! destructure {
    (let {
        $($field:ident $(: $field_alias:ident)?),* $(,)?
    } = $value:ident) => {
        let mut maybe_uninit = ::core::mem::MaybeUninit::new($value);

        #[allow(dead_code)]
        const _: () = assert!(!$crate::destructure::has_duplicates(&[$(stringify!($field)),*]), "you can't destructure a field twice");

        $(
            let $crate::destructure::or!($($field_alias)? $field) = unsafe { ::core::ptr::addr_of_mut!((*maybe_uninit.as_mut_ptr()).$field).read() };
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
    use super::*;

    #[test]
    fn example() {
        struct Foo {
            bar: i32,
            baz: i64,
        }

        let foo = Foo { bar: 3, baz: 5 };

        destructure!(let { bar: qux, baz } = foo);

        assert_eq!(qux, 3);
        assert_eq!(baz, 5);
    }
}
