use core::{error::Error, fmt, ops::Deref, str::Utf8Error};

/// A possible error value when converting a string from a UTF-8 byte vector.
///
/// This type is the error type for [`BumpString::from_utf8`](crate::BumpString::from_utf8), [`MutBumpString::from_utf8`](crate::MutBumpString::from_utf8) and [`BumpBox<str>::from_utf8`](crate::BumpBox::from_utf8). It
/// is designed in such a way to carefully avoid reallocations: the
/// [`into_bytes`](FromUtf8Error::into_bytes) method will give back the byte vector that was used in the
/// conversion attempt.
///
/// The [`Utf8Error`] type provided by [`std::str`] represents an error that may
/// occur when converting a slice of [`u8`]s to a [`&str`]. In this sense, it's
/// an analogue to `FromUtf8Error`, and you can get one from a `FromUtf8Error`
/// through the [`utf8_error`] method.
///
/// [`Utf8Error`]: core::str::Utf8Error
/// [`std::str`]: core::str "std::str"
/// [`&str`]: prim@str "&str"
/// [`utf8_error`]: FromUtf8Error::utf8_error
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "panic-on-alloc", derive(Clone))]
pub struct FromUtf8Error<Bytes> {
    bytes: Bytes,
    error: Utf8Error,
}

impl<Bytes> FromUtf8Error<Bytes> {
    #[inline(always)]
    pub(crate) const fn new(error: Utf8Error, bytes: Bytes) -> Self {
        Self { bytes, error }
    }

    /// Returns a slice of [`u8`]s bytes that were attempted to convert to a `String`.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8]
    where
        Bytes: Deref<Target = [u8]>,
    {
        &self.bytes
    }

    /// Returns the bytes that were attempted to convert to a `String`.
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline(always)]
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    /// Fetch a `Utf8Error` to get more details about the conversion failure.
    ///
    /// The [`Utf8Error`] type provided by [`std::str`] represents an error that may
    /// occur when converting a slice of [`u8`]s to a [`&str`]. In this sense, it's
    /// an analogue to `FromUtf8Error`. See its documentation for more details
    /// on using it.
    ///
    /// [`std::str`]: core::str "std::str"
    /// [`&str`]: prim@str "&str"
    #[inline(always)]
    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl<Bytes> fmt::Display for FromUtf8Error<Bytes> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl<Bytes: fmt::Debug> Error for FromUtf8Error<Bytes> {}
