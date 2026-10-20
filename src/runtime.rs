//! Session-owned lifecycle and playback state; independent of UI notification delivery.
use anyhow::{bail, Result};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::Notify;

#[derive(Default)]
pub struct SessionControl {
    stopped: AtomicBool,
    failure: Mutex<Option<String>>,
    wake: Notify,
    pub text_jobs: AtomicUsize,
    first_audio_us: AtomicU64,
}
impl SessionControl {
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.wake.notify_waiters();
    }
    pub fn fail(&self, message: String) {
        if let Ok(mut failure) = self.failure.lock() {
            if !self.is_stopped() {
                *failure = Some(message);
                self.stop();
            }
        } else {
            self.stop();
        }
    }
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().ok()?.clone()
    }
    pub async fn cancelled(&self) {
        // Register before checking the flag, so Stop cannot be lost between them.
        let notified = self.wake.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if !self.is_stopped() {
            notified.await;
        }
    }
    pub fn check(&self) -> Result<()> {
        anyhow::ensure!(!self.is_stopped(), "Stopped");
        Ok(())
    }
    pub fn record_first_audio(&self, elapsed: Duration) {
        self.first_audio_us.store(
            elapsed.as_micros().max(1).min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
    }
    pub fn first_audio_ms(&self) -> Option<f64> {
        match self.first_audio_us.load(Ordering::Relaxed) {
            0 => None,
            v => Some(v as f64 / 1000.0),
        }
    }
}

pub struct PlaybackBuffer {
    queue: Mutex<VecDeque<f32>>,
    rate: u32,
    capacity: usize,
    stall_timeout: Duration,
}
impl PlaybackBuffer {
    pub fn new(rate: u32) -> Self {
        Self::with_stall_timeout(rate, Duration::from_secs(2))
    }
    fn with_stall_timeout(rate: u32, stall_timeout: Duration) -> Self {
        assert!(rate > 0);
        let capacity = (rate as usize * 4 / 5).max(1);
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            rate,
            capacity,
            stall_timeout,
        }
    }
    pub fn seconds(&self) -> f32 {
        self.queue
            .lock()
            .map(|q| q.len() as f32 / self.rate as f32)
            .unwrap_or(0.0)
    }
    pub fn clear(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }
    pub fn push(&self, samples: &[f32], control: &SessionControl) -> Result<()> {
        let mut cursor = 0;
        let mut last_progress = Instant::now();
        let mut previous_len = usize::MAX;
        while cursor < samples.len() {
            control.check()?;
            let mut q = self
                .queue
                .lock()
                .map_err(|_| anyhow::anyhow!("Playback queue unavailable"))?;
            if q.len() < previous_len {
                last_progress = Instant::now();
            }
            let count = (self.capacity - q.len()).min(samples.len() - cursor);
            if count > 0 {
                q.extend(&samples[cursor..cursor + count]);
                cursor += count;
                last_progress = Instant::now();
            }
            previous_len = q.len();
            drop(q);
            if cursor == samples.len() {
                return control.check();
            }
            if last_progress.elapsed() >= self.stall_timeout {
                bail!("Output device stopped consuming audio; restart the session");
            }
            thread::sleep(Duration::from_millis(5));
        }
        control.check()
    }
    /// CPAL callback uses a try-lock and always emits silence after cancellation.
    /// The caller provides channel conversion; no heap allocation in this method.
    pub fn render<T>(
        &self,
        data: &mut [T],
        channels: usize,
        selected: usize,
        control: &SessionControl,
        convert: impl Fn(f32) -> T,
    ) {
        let mut queue = if control.is_stopped() {
            None
        } else {
            self.queue.try_lock().ok()
        };
        for frame in data.chunks_exact_mut(channels) {
            let value = if control.is_stopped() {
                0.0
            } else {
                queue.as_mut().and_then(|q| q.pop_front()).unwrap_or(0.0)
            };
            for (index, sample) in frame.iter_mut().enumerate() {
                *sample = convert(if index == selected { value } else { 0.0 });
            }
        }
    }
}

/// Report panics durably; a UI notification is not needed to silence output.
pub fn guarded(control: &SessionControl, work: impl FnOnce() -> Result<()>) {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        Ok(Err(error)) => control.fail(format!("{error:#}")),
        Err(_) => {
            control.fail("Translation worker failed unexpectedly; restart the session".into())
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn stop_silences_buffer_without_ui_processing() {
        let control = SessionControl::default();
        let playback = PlaybackBuffer::new(16000);
        playback.push(&[0.5; 20], &control).unwrap();
        control.fail("network failed".into());
        let mut output = [1.0; 20];
        playback.render(&mut output, 2, 1, &control, |v| v);
        assert!(output.iter().all(|&s| s == 0.0));
        assert_eq!(control.failure().as_deref(), Some("network failed"));
        control.fail("secondary error".into());
        assert_eq!(control.failure().as_deref(), Some("network failed"));
    }
    #[test]
    fn restart_owns_a_separate_playback_buffer_and_token() {
        let old = SessionControl::default();
        let old_buffer = PlaybackBuffer::new(16000);
        old_buffer.push(&[0.9; 8], &old).unwrap();
        old.stop();
        let fresh = SessionControl::default();
        let fresh_buffer = PlaybackBuffer::new(16000);
        assert!(old_buffer.push(&[1.0], &old).is_err());
        fresh_buffer.push(&[0.2; 4], &fresh).unwrap();
        let mut output = [0.0; 4];
        fresh_buffer.render(&mut output, 1, 0, &fresh, |v| v);
        assert_eq!(output, [0.2; 4]);
    }
    #[test]
    fn stalled_output_is_bounded_and_reports_an_error() {
        let control = SessionControl::default();
        let playback = PlaybackBuffer::with_stall_timeout(100, Duration::from_millis(30));
        assert!(playback
            .push(&[0.1; 81], &control)
            .unwrap_err()
            .to_string()
            .contains("stopped consuming"));
        assert!(playback.seconds() <= 0.8);
    }
    #[test]
    fn producer_backpressure_releases_on_stop() {
        let control = Arc::new(SessionControl::default());
        let playback = Arc::new(PlaybackBuffer::new(100));
        playback.push(&[0.1; 80], &control).unwrap();
        let token = control.clone();
        let buffer = playback.clone();
        let worker = thread::spawn(move || buffer.push(&[0.3; 10], &token));
        thread::sleep(Duration::from_millis(15));
        control.stop();
        assert!(worker.join().unwrap().is_err());
    }
    #[test]
    fn cancellation_before_poll_is_not_lost() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let control = SessionControl::default();
        control.stop();
        rt.block_on(async {
            tokio::time::timeout(Duration::from_millis(50), control.cancelled())
                .await
                .unwrap();
        });
    }
}
