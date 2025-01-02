//! Contains ownership utilities

use std::ops::Deref;

/// A [`Cow`] without the ability to clone the borrowed version.
///
/// Represents a piece of data (T) that is either owned or borrowed.
pub enum MaybeOwned<'a, T> {
    /// The data is owned by this
    Owned(Box<T>),
    /// The data is borrowed by this
    Borrowed(&'a T),
}

impl<'a, T> AsRef<T> for MaybeOwned<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Owned(owned) => owned.as_ref(),
            Self::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, T> Deref for MaybeOwned<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}