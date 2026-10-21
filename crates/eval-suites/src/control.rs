use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

/// Cooperative cancellation for a benchmark run.
#[derive(Default)]
pub struct RunControl {
    cancelled: AtomicBool,
    notify: Notify,
}

impl RunControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn cancel_wakes_all_waiters() {
        let control = std::sync::Arc::new(RunControl::new());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = std::sync::Arc::clone(&control);
            handles.push(tokio::spawn(async move {
                c.cancelled().await;
                true
            }));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        control.cancel();
        for handle in handles {
            let woke = tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .expect("waiter should wake")
                .unwrap();
            assert!(woke);
        }
        assert!(control.is_cancelled());
    }

    #[tokio::test]
    async fn already_cancelled_returns_immediately() {
        let control = RunControl::new();
        control.cancel();
        tokio::time::timeout(Duration::from_millis(50), control.cancelled())
            .await
            .expect("already-cancelled future should complete");
    }
}
