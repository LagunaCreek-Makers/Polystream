//! App-level faders and post-fader block meters. No locks or allocation in callbacks.
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
pub struct AudioLevel {
    gain: AtomicU32,
    muted: AtomicBool,
    rms: AtomicU32,
    peak: AtomicU32,
    clipped: AtomicU64,
}
impl Default for AudioLevel {
    fn default() -> Self {
        Self {
            gain: AtomicU32::new(1f32.to_bits()),
            muted: AtomicBool::new(false),
            rms: AtomicU32::new(0),
            peak: AtomicU32::new(0),
            clipped: AtomicU64::new(0),
        }
    }
}
impl AudioLevel {
    pub fn set_input_db(&self, db: f32) {
        let db = if db.is_finite() {
            db.clamp(-24., 24.)
        } else {
            0.
        };
        self.gain
            .store(10f32.powf(db / 20.).to_bits(), Ordering::Relaxed);
    }
    pub fn set_volume(&self, volume: f32) {
        let volume = if volume.is_finite() {
            volume.clamp(0., 1.)
        } else {
            0.
        };
        self.gain.store(volume.to_bits(), Ordering::Relaxed);
    }
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }
    pub fn gain(&self) -> f32 {
        if self.muted.load(Ordering::Relaxed) {
            0.
        } else {
            f32::from_bits(self.gain.load(Ordering::Relaxed))
        }
    }
    pub fn publish(&self, meter: &SampleMeter) {
        self.rms.store(meter.rms().to_bits(), Ordering::Relaxed);
        self.peak.store(meter.peak.to_bits(), Ordering::Relaxed);
        self.clipped.fetch_add(meter.clipped, Ordering::Relaxed);
    }
    pub fn rms(&self) -> f32 {
        f32::from_bits(self.rms.load(Ordering::Relaxed))
    }
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak.load(Ordering::Relaxed))
    }
    pub fn clipped(&self) -> u64 {
        self.clipped.load(Ordering::Relaxed)
    }
    pub fn clear_meter(&self) {
        self.rms.store(0, Ordering::Relaxed);
        self.peak.store(0, Ordering::Relaxed);
    }
    pub fn reset_clipping(&self) {
        self.clipped.store(0, Ordering::Relaxed);
    }
}
#[derive(Default)]
pub struct SampleMeter {
    sum: f64,
    count: u64,
    peak: f32,
    clipped: u64,
}
impl SampleMeter {
    pub fn process(&mut self, sample: f32, gain: f32) -> f32 {
        let scaled = if sample.is_finite() {
            sample * gain
        } else {
            0.
        };
        if scaled.abs() > 1. {
            self.clipped += 1;
        }
        let value = scaled.clamp(-1., 1.);
        self.sum += (value as f64).powi(2);
        self.count += 1;
        self.peak = self.peak.max(value.abs());
        value
    }
    pub fn rms(&self) -> f32 {
        if self.count == 0 {
            0.
        } else {
            (self.sum / self.count as f64).sqrt() as f32
        }
    }
}
/// Five-millisecond ramp for fader changes; Stop remains immediately silent.
pub struct GainRamp {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
}
impl GainRamp {
    pub fn new(current: f32, target: f32, rate: u32) -> Self {
        let remaining = (rate / 200).max(1);
        Self {
            current,
            target,
            step: (target - current) / remaining as f32,
            remaining,
        }
    }
    pub fn advance(&mut self) -> f32 {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.current = if self.remaining == 0 {
                self.target
            } else {
                self.current + self.step
            };
        }
        self.current
    }
    pub fn current(&self) -> f32 {
        self.current
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_gain_meter_is_post_gain_and_clamping() {
        let level = AudioLevel::default();
        level.set_input_db(6.0206);
        let mut meter = SampleMeter::default();
        assert!((meter.process(0.25, level.gain()) - 0.5).abs() < 0.0001);
        assert_eq!(meter.process(0.75, level.gain()), 1.);
        assert_eq!(meter.process(f32::NAN, level.gain()), 0.);
        level.publish(&meter);
        assert!((level.rms() - (1.25f32 / 3.).sqrt()).abs() < 0.0001);
        assert_eq!(level.peak(), 1.);
        assert_eq!(level.clipped(), 1);
        level.clear_meter();
        assert_eq!(level.rms(), 0.);
        assert_eq!(level.clipped(), 1);
        level.reset_clipping();
        assert_eq!(level.clipped(), 0);
    }
    #[test]
    fn volumes_are_independent_and_mute_preserves_selected_volume() {
        let a = AudioLevel::default();
        let b = AudioLevel::default();
        a.set_volume(0.25);
        assert_eq!(a.gain(), 0.25);
        assert_eq!(b.gain(), 1.);
        a.set_muted(true);
        assert_eq!(a.gain(), 0.);
        a.set_muted(false);
        assert_eq!(a.gain(), 0.25);
        a.set_volume(f32::NAN);
        assert_eq!(a.gain(), 0.);
        a.set_input_db(f32::NAN);
        assert_eq!(a.gain(), 1.);
    }
    #[test]
    fn fader_ramp_reaches_target_without_overshoot() {
        let mut ramp = GainRamp::new(1., 0., 1000);
        let samples: Vec<_> = (0..10).map(|_| ramp.advance()).collect();
        assert!(samples.windows(2).all(|s| s[0] >= s[1]));
        assert_eq!(samples[4], 0.);
        assert_eq!(samples[9], 0.);
    }
}
