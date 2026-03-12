//! Tokio runtime implementation

use std::fmt;
use std::future::Future;
use std::time::Duration;

use super::{JoinHandle, Runtime, RuntimeType, Sleep};

pub struct TokioRuntime {
    runtime: tokio::runtime::Runtime,
}

impl TokioRuntime {
    pub fn current() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        Self { runtime }
    }

    pub fn new(runtime: tokio::runtime::Runtime) -> Self {
        Self { runtime }
    }

    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }
}

impl Default for TokioRuntime {
    fn default() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        Self { runtime }
    }
}

impl fmt::Debug for TokioRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokioRuntime").finish()
    }
}

impl Runtime for TokioRuntime {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Tokio
    }

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
        JoinHandle {
            inner: self.runtime.spawn(future),
        }
    }

    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    fn spawn_blocking<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        JoinHandle {
            inner: self.runtime.spawn_blocking(f),
        }
    }

    fn sleep(&self, duration: Duration) -> Sleep {
        Sleep {
            inner: Box::pin(tokio::time::sleep(duration)),
        }
    }
}
