//! Runtime abstraction for Scrapio
//!
//! Currently uses tokio as the default runtime.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub mod tokio_runtime;
pub use tokio_runtime::TokioRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeType {
    #[default]
    Tokio,
}

/// Sleep wrapper - uses Pin<Box<>> internally to handle !Unpin types
pub struct Sleep {
    inner: Pin<Box<tokio::time::Sleep>>,
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

/// JoinHandle wrapper - unwraps JoinError automatically
pub struct JoinHandle<T> {
    inner: tokio::task::JoinHandle<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.inner).poll(cx) {
            Poll::Ready(Ok(res)) => Poll::Ready(res),
            Poll::Ready(Err(e)) => panic!("Task panicked: {}", e),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait Runtime: Send + Sync + fmt::Debug {
    fn runtime_type(&self) -> RuntimeType;
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()>;
    fn block_on<F: Future>(&self, future: F) -> F::Output;
    fn spawn_blocking<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static;
    fn sleep(&self, duration: Duration) -> Sleep;
}
