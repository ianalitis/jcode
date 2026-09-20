/// Discovery and buffering controls for [`JcodeClient::global_events`].
#[derive(Debug, Clone, Copy)]
pub struct GlobalEventsOptions {
    /// How often persisted sessions are rescanned. Zero performs one scan.
    pub discovery_interval: Duration,
    /// Maximum events waiting for the consumer before the stream fails loudly.
    pub max_buffered_events: usize,
}

impl Default for GlobalEventsOptions {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(1),
            max_buffered_events: 10_000,
        }
    }
}

struct GlobalEventControl {
    stopped: AtomicBool,
    terminal_error: Mutex<Option<Error>>,
    children: Mutex<HashMap<String, JcodeClient>>,
    tx: SyncSender<ApiEvent>,
    max_buffered_events: usize,
    wake_lock: Mutex<()>,
    wake: Condvar,
}

/// Events fanned in from every persisted and newly-created session.
///
/// Delivery begins when each per-session child attaches. Ordering is preserved
/// within a session; no total order across sessions is promised. Dropping this
/// stream cancels discovery and closes all child connections.
pub struct GlobalEventStream {
    rx: Receiver<ApiEvent>,
    control: Arc<GlobalEventControl>,
    discovery: Option<std::thread::JoinHandle<()>>,
}

impl GlobalEventStream {
    /// Block for the next event, returning the terminal stream error once.
    pub fn next(&self) -> Result<Option<ApiEvent>> {
        loop {
            if let Some(error) = take_global_error(&self.control) {
                return Err(error);
            }
            if self.control.stopped.load(Ordering::Acquire) {
                return Ok(None);
            }
            match self.rx.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => {
                    if let Some(error) = take_global_error(&self.control) {
                        return Err(error);
                    }
                    if self.control.stopped.load(Ordering::Acquire) {
                        return Ok(None);
                    }
                    return Ok(Some(event));
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.control.stopped.load(Ordering::Acquire) {
                        if let Some(error) = take_global_error(&self.control) {
                            return Err(error);
                        }
                        return Ok(None);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(None),
            }
        }
    }

    /// Wait up to `timeout` for an event. `Ok(None)` means timeout or shutdown.
    pub fn next_timeout(&self, timeout: Duration) -> Result<Option<ApiEvent>> {
        if let Some(error) = take_global_error(&self.control) {
            return Err(error);
        }
        if self.control.stopped.load(Ordering::Acquire) {
            return Ok(None);
        }
        match self.rx.recv_timeout(timeout) {
            Ok(event) => {
                if let Some(error) = take_global_error(&self.control) {
                    Err(error)
                } else if self.control.stopped.load(Ordering::Acquire) {
                    Ok(None)
                } else {
                    Ok(Some(event))
                }
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(error) = take_global_error(&self.control) {
                    Err(error)
                } else {
                    Ok(None)
                }
            }
        }
    }
}

impl Iterator for GlobalEventStream {
    type Item = Result<ApiEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        GlobalEventStream::next(self).transpose()
    }
}

impl Drop for GlobalEventStream {
    fn drop(&mut self) {
        stop_global_stream(&self.control, None);
        if let Some(discovery) = self.discovery.take() {
            let _ = discovery.join();
        }
    }
}

fn take_global_error(control: &GlobalEventControl) -> Option<Error> {
    control.terminal_error.lock().ok()?.take()
}

fn stop_global_stream(control: &GlobalEventControl, error: Option<Error>) {
    if let Some(error) = error
        && let Ok(mut terminal) = control.terminal_error.lock()
        && terminal.is_none()
    {
        *terminal = Some(error);
    }
    control.stopped.store(true, Ordering::Release);
    control.wake.notify_all();
    let children = control
        .children
        .lock()
        .ok()
        .map(|mut children| std::mem::take(&mut *children));
    drop(children);
}
