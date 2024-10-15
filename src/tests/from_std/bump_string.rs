use std::{
    assert_matches::assert_matches,
    cell::Cell,
    ops::{Bound, Bound::*, RangeBounds},
    panic, str,
};

use crate::{bump_format, Bump, BumpString, BumpVec};

#[test]
fn test_from_utf8() {
    let bump: Bump = Bump::new();

    let xs = BumpVec::from_array_in(*b"hello", &bump);
    assert_eq!(BumpString::from_utf8(xs).unwrap(), BumpString::from_str_in("hello", &bump));

    let mut xs = BumpVec::new_in(&bump);
    xs.extend_from_slice_copy("ศไทย中华Việt Nam".as_bytes());

    assert_eq!(
        BumpString::from_utf8(xs).unwrap(),
        BumpString::from_str_in("ศไทย中华Việt Nam", &bump)
    );

    let xs = BumpVec::from_array_in(*b"hello\xFF", &bump);
    let err = BumpString::from_utf8(xs).unwrap_err();
    assert_eq!(err.as_bytes(), b"hello\xff");
    assert_eq!(err.utf8_error().valid_up_to(), 5);
    assert_eq!(err.into_bytes(), BumpVec::from_array_in(*b"hello\xff", &bump));
}

#[test]
fn test_push_bytes() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("ABC", &bump);
    unsafe {
        let mv = s.as_mut_vec();
        mv.extend_from_slice_copy(&[b'D']);
    }
    assert_eq!(s, "ABCD");
}

#[test]
fn test_push_str() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::new_in(&bump);
    s.push_str("");
    assert_eq!(&s[0..], "");
    s.push_str("abc");
    assert_eq!(&s[0..], "abc");
    s.push_str("ประเทศไทย中华Việt Nam");
    assert_eq!(&s[0..], "abcประเทศไทย中华Việt Nam");
}

#[test]
fn test_add_assign() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::new_in(&bump);
    s += "";
    assert_eq!(s.as_str(), "");
    s += "abc";
    assert_eq!(s.as_str(), "abc");
    s += "ประเทศไทย中华Việt Nam";
    assert_eq!(s.as_str(), "abcประเทศไทย中华Việt Nam");
}

#[test]
fn test_push() {
    let bump: Bump = Bump::new();
    let mut data = BumpString::from_str_in("ประเทศไทย中", &bump);
    data.push('华');
    data.push('b'); // 1 byte
    data.push('¢'); // 2 byte
    data.push('€'); // 3 byte
    data.push('𤭢'); // 4 byte
    assert_eq!(data, "ประเทศไทย中华b¢€𤭢");
}

#[test]
fn test_pop() {
    let bump: Bump = Bump::new();
    let mut data = BumpString::from_str_in("ประเทศไทย中华b¢€𤭢", &bump);
    assert_eq!(data.pop().unwrap(), '𤭢'); // 4 bytes
    assert_eq!(data.pop().unwrap(), '€'); // 3 bytes
    assert_eq!(data.pop().unwrap(), '¢'); // 2 bytes
    assert_eq!(data.pop().unwrap(), 'b'); // 1 bytes
    assert_eq!(data.pop().unwrap(), '华');
    assert_eq!(data, "ประเทศไทย中");
}

#[test]
fn test_split_off_empty() {
    let bump: Bump = Bump::new();
    let orig = "Hello, world!";
    let mut split = BumpString::from_str_in(orig, &bump);
    let empty: BumpString = split.split_off(orig.len());
    assert!(empty.is_empty());
}

#[test]
#[should_panic]
fn test_split_off_past_end() {
    let bump: Bump = Bump::new();
    let orig = "Hello, world!";
    let mut split = BumpString::from_str_in(orig, &bump);
    let _ = split.split_off(orig.len() + 1);
}

#[test]
#[should_panic]
fn test_split_off_mid_char() {
    let bump: Bump = Bump::new();
    let mut shan = BumpString::from_str_in("山", &bump);
    let _broken_mountain = shan.split_off(1);
}

#[test]
fn test_split_off_ascii() {
    let bump: Bump = Bump::new();
    let mut ab = BumpString::from_str_in("ABCD", &bump);
    let orig_capacity = ab.capacity();
    let cd = ab.split_off(2);
    assert_eq!(ab, "AB");
    assert_eq!(cd, "CD");
    assert_eq!(ab.capacity(), orig_capacity);
}

#[test]
fn test_split_off_unicode() {
    let bump: Bump = Bump::new();
    let mut nihon = BumpString::from_str_in("日本語", &bump);
    let orig_capacity = nihon.capacity();
    let go = nihon.split_off("日本".len());
    assert_eq!(nihon, "日本");
    assert_eq!(go, "語");
    assert_eq!(nihon.capacity(), orig_capacity);
}

#[test]
fn test_str_truncate() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("12345", &bump);
    s.truncate(5);
    assert_eq!(s, "12345");
    s.truncate(3);
    assert_eq!(s, "123");
    s.truncate(0);
    assert_eq!(s, "");

    let mut s = BumpString::from_str_in("12345", &bump);
    let p = s.as_ptr();
    s.truncate(3);
    s.push_str("6");
    let p_ = s.as_ptr();
    assert_eq!(p_, p);
}

#[test]
fn test_str_truncate_invalid_len() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("12345", &bump);
    s.truncate(6);
    assert_eq!(s, "12345");
}

#[test]
#[should_panic]
fn test_str_truncate_split_codepoint() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("\u{FC}", &bump); // ü
    s.truncate(1);
}

#[test]
fn test_str_clear() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("12345", &bump);
    s.clear();
    assert_eq!(s.len(), 0);
    assert_eq!(s, "");
}

#[test]
fn test_str_add() {
    let bump: Bump = Bump::new();
    let a = BumpString::from_str_in("12345", &bump);
    let b = a + "2";
    let b = b + "2";
    assert_eq!(b.len(), 7);
    assert_eq!(b, "1234522");
}

#[test]
fn remove() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("ศไทย中华Việt Nam; foobar", &bump);
    assert_eq!(s.remove(0), 'ศ');
    assert_eq!(s.len(), 33);
    assert_eq!(s, "ไทย中华Việt Nam; foobar");
    assert_eq!(s.remove(17), 'ệ');
    assert_eq!(s, "ไทย中华Vit Nam; foobar");
}

#[test]
#[should_panic]
fn remove_bad() {
    "ศ".to_string().remove(1);
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("α_β_γ", &bump);

    s.retain(|_| true);
    assert_eq!(s, "α_β_γ");

    s.retain(|c| c != '_');
    assert_eq!(s, "αβγ");

    s.retain(|c| c != 'β');
    assert_eq!(s, "αγ");

    s.retain(|c| c == 'α');
    assert_eq!(s, "α");

    s.retain(|_| false);
    assert_eq!(s, "");

    let mut s = BumpString::from_str_in("0è0", &bump);
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let mut count = 0;
        s.retain(|_| {
            count += 1;
            match count {
                1 => false,
                2 => true,
                _ => panic!(),
            }
        });
    }));
    assert!(std::str::from_utf8(s.as_bytes()).is_ok());
}

#[test]
fn insert() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("foobar", &bump);
    s.insert(0, 'ệ');
    assert_eq!(s, "ệfoobar");
    s.insert(6, 'ย');
    assert_eq!(s, "ệfooยbar");
}

#[test]
#[should_panic]
fn insert_bad1() {
    let bump: Bump = Bump::new();
    BumpString::from_str_in("", &bump).insert(1, 't');
}
#[test]
#[should_panic]
fn insert_bad2() {
    let bump: Bump = Bump::new();
    BumpString::from_str_in("ệ", &bump).insert(1, 't');
}

#[test]
fn test_slicing() {
    let bump: Bump = Bump::new();
    let s = BumpString::from_str_in("foobar", &bump);
    assert_eq!("foobar", &s[..]);
    assert_eq!("foo", &s[..3]);
    assert_eq!("bar", &s[3..]);
    assert_eq!("oob", &s[1..4]);
}

#[test]
fn test_simple_types() {
    let bump: Bump = Bump::new();
    assert_eq!(bump_format!(in bump, "{}", 1), "1");
    assert_eq!(bump_format!(in bump, "{}", -1), "-1");
    assert_eq!(bump_format!(in bump, "{}", 200), "200");
    assert_eq!(bump_format!(in bump, "{}", 2), "2");
    assert_eq!(bump_format!(in bump, "{}", true), "true");
    assert_eq!(bump_format!(in bump, "{}", false), "false");
    assert_eq!(bump_format!(in bump, "{}", BumpString::from_str_in("hi", &bump)), "hi");
}

#[test]
fn test_vectors() {
    let bump: Bump = Bump::new();
    let x: Vec<i32> = vec![];
    assert_eq!(bump_format!(in bump, "{x:?}"), "[]");
    assert_eq!(bump_format!(in bump, "{:?}", vec![1]), "[1]");
    assert_eq!(bump_format!(in bump, "{:?}", vec![1, 2, 3]), "[1, 2, 3]");
    assert!(bump_format!(in bump, "{:?}", vec![vec![], vec![1], vec![1, 1]]) == "[[], [1], [1, 1]]");
}

#[test]
fn test_from_iterator() {
    let bump: Bump = Bump::new();
    let s = BumpString::from_str_in("ศไทย中华Việt Nam", &bump);
    let t = "ศไทย中华";
    let u = "Việt Nam";

    let a: String = s.chars().collect();
    assert_eq!(s, a);

    let mut b = t.to_string();
    b.extend(u.chars());
    assert_eq!(s, b);

    let c: String = [t, u].into_iter().collect();
    assert_eq!(s, c);

    let mut d = t.to_string();
    d.extend(vec![u]);
    assert_eq!(s, d);
}

#[test]
fn test_drain() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("αβγ", &bump);
    assert_eq!(s.drain(2..4).collect::<String>(), "β");
    assert_eq!(s, "αγ");

    let mut t = BumpString::from_str_in("abcd", &bump);
    t.drain(..0);
    assert_eq!(t, "abcd");
    t.drain(..1);
    assert_eq!(t, "bcd");
    t.drain(3..);
    assert_eq!(t, "bcd");
    t.drain(..);
    assert_eq!(t, "");
}

#[test]
#[should_panic]
fn test_drain_start_overflow() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("abc", &bump);
    s.drain((Excluded(usize::MAX), Included(0)));
}

#[test]
#[should_panic]
fn test_drain_end_overflow() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::from_str_in("abc", &bump);
    s.drain((Included(0), Included(usize::MAX)));
}

#[test]
fn test_extend_ref() {
    let mut a = "foo".to_string();
    a.extend(&['b', 'a', 'r']);

    assert_eq!(&a, "foobar");
}

#[test]
fn test_into_boxed_str() {
    let bump: Bump = Bump::new();
    let xs = BumpString::from_str_in("hello my name is bob", &bump);
    let ys = xs.into_boxed_str();
    assert_eq!(&*ys, "hello my name is bob");
}

#[test]
fn test_reserve() {
    // This is all the same as test_reserve
    let bump: Bump = Bump::new();
    let mut s = BumpString::new_in(&bump);
    assert_eq!(s.capacity(), 0);

    s.reserve(2);
    assert!(s.capacity() >= 2);

    for _i in 0..16 {
        s.push('0');
    }

    assert!(s.capacity() >= 16);
    s.reserve(16);
    assert!(s.capacity() >= 32);

    s.push('0');

    s.reserve(16);
    assert!(s.capacity() >= 33)
}

#[test]
fn test_from_char() {
    let bump: Bump = Bump::new();
    let mut s = BumpString::new_in(&bump);
    s.push('a');
    assert_eq!(s, "a");
    let mut s = BumpString::new_in(&bump);
    s.push('x');
    assert_eq!(s, "x");
}

#[test]
fn test_str_concat() {
    let bump: Bump = Bump::new();
    let a = BumpString::from_str_in("hello", &bump);
    let b = BumpString::from_str_in("world", &bump);
    let s = bump_format!(in bump, "{a}{b}");
    assert_eq!(s.as_bytes()[9], 'd' as u8);
}
