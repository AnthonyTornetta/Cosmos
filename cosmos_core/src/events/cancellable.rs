//! Represents cancellable messages

use bevy::prelude::*;

pub trait CancellableMessage {
    fn is_cancelled(&self) -> bool;
    fn cancel(&mut self);
}

#[derive(Message)]
pub enum Cancellable<M: Send + Sync + 'static> {
    Active(M),
    Cancelled,
}

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
    pub fn new(m: M) -> Self {
        Self::Active(m)
    }

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
        !matches!(self, Self::Cancelled)
    }

    fn cancel(&mut self) {
        *self = Self::Cancelled
    }
}

pub trait CancellableMessageCmdImpl {
    fn add_cancellable_message<M: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl CancellableMessageCmdImpl for App {
    fn add_cancellable_message<M: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.add_message::<Cancellable<M>>();
        self
    }
}
