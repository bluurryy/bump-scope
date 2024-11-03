use core::fmt;

/// A possible error value when converting a string from a UTF-16 byte slice.
///
/// This type is the error type for the `from_utf16_in` method on
/// <code>([Mut])[BumpString]</code>.
///
/// # Examples
///
/// ```
/// # use bump_scope::{ Bump, BumpString };
/// # let bump: Bump = Bump::new();
///
/// // 𝄞mu<invalid>ic
/// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
///           0xD800, 0x0069, 0x0063];
///
/// assert!(BumpString::from_utf16_in(v, &bump).is_err());
/// ```
///
/// [Mut]: crate::MutBumpString::from_utf16_in
/// [BumpString]: crate::BumpString::from_utf16_in
#[derive(Debug)]
pub struct FromUtf16Error(pub(crate) ());

impl fmt::Display for FromUtf16Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("invalid utf-16: lone surrogate found", f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FromUtf16Error {}
