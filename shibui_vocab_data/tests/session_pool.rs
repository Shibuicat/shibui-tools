use std::time::Duration;

use shibui_vocab_data::utils::session_pool::{session_count_from, SessionPool, DEFAULT_SESSION_COUNT};
use tokio::time::timeout;

const SHORT: Duration = Duration::from_millis(100);

#[test]
fn session_count_defaults_to_three_when_unset() {
    assert_eq!(session_count_from(None), 3);
    assert_eq!(DEFAULT_SESSION_COUNT, 3);
}

#[test]
fn session_count_reads_a_valid_value() {
    assert_eq!(session_count_from(Some("5")), 5);
    assert_eq!(session_count_from(Some(" 1 ")), 1);
}

#[test]
fn session_count_falls_back_to_the_default_on_zero_or_garbage() {
    assert_eq!(session_count_from(Some("0")), DEFAULT_SESSION_COUNT);
    assert_eq!(session_count_from(Some("-2")), DEFAULT_SESSION_COUNT);
    assert_eq!(session_count_from(Some("many")), DEFAULT_SESSION_COUNT);
    assert_eq!(session_count_from(Some("")), DEFAULT_SESSION_COUNT);
}

#[tokio::test]
async fn pool_hands_out_as_many_sessions_as_its_size_at_once() {
    let pool = SessionPool::new(3);

    let first = pool.acquire().await;
    let second = pool.acquire().await;
    let third = pool.acquire().await;

    assert!(first.session.is_none() && second.session.is_none() && third.session.is_none());
}

#[tokio::test]
async fn acquiring_beyond_the_pool_size_waits_until_one_is_released() {
    let pool = SessionPool::new(2);
    let first = pool.acquire().await;
    let _second = pool.acquire().await;

    assert!(timeout(SHORT, pool.acquire()).await.is_err());

    drop(first);

    assert!(timeout(SHORT, pool.acquire()).await.is_ok());
}

#[tokio::test]
async fn released_slot_keeps_its_session_id_for_the_next_request() {
    let pool = SessionPool::new(1);
    {
        let mut pooled = pool.acquire().await;
        pooled.session = Some("session-a".to_string());
    }

    let again = pool.acquire().await;

    assert_eq!(again.session.as_deref(), Some("session-a"));
}

#[tokio::test]
async fn concurrent_holders_never_share_a_session() {
    let pool = std::sync::Arc::new(SessionPool::new(3));
    let held = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

    let tasks: Vec<_> = (0..12)
        .map(|n| {
            let pool = pool.clone();
            let held = held.clone();
            tokio::spawn(async move {
                let mut pooled = pool.acquire().await;
                let id = pooled.session.get_or_insert_with(|| format!("session-{n}")).clone();
                let inserted = held.lock().unwrap().insert(id.clone());
                tokio::time::sleep(Duration::from_millis(20)).await;
                held.lock().unwrap().remove(&id);
                inserted
            })
        })
        .collect();

    for task in tasks {
        assert!(task.await.unwrap(), "two requests held the same session at the same time");
    }
}
