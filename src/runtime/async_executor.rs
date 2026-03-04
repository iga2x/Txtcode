use crate::runtime::Value;
use crate::runtime::errors::RuntimeError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

/// Future type for Txtcode async operations
pub type TxtcodeFuture = Pin<Box<dyn Future<Output = Result<Value, RuntimeError>> + Send>>;

/// Async executor for running async Txtcode code
pub struct AsyncExecutor {
    runtime: Arc<Runtime>,
}

impl AsyncExecutor {
    /// Create a new async executor with a Tokio runtime
    pub fn new() -> Result<Self, RuntimeError> {
        let runtime = Runtime::new()
            .map_err(|e| RuntimeError::new(format!("Failed to create async runtime: {}", e)))?;
        
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }

    /// Spawn a future and return a handle that can be awaited
    pub fn spawn<F>(&self, future: F) -> TxtcodeFuture
    where
        F: Future<Output = Result<Value, RuntimeError>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let runtime = self.runtime.clone();
        
        runtime.spawn(async move {
            let result = future.await;
            let _ = tx.send(result);
        });
        
        Box::pin(async move {
            rx.await
                .map_err(|e| RuntimeError::new(format!("Future cancelled: {}", e)))?
        })
    }

    /// Run a future to completion (blocking)
    pub fn block_on<F>(&self, future: F) -> Result<Value, RuntimeError>
    where
        F: Future<Output = Result<Value, RuntimeError>> + Send,
    {
        self.runtime.block_on(future)
    }

    /// Get a reference to the underlying Tokio runtime
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}

impl Default for AsyncExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create async executor")
    }
}

/// Helper to convert a value to a future
pub fn value_to_future(value: Value) -> TxtcodeFuture {
    Box::pin(async move { Ok(value) })
}

/// Helper to create a future from a result
pub fn result_to_future(result: Result<Value, RuntimeError>) -> TxtcodeFuture {
    Box::pin(async move { result })
}

