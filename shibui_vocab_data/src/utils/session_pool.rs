use std::sync::{Mutex, MutexGuard};

use tokio::sync::{Semaphore, SemaphorePermit};

pub const DEFAULT_SESSION_COUNT: usize = 3;

pub fn session_count_from(value: Option<&str>) -> usize {
    value
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|count| *count >= 1)
        .unwrap_or(DEFAULT_SESSION_COUNT)
}

pub struct SessionPool {
    idle: Mutex<Vec<Option<String>>>,
    available: Semaphore,
}

impl SessionPool {
    pub fn new(size: usize) -> Self {
        Self {
            idle: Mutex::new(vec![None; size]),
            available: Semaphore::new(size),
        }
    }

    pub async fn acquire(&self) -> PooledSession<'_> {
        let permit = self
            .available
            .acquire()
            .await
            .expect("the pool never closes its semaphore");
        let session = self.lock_idle().pop().unwrap_or_default();
        PooledSession {
            pool: self,
            session,
            _permit: permit,
        }
    }

    fn lock_idle(&self) -> MutexGuard<'_, Vec<Option<String>>> {
        self.idle.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct PooledSession<'a> {
    pool: &'a SessionPool,
    pub session: Option<String>,
    _permit: SemaphorePermit<'a>,
}

impl Drop for PooledSession<'_> {
    fn drop(&mut self) {
        let session = self.session.take();
        self.pool.lock_idle().push(session);
    }
}
