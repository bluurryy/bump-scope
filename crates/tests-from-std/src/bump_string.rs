//! Adapted from rust's `library/alloctests/tests/vec.rs` commit fb04372dc56129d69e39af80cac6e81694bd285f

use std::cell::Cell;
use std::ops::Bound::*;
use std::ops::{Bound, RangeBounds};
use std::string::String as StdString;
use std::{panic, str};

use bump_scope::{Bump, bump_format, bump_vec};

type Vec<T, A = Bump> = bump_scope::BumpVec<T, A>;
type String<A> = bump_scope::BumpString<A>;

macro_rules! vec {
    (in $($tt:tt)*) => {
        bump_scope::bump_vec![in $($tt)*]
    };
    ($($tt:tt)*) => {
        bump_scope::bump_vec![in <bump_scope::Bump>::default(); $($tt)*]
    };
}

#[cfg(any())] // not applicable
fn test_from_str() {}

#[cfg(any())] // not applicable
fn test_from_cow_str() {}

#[cfg(any())] // not applicable
fn test_unsized_to_string() {}

#[test]
fn test_from_utf8() {
    let bump: Bump = Bump::new();

    let xs = Vec::from_array_in(*b"hello", &bump);
    assert_eq!(String::from_utf8(xs).unwrap(), String::from_str_in("hello", &bump));

    let mut xs = Vec::new_in(&bump);
    xs.extend_from_slice_copy("ศไทย中华Việt Nam".as_bytes());

    assert_eq!(String::from_utf8(xs).unwrap(), String::from_str_in("ศไทย中华Việt Nam", &bump));

    let xs = Vec::from_array_in(*b"hello\xFF", &bump);
    let err = String::from_utf8(xs).unwrap_err();
    assert_eq!(err.as_bytes(), b"hello\xff");
    assert_eq!(err.utf8_error().valid_up_to(), 5);
    assert_eq!(err.into_bytes(), Vec::from_array_in(*b"hello\xff", &bump));
}

#[test]
fn test_from_utf8_lossy() {
    let bump: Bump = Bump::new();

    let xs = b"hello";
    let ys = "hello";
    assert_eq!(String::from_utf8_lossy_in(xs, &bump), ys);

    let xs = "ศไทย中华Việt Nam".as_bytes();
    let ys = "ศไทย中华Việt Nam";
    assert_eq!(String::from_utf8_lossy_in(xs, &bump), ys);

    let xs = b"Hello\xC2 There\xFF Goodbye";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("Hello\u{FFFD} There\u{FFFD} Goodbye", &bump)
    );

    let xs = b"Hello\xC0\x80 There\xE6\x83 Goodbye";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("Hello\u{FFFD}\u{FFFD} There\u{FFFD} Goodbye", &bump)
    );

    let xs = b"\xF5foo\xF5\x80bar";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("\u{FFFD}foo\u{FFFD}\u{FFFD}bar", &bump)
    );

    let xs = b"\xF1foo\xF1\x80bar\xF1\x80\x80baz";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("\u{FFFD}foo\u{FFFD}bar\u{FFFD}baz", &bump)
    );

    let xs = b"\xF4foo\xF4\x80bar\xF4\xBFbaz";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("\u{FFFD}foo\u{FFFD}bar\u{FFFD}\u{FFFD}baz", &bump)
    );

    let xs = b"\xF0\x80\x80\x80foo\xF0\x90\x80\x80bar";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}foo\u{10000}bar", &bump)
    );

    // surrogates
    let xs = b"\xED\xA0\x80foo\xED\xBF\xBFbar";
    assert_eq!(
        String::from_utf8_lossy_in(xs, &bump),
        String::from_str_in("\u{FFFD}\u{FFFD}\u{FFFD}foo\u{FFFD}\u{FFFD}\u{FFFD}bar", &bump)
    );
}

#[cfg(any())] // TODO: implement
#[test]
fn test_fromutf8error_into_lossy() {
    fn func(input: &[u8]) -> String {
        String::from_utf8(input.to_owned()).unwrap_or_else(|e| e.into_utf8_lossy())
    }

    let xs = b"hello";
    let ys = "hello".to_owned();
    assert_eq!(func(xs), ys);

    let xs = "ศไทย中华Việt Nam".as_bytes();
    let ys = "ศไทย中华Việt Nam".to_owned();
    assert_eq!(func(xs), ys);

    let xs = b"Hello\xC2 There\xFF Goodbye";
    assert_eq!(func(xs), "Hello\u{FFFD} There\u{FFFD} Goodbye".to_owned());

    let xs = b"Hello\xC0\x80 There\xE6\x83 Goodbye";
    assert_eq!(func(xs), "Hello\u{FFFD}\u{FFFD} There\u{FFFD} Goodbye".to_owned());

    let xs = b"\xF5foo\xF5\x80bar";
    assert_eq!(func(xs), "\u{FFFD}foo\u{FFFD}\u{FFFD}bar".to_owned());

    let xs = b"\xF1foo\xF1\x80bar\xF1\x80\x80baz";
    assert_eq!(func(xs), "\u{FFFD}foo\u{FFFD}bar\u{FFFD}baz".to_owned());

    let xs = b"\xF4foo\xF4\x80bar\xF4\xBFbaz";
    assert_eq!(func(xs), "\u{FFFD}foo\u{FFFD}bar\u{FFFD}\u{FFFD}baz".to_owned());

    let xs = b"\xF0\x80\x80\x80foo\xF0\x90\x80\x80bar";
    assert_eq!(func(xs), "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}foo\u{10000}bar".to_owned());

    // surrogates
    let xs = b"\xED\xA0\x80foo\xED\xBF\xBFbar";
    assert_eq!(func(xs), "\u{FFFD}\u{FFFD}\u{FFFD}foo\u{FFFD}\u{FFFD}\u{FFFD}bar".to_owned());
}

#[test]
fn test_from_utf16() {
    let bump: Bump = Bump::new();

    let pairs = [
        (
            String::from_str_in("𐍅𐌿𐌻𐍆𐌹𐌻𐌰\n", &bump),
            vec![in &bump;
                0xd800, 0xdf45, 0xd800, 0xdf3f, 0xd800, 0xdf3b, 0xd800, 0xdf46, 0xd800, 0xdf39,
                0xd800, 0xdf3b, 0xd800, 0xdf30, 0x000a,
            ],
        ),
        (
            String::from_str_in("𐐒𐑉𐐮𐑀𐐲𐑋 𐐏𐐲𐑍\n", &bump),
            vec![in &bump;
                0xd801, 0xdc12, 0xd801, 0xdc49, 0xd801, 0xdc2e, 0xd801, 0xdc40, 0xd801, 0xdc32,
                0xd801, 0xdc4b, 0x0020, 0xd801, 0xdc0f, 0xd801, 0xdc32, 0xd801, 0xdc4d, 0x000a,
            ],
        ),
        (
            String::from_str_in("𐌀𐌖𐌋𐌄𐌑𐌉·𐌌𐌄𐌕𐌄𐌋𐌉𐌑\n", &bump),
            vec![in &bump;
                0xd800, 0xdf00, 0xd800, 0xdf16, 0xd800, 0xdf0b, 0xd800, 0xdf04, 0xd800, 0xdf11,
                0xd800, 0xdf09, 0x00b7, 0xd800, 0xdf0c, 0xd800, 0xdf04, 0xd800, 0xdf15, 0xd800,
                0xdf04, 0xd800, 0xdf0b, 0xd800, 0xdf09, 0xd800, 0xdf11, 0x000a,
            ],
        ),
        (
            String::from_str_in("𐒋𐒘𐒈𐒑𐒛𐒒 𐒕𐒓 𐒈𐒚𐒍 𐒏𐒜𐒒𐒖𐒆 𐒕𐒆\n", &bump),
            vec![in &bump;
                0xd801, 0xdc8b, 0xd801, 0xdc98, 0xd801, 0xdc88, 0xd801, 0xdc91, 0xd801, 0xdc9b,
                0xd801, 0xdc92, 0x0020, 0xd801, 0xdc95, 0xd801, 0xdc93, 0x0020, 0xd801, 0xdc88,
                0xd801, 0xdc9a, 0xd801, 0xdc8d, 0x0020, 0xd801, 0xdc8f, 0xd801, 0xdc9c, 0xd801,
                0xdc92, 0xd801, 0xdc96, 0xd801, 0xdc86, 0x0020, 0xd801, 0xdc95, 0xd801, 0xdc86,
                0x000a,
            ],
        ),
        // Issue #12318, even-numbered non-BMP planes
        (String::from_str_in("\u{20000}", &bump), vec![in &bump; 0xD840, 0xDC00]),
    ];

    for p in &pairs {
        let (s, u) = (*p).clone();
        let s_as_utf16 = s.encode_utf16().collect::<Vec<_>>();
        let u_as_string = String::from_utf16_in(&u, &bump).unwrap();

        assert!(core::char::decode_utf16(u.iter().cloned()).all(|r| r.is_ok()));
        assert_eq!(s_as_utf16, u);

        assert_eq!(u_as_string, s);
        assert_eq!(String::from_utf16_lossy_in(&u, &bump), s);

        assert_eq!(String::from_utf16_in(&s_as_utf16, &bump).unwrap(), s);
        assert_eq!(u_as_string.encode_utf16().collect::<Vec<u16>>(), u);
    }
}

#[test]
fn test_utf16_invalid() {
    let bump: Bump = Bump::new();

    // completely positive cases tested above.
    // lead + eof
    assert!(String::from_utf16_in(&[0xD800], &bump).is_err());
    // lead + lead
    assert!(String::from_utf16_in(&[0xD800, 0xD800], &bump).is_err());

    // isolated trail
    assert!(String::from_utf16_in(&[0x0061, 0xDC00], &bump).is_err());

    // general
    assert!(String::from_utf16_in(&[0xD800, 0xd801, 0xdc8b, 0xD800], &bump).is_err());
}

#[test]
fn test_from_utf16_lossy() {
    let bump: Bump = Bump::new();

    // completely positive cases tested above.
    // lead + eof
    assert_eq!(
        String::from_utf16_lossy_in(&[0xD800], &bump),
        String::from_str_in("\u{FFFD}", &bump)
    );
    // lead + lead
    assert_eq!(
        String::from_utf16_lossy_in(&[0xD800, 0xD800], &bump),
        String::from_str_in("\u{FFFD}\u{FFFD}", &bump)
    );

    // isolated trail
    assert_eq!(
        String::from_utf16_lossy_in(&[0x0061, 0xDC00], &bump),
        String::from_str_in("a\u{FFFD}", &bump)
    );

    // general
    assert_eq!(
        String::from_utf16_lossy_in(&[0xD800, 0xd801, 0xdc8b, 0xD800], &bump),
        String::from_str_in("\u{FFFD}𐒋\u{FFFD}", &bump)
    );
}

#[test]
fn test_push_bytes() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("ABC", &bump);
    unsafe {
        let mv = s.as_mut_vec();
        mv.extend_from_slice_copy(&[b'D']);
    }
    assert_eq!(s, "ABCD");
}

#[test]
fn test_push_str() {
    let bump: Bump = Bump::new();
    let mut s = String::new_in(&bump);
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
    let mut s = String::new_in(&bump);
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
    let mut data = String::from_str_in("ประเทศไทย中", &bump);
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
    let mut data = String::from_str_in("ประเทศไทย中华b¢€𤭢", &bump);
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
    let mut split = String::from_str_in(orig, &bump);
    let empty: String<_> = split.split_off(orig.len()..);
    assert!(empty.is_empty());
}

#[test]
#[should_panic]
fn test_split_off_past_end() {
    let bump: Bump = Bump::new();
    let orig = "Hello, world!";
    let mut split = String::from_str_in(orig, &bump);
    let _ = split.split_off(orig.len() + 1..);
}

#[test]
#[should_panic]
fn test_split_off_mid_char() {
    let bump: Bump = Bump::new();
    let mut shan = String::from_str_in("山", &bump);
    let _broken_mountain = shan.split_off(1..);
}

#[test]
fn test_split_off_ascii() {
    let bump: Bump = Bump::new();
    let mut ab = String::from_str_in("ABCD", &bump);
    let orig_capacity = ab.capacity();
    let cd = ab.split_off(2..);
    assert_eq!(ab, "AB");
    assert_eq!(cd, "CD");
    assert_eq!(ab.capacity(), ab.len());
    assert_eq!(cd.capacity(), orig_capacity - ab.len());
}

#[test]
fn test_split_off_unicode() {
    let bump: Bump = Bump::new();
    let mut nihon = String::from_str_in("日本語", &bump);
    let orig_capacity = nihon.capacity();
    let go = nihon.split_off("日本".len()..);
    assert_eq!(nihon, "日本");
    assert_eq!(go, "語");

    // It's not guaranteed that these assertions succeed
    // but they will in the current implementation.
    assert_eq!(nihon.capacity(), nihon.len());
    assert_eq!(go.capacity(), orig_capacity - nihon.len());
}

#[test]
fn test_str_truncate() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.truncate(5);
    assert_eq!(s, "12345");
    s.truncate(3);
    assert_eq!(s, "123");
    s.truncate(0);
    assert_eq!(s, "");

    let mut s = String::from_str_in("12345", &bump);
    let p = s.as_ptr();
    s.truncate(3);
    s.push_str("6");
    let p_ = s.as_ptr();
    assert_eq!(p_, p);
}

#[test]
fn test_str_truncate_invalid_len() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.truncate(6);
    assert_eq!(s, "12345");
}

#[test]
#[should_panic]
fn test_str_truncate_split_codepoint() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("\u{FC}", &bump); // ü
    s.truncate(1);
}

#[test]
fn test_str_clear() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.clear();
    assert_eq!(s.len(), 0);
    assert_eq!(s, "");
}

#[test]
fn test_str_add() {
    let bump: Bump = Bump::new();
    let a = String::from_str_in("12345", &bump);
    let b = a + "2";
    let b = b + "2";
    assert_eq!(b.len(), 7);
    assert_eq!(b, "1234522");
}

#[test]
fn remove() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("ศไทย中华Việt Nam; foobar", &bump);
    assert_eq!(s.remove(0), 'ศ');
    assert_eq!(s.len(), 33);
    assert_eq!(s, "ไทย中华Việt Nam; foobar");
    assert_eq!(s.remove(17), 'ệ');
    assert_eq!(s, "ไทย中华Vit Nam; foobar");
}

#[test]
#[should_panic]
fn remove_bad() {
    let bump: Bump = Bump::new();
    String::from_str_in("ศ", &bump).remove(1);
}

#[cfg(any())] // TODO: implement `remove_matches`
#[test]
fn test_remove_matches() {
    // test_single_pattern_occurrence
    let mut s = "abc".to_string();
    s.remove_matches('b');
    assert_eq!(s, "ac");
    // repeat_test_single_pattern_occurrence
    s.remove_matches('b');
    assert_eq!(s, "ac");

    // test_single_character_pattern
    let mut s = "abcb".to_string();
    s.remove_matches('b');
    assert_eq!(s, "ac");

    // test_pattern_with_special_characters
    let mut s = "ศไทย中华Việt Nam; foobarศ".to_string();
    s.remove_matches('ศ');
    assert_eq!(s, "ไทย中华Việt Nam; foobar");

    // test_pattern_empty_text_and_pattern
    let mut s = "".to_string();
    s.remove_matches("");
    assert_eq!(s, "");

    // test_pattern_empty_text
    let mut s = "".to_string();
    s.remove_matches("something");
    assert_eq!(s, "");

    // test_empty_pattern
    let mut s = "Testing with empty pattern.".to_string();
    s.remove_matches("");
    assert_eq!(s, "Testing with empty pattern.");

    // test_multiple_consecutive_patterns_1
    let mut s = "aaaaa".to_string();
    s.remove_matches('a');
    assert_eq!(s, "");

    // test_multiple_consecutive_patterns_2
    let mut s = "Hello **world****today!**".to_string();
    s.remove_matches("**");
    assert_eq!(s, "Hello worldtoday!");

    // test_case_insensitive_pattern
    let mut s = "CASE ** SeNsItIvE ** PaTtErN.".to_string();
    s.remove_matches("sEnSiTiVe");
    assert_eq!(s, "CASE ** SeNsItIvE ** PaTtErN.");

    // test_pattern_with_digits
    let mut s = "123 ** 456 ** 789".to_string();
    s.remove_matches("**");
    assert_eq!(s, "123  456  789");

    // test_pattern_occurs_after_empty_string
    let mut s = "abc X defXghi".to_string();
    s.remove_matches("X");
    assert_eq!(s, "abc  defghi");

    // test_large_pattern
    let mut s = "aaaXbbbXcccXdddXeee".to_string();
    s.remove_matches("X");
    assert_eq!(s, "aaabbbcccdddeee");

    // test_pattern_at_multiple_positions
    let mut s = "Pattern ** found ** multiple ** times ** in ** text.".to_string();
    s.remove_matches("**");
    assert_eq!(s, "Pattern  found  multiple  times  in  text.");
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("α_β_γ", &bump);

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

    let mut s = String::from_str_in("0è0", &bump);
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
    let mut s = String::from_str_in("foobar", &bump);
    s.insert(0, 'ệ');
    assert_eq!(s, "ệfoobar");
    s.insert(6, 'ย');
    assert_eq!(s, "ệfooยbar");
}

#[test]
#[should_panic]
fn insert_bad1() {
    let bump: Bump = Bump::new();
    String::from_str_in("", &bump).insert(1, 't');
}
#[test]
#[should_panic]
fn insert_bad2() {
    let bump: Bump = Bump::new();
    String::from_str_in("ệ", &bump).insert(1, 't');
}

#[test]
fn test_slicing() {
    let bump: Bump = Bump::new();
    let s = String::from_str_in("foobar", &bump);
    assert_eq!("foobar", &s[..]);
    assert_eq!("foo", &s[..3]);
    assert_eq!("bar", &s[3..]);
    assert_eq!("oob", &s[1..4]);
}

#[test]
fn test_simple_types() {
    let bump: Bump = Bump::new();
    assert_eq!(bump_format!(in &bump, "{}", 1), "1");
    assert_eq!(bump_format!(in &bump, "{}", -1), "-1");
    assert_eq!(bump_format!(in &bump, "{}", 200), "200");
    assert_eq!(bump_format!(in &bump, "{}", 2), "2");
    assert_eq!(bump_format!(in &bump, "{}", true), "true");
    assert_eq!(bump_format!(in &bump, "{}", false), "false");
    assert_eq!(bump_format!(in &bump, "{}", String::from_str_in("hi", &bump)), "hi");
}

#[test]
fn test_vectors() {
    let bump: Bump = Bump::new();
    let x: Vec<i32, _> = bump_vec![in &bump];
    assert_eq!(bump_format!(in &bump, "{x:?}"), "[]");
    assert_eq!(bump_format!(in &bump, "{:?}", bump_vec![in &bump; 1]), "[1]");
    assert_eq!(bump_format!(in &bump, "{:?}", bump_vec![in &bump; 1, 2, 3]), "[1, 2, 3]");
    assert!(
        bump_format!(in &bump, "{:?}", bump_vec![in &bump; bump_vec![in &bump], bump_vec![in &bump; 1], bump_vec![in &bump; 1, 1]])
            == "[[], [1], [1, 1]]"
    );
}

#[cfg(any())] // TODO: implement
#[test]
fn test_from_iterator() {
    let bump: Bump = Bump::new();
    let s = String::from_str_in("ศไทย中华Việt Nam", &bump);
    let t = "ศไทย中华";
    let u = "Việt Nam";

    let a: String = s.chars().collect();
    assert_eq!(s, a);

    let mut b = String::from_str_in(t, &bump);
    b.extend(u.chars());
    assert_eq!(s, b);

    let c: String = [t, u].into_iter().collect();
    assert_eq!(s, c);

    let mut d = String::from_str_in(t, &bump);
    d.extend(vec![u]);
    assert_eq!(s, d);
}

#[test]
fn test_drain() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("αβγ", &bump);
    assert_eq!(s.drain(2..4).collect::<StdString>(), "β");
    assert_eq!(s, "αγ");

    let mut t = String::from_str_in("abcd", &bump);
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
    let mut s = String::from_str_in("abc", &bump);
    s.drain((Excluded(usize::MAX), Included(0)));
}

#[test]
#[should_panic]
fn test_drain_end_overflow() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("abc", &bump);
    s.drain((Included(0), Included(usize::MAX)));
}

#[test]
fn test_replace_range() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("Hello, world!", &bump);
    s.replace_range(7..12, "世界");
    assert_eq!(s, "Hello, 世界!");
}

#[test]
#[should_panic]
fn test_replace_range_char_boundary() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("Hello, 世界!", &bump);
    s.replace_range(..8, "");
}

#[test]
fn test_replace_range_inclusive_range() {
    let bump: Bump = Bump::new();
    let mut v = String::from_str_in("12345", &bump);
    v.replace_range(2..=3, "789");
    assert_eq!(v, "127895");
    v.replace_range(1..=2, "A");
    assert_eq!(v, "1A895");
}

#[test]
#[should_panic]
fn test_replace_range_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.replace_range(5..6, "789");
}

#[test]
#[should_panic]
fn test_replace_range_inclusive_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.replace_range(5..=5, "789");
}

#[test]
#[should_panic]
fn test_replace_range_start_overflow() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("123", &bump);
    s.replace_range((Excluded(usize::MAX), Included(0)), "");
}

#[test]
#[should_panic]
fn test_replace_range_end_overflow() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("456", &bump);
    s.replace_range((Included(0), Included(usize::MAX)), "");
}

#[test]
fn test_replace_range_empty() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.replace_range(1..2, "");
    assert_eq!(s, "1345");
}

#[test]
fn test_replace_range_unbounded() {
    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("12345", &bump);
    s.replace_range(.., "");
    assert_eq!(s, "");
}

#[test]
fn test_replace_range_evil_start_bound() {
    struct EvilRange(Cell<bool>);

    impl RangeBounds<usize> for EvilRange {
        fn start_bound(&self) -> Bound<&usize> {
            Bound::Included(if self.0.get() {
                &1
            } else {
                self.0.set(true);
                &0
            })
        }
        fn end_bound(&self) -> Bound<&usize> {
            Bound::Unbounded
        }
    }

    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("🦀", &bump);
    s.replace_range(EvilRange(Cell::new(false)), "");
    assert_eq!(Ok(""), str::from_utf8(s.as_bytes()));
}

#[test]
fn test_replace_range_evil_end_bound() {
    struct EvilRange(Cell<bool>);

    impl RangeBounds<usize> for EvilRange {
        fn start_bound(&self) -> Bound<&usize> {
            Bound::Included(&0)
        }
        fn end_bound(&self) -> Bound<&usize> {
            Bound::Excluded(if self.0.get() {
                &3
            } else {
                self.0.set(true);
                &4
            })
        }
    }

    let bump: Bump = Bump::new();
    let mut s = String::from_str_in("🦀", &bump);
    s.replace_range(EvilRange(Cell::new(false)), "");
    assert_eq!(Ok(""), str::from_utf8(s.as_bytes()));
}

#[test]
fn test_extend_ref() {
    let bump: Bump = Bump::new();
    let mut a = String::from_str_in("foo", &bump);
    a.extend(&['b', 'a', 'r']);

    assert_eq!(&a, "foobar");
}

#[test]
fn test_into_boxed_str() {
    let bump: Bump = Bump::new();
    let xs = String::from_str_in("hello my name is bob", &bump);
    let ys = xs.into_boxed_str();
    assert_eq!(&*ys, "hello my name is bob");
}

#[test]
fn test_reserve_exact() {
    let bump: Bump = Bump::new();
    let mut s = String::new_in(&bump);
    assert_eq!(s.capacity(), 0);

    s.reserve_exact(2);
    assert!(s.capacity() >= 2);

    for _i in 0..16 {
        s.push('0');
    }

    assert!(s.capacity() >= 16);
    s.reserve_exact(16);
    assert!(s.capacity() >= 32);

    s.push('0');

    s.reserve_exact(16);
    assert!(s.capacity() >= 33)
}

#[test]
#[cfg_attr(miri, ignore)] // Miri does not support signalling OOM
fn test_try_with_capacity() {
    let bump: Bump = Bump::new();

    let string = String::try_with_capacity_in(1000, &bump).unwrap();
    assert_eq!(0, string.len());
    assert!(string.capacity() >= 1000 && string.capacity() <= isize::MAX as usize);

    assert!(String::try_with_capacity_in(usize::MAX, &bump).is_err());
}

#[cfg(any())] // we don't have try reserve error variants
fn test_try_reserve() {}

#[cfg(any())] // we don't have try reserve error variants
fn test_try_reserve_exact() {}

#[test]
fn test_from_char() {
    let bump: Bump = Bump::new();
    let mut s = String::new_in(&bump);
    s.push('a');
    assert_eq!(s, "a");
    let mut s = String::new_in(&bump);
    s.push('x');
    assert_eq!(s, "x");
}

#[test]
fn test_str_concat() {
    let bump: Bump = Bump::new();
    let a = String::from_str_in("hello", &bump);
    let b = String::from_str_in("world", &bump);
    let s = bump_format!(in &bump, "{a}{b}");
    assert_eq!(s.as_bytes()[9], 'd' as u8);
}
