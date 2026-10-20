use std::collections::VecDeque;
use std::f64::consts::PI;

/// Offline windowed-sinc conversion of a complete phrase. This must not run in
/// the audio device callback. Low-pass cutoff prevents aliasing when downsampling.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    assert!(from > 0 && to > 0);
    if from == to {
        return input.to_vec();
    }
    let count = (input.len() as u64 * to as u64 / from as u64) as usize;
    let cutoff = (to as f64 / from as f64).min(1.0) * 0.94;
    let radius = (24.0 / cutoff).ceil() as i64;
    (0..count)
        .map(|i| {
            let pos = i as f64 * from as f64 / to as f64;
            let base = pos.floor() as i64;
            let (mut sum, mut weight) = (0.0, 0.0);
            for j in (base - radius)..=(base + radius) {
                if j < 0 || j >= input.len() as i64 {
                    continue;
                }
                let distance = pos - j as f64;
                let x = distance * cutoff;
                let sinc = if x.abs() < 1e-9 {
                    1.0
                } else {
                    (PI * x).sin() / (PI * x)
                };
                let window = 0.5 + 0.5 * (PI * distance / (radius as f64 + 1.0)).cos();
                let w = cutoff * sinc * window;
                sum += input[j as usize] as f64 * w;
                weight += w;
            }
            if weight.abs() > 1e-9 {
                (sum / weight).clamp(-1.0, 1.0) as f32
            } else {
                0.0
            }
        })
        .collect()
}

/// Carries low-pass filter phase/history across arbitrary device/network chunks.
pub struct StreamingResampler {
    buffer: Vec<f32>,
    position: f64,
    step: f64,
    cutoff: f64,
    radius: usize,
}
impl StreamingResampler {
    pub fn new(from: u32, to: u32) -> Self {
        assert!(from > 0 && to > 0);
        let cutoff = (to as f64 / from as f64).min(1.0) * 0.94;
        let radius = (24.0 / cutoff).ceil() as usize;
        Self {
            buffer: vec![0.0; radius],
            position: radius as f64,
            step: from as f64 / to as f64,
            cutoff,
            radius,
        }
    }
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        self.buffer.extend_from_slice(input);
        let mut output = Vec::new();
        while self.position + (self.radius as f64) < self.buffer.len() as f64 {
            let base = self.position.floor() as usize;
            let (mut sum, mut weight) = (0.0, 0.0);
            for index in base.saturating_sub(self.radius)..=(base + self.radius) {
                let distance = self.position - index as f64;
                let x = distance * self.cutoff;
                let sinc = if x.abs() < 1e-9 {
                    1.0
                } else {
                    (PI * x).sin() / (PI * x)
                };
                let window = 0.5 + 0.5 * (PI * distance / (self.radius as f64 + 1.0)).cos();
                let w = self.cutoff * sinc * window;
                sum += self.buffer[index] as f64 * w;
                weight += w;
            }
            output.push(if weight.abs() > 1e-9 {
                (sum / weight).clamp(-1.0, 1.0) as f32
            } else {
                0.0
            });
            self.position += self.step;
        }
        let remove = (self.position.floor() as usize).saturating_sub(self.radius);
        self.buffer.drain(..remove);
        self.position -= remove as f64;
        output
    }
    pub fn flush(&mut self) -> Vec<f32> {
        self.push(&vec![0.0; self.radius + 1])
    }
}
/// Retain an unmatched byte when an HTTP read splits a PCM16 sample.
#[derive(Default)]
pub struct Pcm16Decoder {
    pending: Option<u8>,
}
impl Pcm16Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<f32> {
        let mut data = Vec::with_capacity(bytes.len() + 1);
        if let Some(byte) = self.pending.take() {
            data.push(byte);
        }
        data.extend_from_slice(bytes);
        let (pairs, tail) = data.as_chunks::<2>();
        self.pending = tail.first().copied();
        pairs
            .iter()
            .map(|pair| i16::from_le_bytes(*pair) as f32 / 32768.0)
            .collect()
    }
    pub fn complete(&self) -> bool {
        self.pending.is_none()
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// 16-bit PCM WAV, mono; samples already converted to the desired rate.
pub fn wav_pcm16(samples: &[f32], rate: u32) -> Vec<u8> {
    let size = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    for &s in samples {
        bytes.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    bytes
}

/// Energy VAD on fixed 20 ms frames. A baseline gate, not a neural speech detector.
pub struct Segmenter {
    threshold: f32,
    pre_roll: VecDeque<Vec<f32>>,
    phrase: Vec<f32>,
    voiced_frames: usize,
    frames: usize,
    silent_frames: usize,
    pause_frames: usize,
    max_frames: usize,
}
impl Segmenter {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            pre_roll: VecDeque::new(),
            phrase: Vec::new(),
            voiced_frames: 0,
            frames: 0,
            silent_frames: 0,
            pause_frames: 25,
            max_frames: 400,
        }
    }
    pub fn live(threshold: f32) -> Self {
        Self {
            pause_frames: 18,
            max_frames: 200,
            ..Self::new(threshold)
        }
    }
    pub fn preview(&self) -> Option<Vec<f32>> {
        if self.voiced_frames >= 6 {
            Some(self.phrase.clone())
        } else {
            None
        }
    }
    pub fn reset(&mut self) {
        *self = Self {
            pause_frames: self.pause_frames,
            max_frames: self.max_frames,
            ..Self::new(self.threshold)
        };
    }
    pub fn push(&mut self, frame: &[f32]) -> Option<Vec<f32>> {
        let voiced = rms(frame) >= self.threshold;
        if self.phrase.is_empty() && !voiced {
            self.pre_roll.push_back(frame.to_vec());
            if self.pre_roll.len() > 10 {
                self.pre_roll.pop_front();
            }
            return None;
        }
        if self.phrase.is_empty() {
            for old in self.pre_roll.drain(..) {
                self.phrase.extend(old);
            }
        }
        self.phrase.extend_from_slice(frame);
        self.frames += 1;
        if voiced {
            self.voiced_frames += 1;
            self.silent_frames = 0;
        } else {
            self.silent_frames += 1;
        }
        if self.silent_frames >= self.pause_frames || self.frames >= self.max_frames {
            let phrase = std::mem::take(&mut self.phrase);
            let valid = self.voiced_frames >= 6;
            self.reset();
            if valid {
                return Some(phrase);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_resampler_is_independent_of_callback_boundaries() {
        let input: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.13).sin()).collect();
        let mut whole = StreamingResampler::new(48000, 16000);
        let expected = whole.push(&input);
        let mut split = StreamingResampler::new(48000, 16000);
        let actual: Vec<f32> = input
            .chunks(137)
            .flat_map(|part| split.push(part))
            .collect();
        assert_eq!(expected.len(), actual.len());
        assert!(expected
            .iter()
            .zip(&actual)
            .all(|(a, b)| (a - b).abs() < 1e-5));
    }
    #[test]
    fn odd_http_reads_do_not_corrupt_pcm() {
        let mut decoder = Pcm16Decoder::default();
        let bytes = [0, 0, 255, 127, 0, 128];
        let mut samples = decoder.push(&bytes[..3]);
        assert!(!decoder.complete());
        samples.extend(decoder.push(&bytes[3..]));
        assert!(decoder.complete());
        assert_eq!(samples, vec![0.0, 32767.0 / 32768.0, -1.0]);
    }
    #[test]
    fn vad_silence_short_noise_and_phrase_limit() {
        let mut s = Segmenter::new(0.02);
        for _ in 0..500 {
            assert!(s.push(&[0.0; 320]).is_none());
        }
        s.push(&[0.4; 320]);
        for _ in 0..25 {
            assert!(s.push(&[0.0; 320]).is_none());
        }
        for _ in 0..399 {
            assert!(s.push(&[0.4; 320]).is_none());
        }
        assert!(s.push(&[0.4; 320]).is_some());
    }
    #[test]
    fn phrase_ends_at_pause_and_has_pre_roll() {
        let mut s = Segmenter::new(0.02);
        for _ in 0..10 {
            s.push(&[0.0; 320]);
        }
        for _ in 0..10 {
            s.push(&[0.4; 320]);
        }
        for _ in 0..24 {
            assert!(s.push(&[0.0; 320]).is_none());
        }
        assert_eq!(s.push(&[0.0; 320]).unwrap().len(), 45 * 320);
    }
    #[test]
    fn wav_header_and_clipping() {
        let w = wav_pcm16(&[-2.0, 0.0, 2.0], 16000);
        assert_eq!(w.len(), 50);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 16000);
        assert_eq!(i16::from_le_bytes(w[48..50].try_into().unwrap()), 32767);
    }
    #[test]
    fn downsampling_rejects_out_of_band_energy() {
        let tone = |hz: f32| {
            (0..4800)
                .map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / 48000.0).sin())
                .collect::<Vec<_>>()
        };
        let low = resample(&tone(1000.0), 48000, 16000);
        let high = resample(&tone(12000.0), 48000, 16000);
        assert_eq!(low.len(), 1600);
        assert!(rms(&low[100..1500]) > 0.65);
        assert!(rms(&high[100..1500]) < 0.02);
    }
}
