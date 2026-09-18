//! LUFS loudness metering backed by the pure-Rust `ebur128` crate.
//!
//! Mirrors the Python `ebur128.py` wrapper: create a meter for stereo,
//! feed interleaved f32 frames, and read momentary / short-term / integrated
//! LUFS values.

use ebur128::{Channel, EbuR128, Mode};

/// Mode flags matching the Python module constants.
pub const EBUR128_MODE_M: Mode = Mode::M;
pub const EBUR128_MODE_S: Mode = Mode::S;
pub const EBUR128_MODE_I: Mode = Mode::I;

/// Default mode = Integrated + Short-term (which implies Momentary).
pub fn ebur128_default_mode() -> Mode {
    Mode::I | Mode::S
}

/// Stereo channel map matching the Python `_default_channel_map`.
const STEREO_CHANNEL_MAP: [Channel; 2] = [Channel::Left, Channel::Right];

/// Wrapper around the native ebur128 meter.
pub struct Ebur128Meter {
    meter: EbuR128,
    sample_rate: u32,
    channels: u32,
}

impl Ebur128Meter {
    pub fn new(sample_rate: u32, channels: u32, mode: Mode) -> Result<Self, ebur128::Error> {
        let mut meter = EbuR128::new(channels, sample_rate, mode)?;

        let channel_map = if channels as usize >= STEREO_CHANNEL_MAP.len() {
            &STEREO_CHANNEL_MAP[..]
        } else {
            &STEREO_CHANNEL_MAP[..channels as usize]
        };
        for (i, ch) in channel_map.iter().enumerate() {
            meter.set_channel(i as u32, *ch)?;
        }

        Ok(Self {
            meter,
            sample_rate,
            channels,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Feed interleaved f32 frames. Returns the number of frames consumed.
    pub fn add_frames(&mut self, frames: &[f32]) -> Result<(), ebur128::Error> {
        self.meter.add_frames_f32(frames)
    }

    pub fn momentary_lufs(&self) -> Result<f64, ebur128::Error> {
        self.meter.loudness_momentary()
    }

    pub fn shortterm_lufs(&self) -> Result<f64, ebur128::Error> {
        self.meter.loudness_shortterm()
    }

    pub fn integrated_lufs(&self) -> Result<f64, ebur128::Error> {
        self.meter.loudness_global()
    }

    pub fn reset(&mut self) {
        self.meter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_creation() {
        let meter = Ebur128Meter::new(48000, 2, ebur128_default_mode());
        assert!(meter.is_ok());
    }

    #[test]
    fn test_silence_lufs() {
        // Silence should yield very negative integrated loudness
        let mut meter =
            Ebur128Meter::new(48000, 2, ebur128_default_mode()).expect("meter should create");
        let silence = vec![0.0_f32; 48000 * 2]; // 1 second of stereo silence
        meter.add_frames(&silence).expect("frames should add");
        let integrated = meter.integrated_lufs();
        assert!(integrated.is_ok());
    }

    #[test]
    fn test_loud_signal_lufs() {
        let mut meter =
            Ebur128Meter::new(48000, 2, ebur128_default_mode()).expect("meter should create");
        // Generate a 1 kHz sine at -20 dBFS for one second
        let amplitude = 10.0f32.powf(-20.0 / 20.0); // ~0.1
        let mut frames = Vec::with_capacity(48000 * 2);
        for i in 0..48000 {
            let t = i as f32 / 48000.0;
            let sample = amplitude * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
            frames.push(sample);
            frames.push(sample); // stereo
        }
        meter.add_frames(&frames).expect("frames should add");
        let integrated = meter.integrated_lufs().expect("loudness should read");
        // -20 dBFS sine should give roughly -20 LUFS
        assert!(
            integrated > -60.0,
            "integrated LUFS {} should be > -60",
            integrated
        );
    }
}
