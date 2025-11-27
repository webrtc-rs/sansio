//! # SansIO Runtime - Async Executor Abstraction
//!
//! `sansio-rt` provides a runtime abstraction layer that works with both smol and tokio,
//! allowing you to write runtime-agnostic async code for the sansio ecosystem.
//!
//! ## Features
//!
//! - **Runtime Flexibility**: Choose between smol or tokio at compile time
//! - **CPU Pinning**: Pin executor threads to specific CPU cores
//! - **Thread Naming**: Name executor threads for debugging
//! - **Consistent API**: Uniform API across both runtimes
//!
//! ## Quick Start
//!
//! ### With Default (smol) Runtime
//!
//! ```toml
//! [dependencies]
//! sansio-rt = "0.0.5"
//! ```
//!
//! ```rust,no_run
//! use sansio_rt::LocalExecutorBuilder;
//!
//! LocalExecutorBuilder::default()
//!     .run(async {
//!         println!("Running on smol!");
//!     });
//! ```
//!
//! ### With Tokio Runtime
//!
//! ```toml
//! [dependencies]
//! sansio-rt = { version = "0.0.5", default-features = false, features = ["runtime-tokio"] }
//! ```
//!
//! ```rust,no_run
//! use sansio_rt::LocalExecutorBuilder;
//!
//! LocalExecutorBuilder::default()
//!     .run(async {
//!         println!("Running on tokio!");
//!     });
//! ```
//!
//! ### With CPU Pinning
//!
//! ```rust,no_run
//! use sansio_rt::LocalExecutorBuilder;
//! use core_affinity::CoreId;
//!
//! LocalExecutorBuilder::new()
//!     .name("my-executor")
//!     .core_id(CoreId { id: 0 })
//!     .run(async {
//!         println!("Running on CPU core 0!");
//!     });
//! ```
//!
//! ### Spawning Local Tasks
//!
//! ```rust,no_run
//! use sansio_rt::{LocalExecutorBuilder, spawn_local};
//!
//! LocalExecutorBuilder::default().run(async {
//!     let task1 = spawn_local(async {
//!          println!("Task 1");
//!          42
//!     });
//!
//!     let task2 = spawn_local(async {
//!          println!("Task 2");
//!          100
//!     });
//!
//!     let result1 = task1.await.unwrap();
//!     let result2 = task2.await.unwrap();
//!
//!     println!("Results: {}, {}", result1, result2);
//!});
//! ```
//!
//! ## Feature Flags
//!
//! - **`runtime-smol`** (default): Use smol's LocalExecutor
//! - **`runtime-tokio`**: Use tokio's LocalSet
//!
//! **Note:** Only one runtime feature can be enabled at a time.
//!
//! ## API
//!
//! The main exports are:
//!
//! - [`LocalExecutorBuilder`]: Builder for configuring and creating executors
//! - [`spawn_local()`]: Spawn a task on the current executor, returns a [`Task`]
//! - [`yield_local()`]: Yield to other tasks (async in both runtimes)
//! - [`Task<T>`]: A handle to a spawned task, implements `Future<Output = Result<T, TaskError>>`
//! - [`TaskError`]: Error type returned when a task fails (panic or cancellation)
//!
//! ## Task Execution
//!
//! Tasks spawned with [`spawn_local()`] return a [`Task<T>`] handle that can be awaited:
//!
//! ```rust,no_run
//! use sansio_rt::{LocalExecutorBuilder, spawn_local};
//!
//! LocalExecutorBuilder::default().run(async {
//!     let task = spawn_local(async { 42 });
//!
//!     // Task implements Future<Output = Result<T, TaskError>>
//!     match task.await {
//!         Ok(result) => println!("Task completed: {}", result),
//!         Err(e) => eprintln!("Task failed: {}", e),
//!     }
//! });
//! ```
//!
//! ### Detaching Tasks
//!
//! Tasks can be detached to run in the background without awaiting them:
//!
//! ```rust,no_run
//! use sansio_rt::{LocalExecutorBuilder, spawn_local};
//!
//! LocalExecutorBuilder::default().run(async {
//!     let task = spawn_local(async {
//!         println!("Running in background");
//!     });
//!
//!     // Detach - task continues running even though we don't await it
//!     task.detach();
//! });
//! ```
//!
//! ## Error Handling
//!
//! - **smol**: Tasks never fail - panics propagate directly to the awaiter
//! - **tokio**: Tasks can fail if they panic or are cancelled/aborted
//!
//! ## Documentation
//!
//! For detailed documentation, see the [LocalExecutor guide](doc/LocalExecutor.md) in the repository.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sansio-org/sansio/master/doc/sansio-white.png"
)]
#![warn(rust_2018_idioms)]
#![allow(dead_code)]
#![warn(missing_docs)]

// =============================================================================
// Smol-based implementation
// =============================================================================

#[cfg(feature = "runtime-smol")]
mod smol;

#[cfg(feature = "runtime-smol")]
pub use smol::*;

// =============================================================================
// Tokio-based implementation
// =============================================================================

#[cfg(feature = "runtime-tokio")]
mod tokio;

#[cfg(feature = "runtime-tokio")]
pub use tokio::*;

// =============================================================================
// Compile-time guards
// =============================================================================

// Compile error if neither or both features are enabled
#[cfg(not(any(feature = "runtime-smol", feature = "runtime-tokio")))]
compile_error!("Either 'runtime-smol' or 'runtime-tokio' feature must be enabled");

#[cfg(all(feature = "runtime-smol", feature = "runtime-tokio"))]
compile_error!(
    "Only one runtime feature can be enabled at a time: 'runtime-smol' or 'runtime-tokio'"
);
