//! Not-terrible AsyncDrop primitives
//!
//! Defines the [`AsyncDrop`] trait and [`AsyncDropper`]/[`AsyncDropperTimeout`] structs,
//! which can be used to run code on the same executor thread on-[`Drop`], before dropping
//! the work itself.
//!
//! This works by using [`spawn_local`](tokio::task::spawn_local) to run the actual drop
//! work on the same thread, then [`block_in_place`](tokio::task::block_in_place) to hint
//! to tokio that this thread will be occupied for a bit. This may force tokio to move work
//! to other threads, so be careful of executor thrashing with small workloads.

use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

pub trait AsyncDrop: 'static {
    fn async_drop(&mut self) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct AsyncDropper<T: AsyncDrop>(Option<T>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncDropperTimeout<T: AsyncDrop>(Option<T>, Duration);

impl<T: AsyncDrop> AsyncDropper<T> {
    #[inline]
    pub const fn new(task: T) -> Self {
        AsyncDropper(Some(task))
    }
}

impl<T: AsyncDrop> AsyncDropperTimeout<T> {
    #[inline]
    pub const fn new(timeout: Duration, task: T) -> Self {
        AsyncDropperTimeout(Some(task), timeout)
    }
}

impl<T: AsyncDrop> Deref for AsyncDropper<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref().unwrap_unchecked() }
    }
}

impl<T: AsyncDrop> DerefMut for AsyncDropper<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut().unwrap_unchecked() }
    }
}

impl<T: AsyncDrop> Deref for AsyncDropperTimeout<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref().unwrap_unchecked() }
    }
}

impl<T: AsyncDrop> DerefMut for AsyncDropperTimeout<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut().unwrap_unchecked() }
    }
}

impl<T: AsyncDrop> Drop for AsyncDropper<T> {
    fn drop(&mut self) {
        if let Some(mut task) = self.0.take() {
            block_local(task.async_drop());
        }
    }
}

impl<T: AsyncDrop> Drop for AsyncDropperTimeout<T> {
    fn drop(&mut self) {
        if let Some(mut task) = self.0.take() {
            let timeout = self.1;

            block_local(async move {
                _ = tokio::time::timeout(timeout, task.async_drop()).await;
            });
        }
    }
}

fn block_local<F>(f: F)
where
    F: Future<Output = ()>,
{
    tokio::task::block_in_place(move || {
        tokio::runtime::Handle::current().block_on(f);
    });
}
