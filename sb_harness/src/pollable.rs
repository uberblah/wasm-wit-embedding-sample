use futures::task::noop_waker;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum PollableResult<O> {
    Ready(O),
    Pending,
    Stale,
}

/// A fully-owned wrapper around any Future that can be polled manually.
pub struct Pollable<O>
{
    inner: Pin<Box<dyn Future<Output = O>>>,
    stale: bool,
}

impl<O> Pollable<O>
{
    /// Create a new Pollable future, taking ownership of the future.
    pub fn new(fut: Box<dyn Future<Output=O>>) -> Self {
        Self {
            inner: fut.into(),
            stale: false,
        }
    }

    /// Poll the inner future once, returning the Poll result.
    /// Safe to call repeatedly.
    pub fn poll(&mut self) -> PollableResult<O> {
        if self.stale {
            return PollableResult::Stale;
        }
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let output = self.inner.as_mut().poll(&mut cx);
        if let Poll::Ready(value) = output {
            self.stale = true; // Mark as stale after the first poll
            PollableResult::Ready(value)
        } else {
            PollableResult::Pending
        }
    }
}
