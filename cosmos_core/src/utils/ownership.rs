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

impl<T> AsRef<T> for MaybeOwned<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Owned(owned) => owned.as_ref(),
            Self::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<T> Deref for MaybeOwned<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> From<T> for MaybeOwned<'_, T> {
    fn from(value: T) -> Self {
        Self::Owned(Box::new(value))
    }
}

impl<'a, T> From<&'a T> for MaybeOwned<'a, T> {
    fn from(value: &'a T) -> MaybeOwned<'a, T> {
        Self::Borrowed(value)
    }
}
