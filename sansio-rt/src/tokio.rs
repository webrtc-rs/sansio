//! Tokio-based local executor implementation
//!
//! This module provides a local executor implementation using tokio's `LocalSet`.

use core_affinity::{CoreId, set_for_current};
use scoped_tls::scoped_thread_local;
use std::{
    future::Future,
    io::Result,
    thread::{self, JoinHandle},
};
use tokio::task::{JoinHandle as TokioJoinHandle, LocalSet};

scoped_thread_local!(pub(super) static LOCAL_SET: LocalSet);

/// A factory that can be used to configure and create a tokio [`LocalSet`].
#[derive(Debug, Default)]
pub struct LocalExecutorBuilder {
    core_id: Option<CoreId>,
    name: String,
}

impl LocalExecutorBuilder {
    /// Creates a new LocalExecutorBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Names the thread-to-be. Currently, the name is used for identification only in panic messages.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Pins the thread to the specified CPU core
    pub fn core_id(mut self, core_id: CoreId) -> Self {
        self.core_id = Some(core_id);
        self
    }

    /// Runs the local executor on the current thread until the given future completes.
    pub fn run<T>(mut self, f: impl Future<Output = T>) -> T {
        if let Some(core_id) = self.core_id.take() {
            set_for_current(core_id);
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        let local_set = LocalSet::new();
        LOCAL_SET.set(&local_set, || rt.block_on(local_set.run_until(f)))
    }

    /// Spawns a thread to run the local executor until the given future completes.
    pub fn spawn<G, F, T>(mut self, fut_gen: G) -> Result<JoinHandle<T>>
    where
        G: FnOnce() -> F + Send + 'static,
        F: Future<Output = T> + 'static,
        T: Send + 'static,
    {
        let mut core_id = self.core_id.take();

        thread::Builder::new().name(self.name).spawn(move || {
            if let Some(core_id) = core_id.take() {
                set_for_current(core_id);
            }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");

            let local_set = LocalSet::new();
            LOCAL_SET.set(&local_set, || rt.block_on(local_set.run_until(fut_gen())))
        })
    }
}

/// Spawns a task onto the current single-threaded executor.
///
/// If called from a tokio [`LocalSet`], the task is spawned on it.
/// Otherwise, this method panics.
pub fn spawn_local<T: 'static>(future: impl Future<Output = T> + 'static) -> TokioJoinHandle<T> {
    if LOCAL_SET.is_set() {
        LOCAL_SET.with(|local_set| local_set.spawn_local(future))
    } else {
        panic!("`spawn_local()` must be called from a tokio `LocalSet`")
    }
}

/// Yield to allow other tasks to run.
///
/// **Note**: This is an async function in the tokio implementation, unlike smol's
/// synchronous version. This is because tokio only supports async task yielding.
///
/// This function yields execution to allow other tasks in the same LocalSet to run.
/// Call it as `yield_local().await` in async contexts.
///
/// # Example
/// ```rust,ignore
/// use sansio::{LocalExecutorBuilder, spawn_local, yield_local};
///
/// LocalExecutorBuilder::default().run(async {
///     spawn_local(async {
///         println!("Task 1 starting");
///         yield_local().await;  // Let other tasks run
///         println!("Task 1 resuming");
///     });
///
///     spawn_local(async {
///         println!("Task 2 running");
///     });
/// });
/// ```
pub async fn yield_local() {
    tokio::task::yield_now().await
}
