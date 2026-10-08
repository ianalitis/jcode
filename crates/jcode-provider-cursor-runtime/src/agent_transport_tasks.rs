//! The two children owned by one Cursor turn and their bounded shutdown.

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{JoinError, JoinSet};
use tokio::time::{Instant, interval_at};

use super::{CONNECT_TIMEOUT, HEARTBEAT_INTERVAL, heartbeat_frame};

pub(super) enum Completion {
    Driver(Result<()>),
    Sender(Result<()>),
}

impl Completion {
    fn result(self) -> Result<()> {
        match self {
            Self::Driver(result) | Self::Sender(result) => result,
        }
    }
}

pub(super) fn premature(completion: Result<Completion, JoinError>) -> anyhow::Error {
    match completion {
        Ok(Completion::Driver(Ok(()))) => {
            anyhow!("Cursor HTTP/2 driver ended before terminal frame")
        }
        Ok(Completion::Sender(Ok(()))) => anyhow!("Cursor sender ended before terminal frame"),
        Ok(completion) => match completion.result() {
            Err(error) => error,
            Ok(()) => unreachable!("successful completion handled above"),
        },
        Err(error) if error.is_cancelled() => anyhow!("Cursor transport task cancelled"),
        Err(_) => anyhow!("Cursor transport task panicked"),
    }
}

pub(super) async fn send(
    mut stream: h2::SendStream<Bytes>,
    frames: Vec<Vec<u8>>,
    mut outbound: mpsc::Receiver<Vec<u8>>,
    mut stop: oneshot::Receiver<()>,
) -> Result<()> {
    'sending: {
        for (idx, frame) in frames.into_iter().enumerate() {
            stream
                .send_data(Bytes::from(frame), false)
                .context("Cursor initial request send failed")?;
            while let Ok(frame) = outbound.try_recv() {
                stream
                    .send_data(Bytes::from(frame), false)
                    .context("Cursor outbound reply send failed")?;
            }
            let pace = match idx {
                0 => std::time::Duration::from_millis(1500),
                1 => std::time::Duration::from_millis(800),
                _ => std::time::Duration::from_millis(400),
            };
            tokio::select! {
                _ = &mut stop => break 'sending,
                _ = tokio::time::sleep(pace) => {}
                Some(frame) = outbound.recv() => {
                    stream.send_data(Bytes::from(frame), false)
                        .context("Cursor paced reply send failed")?;
                }
            }
        }
        let mut ticker = interval_at(Instant::now() + HEARTBEAT_INTERVAL, HEARTBEAT_INTERVAL);
        loop {
            tokio::select! {
                _ = &mut stop => break,
                Some(frame) = outbound.recv() => {
                    stream.send_data(Bytes::from(frame), false)
                        .context("Cursor outbound reply send failed")?;
                }
                _ = ticker.tick() => {
                    stream.send_data(Bytes::from(heartbeat_frame()), false)
                        .context("Cursor heartbeat send failed")?;
                }
            }
        }
    }
    stream
        .send_data(Bytes::new(), true)
        .context("Cursor request FIN send failed")
}

/// Abort is a request. Joining every child below observes actual destruction.
pub(super) async fn abort_and_drain(tasks: &mut JoinSet<Completion>) -> Result<()> {
    tasks.abort_all();
    let mut failure = None;
    while let Some(completion) = tasks.join_next().await {
        let result = match completion {
            Ok(completion) => completion.result(),
            Err(error) if error.is_cancelled() => Ok(()),
            Err(_) => Err(anyhow!("Cursor transport task panicked")),
        };
        if let Err(error) = result {
            failure.get_or_insert(error);
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(super) async fn finish(tasks: &mut JoinSet<Completion>) -> Result<()> {
    let graceful = tokio::time::timeout(CONNECT_TIMEOUT, async {
        let mut failure = None;
        while let Some(completion) = tasks.join_next().await {
            let sender = matches!(completion, Ok(Completion::Sender(_)));
            let result = match completion {
                Ok(completion) => completion.result(),
                Err(error) if error.is_cancelled() => {
                    Err(anyhow!("Cursor transport task cancelled during shutdown"))
                }
                Err(_) => Err(anyhow!("Cursor transport task panicked")),
            };
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
            if sender {
                break;
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    })
    .await;
    let drained = abort_and_drain(tasks).await;
    match graceful {
        Ok(result) => result.and(drained),
        Err(_) => Err(anyhow!("Timed out stopping Cursor transport tasks")),
    }
}
