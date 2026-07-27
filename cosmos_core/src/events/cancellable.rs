//! Represents cancellable messages

use bevy::prelude::*;

/// A message that can be cancelled - see [`CancellableMessage<M>`]
pub trait CancellableMessage {
    /// Returns true if this is cancelled
    fn is_cancelled(&self) -> bool;
    /// Cancels this event
    fn cancel(&mut self);
}

#[derive(Message)]
/// Denotes a Message as being cancellable
pub enum Cancellable<M: Send + Sync + 'static> {
    /// This message has not been cancelled
    Active(M),
    /// This message has been cancelled, and can be ignored
    Cancelled,
}

/// Iterates over a cancellable event - basically the same as an `Option` iterator.
pub struct CancellableIter<'a, M: Send + Sync + 'a> {
    inner: Option<&'a M>,
}

impl<'a, M: Send + Sync + 'a> Iterator for CancellableIter<'a, M> {
    type Item = &'a M;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.take()
    }
}

impl<'a, M: Send + Sync + 'static> IntoIterator for &'a Cancellable<M> {
    type IntoIter = CancellableIter<'a, M>;
    type Item = &'a M;

    fn into_iter(self) -> Self::IntoIter {
        CancellableIter {
            inner: match self {
                Cancellable::Cancelled => None,
                Cancellable::Active(a) => Some(a),
            },
        }
    }
}

impl<M: Send + Sync + 'static> Cancellable<M> {
    /// Creates a new [`Self::Active`] event from this
    pub fn new(m: M) -> Self {
        Self::Active(m)
    }

    /// Some if not cancelled
    pub fn as_option(&self) -> Option<&M> {
        match self {
            Self::Cancelled => None,
            Self::Active(m) => Some(m),
        }
    }
}

impl<M: Send + Sync + 'static> From<M> for Cancellable<M> {
    fn from(value: M) -> Self {
        Self::Active(value)
    }
}

impl<M: Send + Sync + 'static> CancellableMessage for Cancellable<M> {
    fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    fn cancel(&mut self) {
        *self = Self::Cancelled
    }
}

/// Simple helper trait for [`App`] `add_cancellable_message`
pub trait CancellableMessageCmdImpl {
    /// Simple helper trait for [`App`] `add_cancellable_message`
    fn add_cancellable_message<M: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl CancellableMessageCmdImpl for App {
    fn add_cancellable_message<M: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.add_message::<Cancellable<M>>();
        self
    }
}
