//! Spectrum / monitor analyzer panel.
//!
//! Holds the analyzer DSP state (band powers, smoothing, display gain,
//! and LUFS loudness) and exposes a GTK4 widget for rendering.

use std::cell::RefCell;
use std::rc::Rc;

use crate::analyzer::{
    self, AnalyzerLoudnessSnapshot, analyzer_db_to_display_norm, samples_to_log_band_db_values,
    smooth_power_values,
};
use crate::ebur128::{Ebur128Meter, ebur128_default_mode};

/// State managed by the analyzer panel.
pub struct AnalyzerPanel {
    /// Current smoothed band dB levels (one per analyzer band).
    pub levels: Rc<RefCell<Vec<f64>>>,
    /// Display gain applied on top of measured dB.
    pub display_gain_db: f64,
    /// Response speed (0.02 – 15.0).
    pub response_speed: f64,
    /// Whether monitoring is active.
    pub enabled: bool,
    /// Whether the display is frozen.
    pub frozen: bool,
    /// Latest loudness snapshot.
    pub loudness_snapshot: Option<AnalyzerLoudnessSnapshot>,
    /// Internal sample buffer for FFT input.
    sample_buffer: RefCell<Vec<f32>>,
    /// Previous smoothed powers for the IIR smoother.
    previous_powers: RefCell<Vec<f64>>,
    /// Frame counter for periodic analysis.
    frame_index: RefCell<usize>,
    /// LUFS meter instance.
    loudness_meter: RefCell<Option<Ebur128Meter>>,
}

impl Default for AnalyzerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerPanel {
    pub fn new() -> Self {
        Self {
            levels: Rc::new(RefCell::new(vec![0.0; analyzer::ANALYZER_BIN_COUNT])),
            display_gain_db: analyzer::ANALYZER_DISPLAY_GAIN_DEFAULT,
            response_speed: analyzer::ANALYZER_RESPONSE_DEFAULT,
            enabled: false,
            frozen: false,
            loudness_snapshot: None,
            sample_buffer: RefCell::new(Vec::new()),
            previous_powers: RefCell::new(Vec::new()),
            frame_index: RefCell::new(0),
            loudness_meter: RefCell::new(None),
        }
    }

    /// Feed a chunk of interleaved f32 audio (mono or stereo).
    pub fn feed_samples(&self, samples: &[f32]) {
        if !self.enabled || self.frozen {
            return;
        }

        let mut buffer = self.sample_buffer.borrow_mut();
        buffer.extend_from_slice(samples);

        // Keep the buffer bounded to one FFT window worth of samples.
        let fft_size = analyzer::analyzer_fft_size(crate::core::SAMPLE_RATE);
        if buffer.len() > fft_size * 2 {
            let excess = buffer.len() - fft_size;
            buffer.drain(0..excess);
        }

        let mut frame = self.frame_index.borrow_mut();
        *frame += 1;
    }

    /// Consume accumulated samples, run FFT, and update smoothed levels.
    pub fn process_fft(&self) {
        let fft_size = analyzer::analyzer_fft_size(crate::core::SAMPLE_RATE);
        let samples = self.sample_buffer.borrow();
        if samples.len() < fft_size / 2 {
            return;
        }

        let db_values = samples_to_log_band_db_values(&samples, crate::core::SAMPLE_RATE, fft_size);

        let powers = power_values_to_linear(&db_values);
        let alpha =
            analyzer::analyzer_smoothing_alpha(self.response_speed, 1584, crate::core::SAMPLE_RATE);

        let prev = self.previous_powers.borrow();
        let smoothed = smooth_power_values(&prev, &powers, alpha);
        *self.previous_powers.borrow_mut() = smoothed.clone();

        let levels: Vec<f64> = smoothed
            .iter()
            .map(|&p| {
                let db = 10.0
                    * (p.max(analyzer::ANALYZER_POWER_FLOOR))
                        .log10()
                        .max(analyzer::ANALYZER_DB_FLOOR);
                analyzer::normalize_spectrum_db(db)
            })
            .collect();

        *self.levels.borrow_mut() = levels;
    }

    /// Feed audio frames to the LUFS meter and return a snapshot.
    pub fn process_loudness(&self, frames: &[f32]) -> Option<AnalyzerLoudnessSnapshot> {
        let mut meter_opt = self.loudness_meter.borrow_mut();
        if meter_opt.is_none() {
            match Ebur128Meter::new(
                crate::core::SAMPLE_RATE as u32,
                if frames.len().is_multiple_of(2) { 2 } else { 1 },
                ebur128_default_mode(),
            ) {
                Ok(m) => *meter_opt = Some(m),
                Err(_) => return None,
            }
        }

        let meter = meter_opt.as_mut().unwrap();
        if meter.add_frames(frames).is_err() {
            return None;
        }

        let momentary = meter.momentary_lufs().ok()?;
        let shortterm = meter.shortterm_lufs().ok()?;
        let integrated = meter.integrated_lufs().ok()?;

        Some(AnalyzerLoudnessSnapshot {
            momentary_lufs: momentary,
            shortterm_lufs: shortterm,
            integrated_lufs: integrated,
        })
    }
}

/// Convert dB values back to linear power (inverse of power_values_to_db_values).
fn power_values_to_linear(db_values: &[f64]) -> Vec<f64> {
    db_values
        .iter()
        .map(|&db| 10.0_f64.powf(db / 10.0))
        .collect()
}

/// Convenience: compute display-normalized heights for a GTK drawing area.
pub fn band_display_heights(levels: &[f64], display_gain_db: f64, height: f64) -> Vec<f64> {
    let usable = height.max(1.0);
    levels
        .iter()
        .map(|&level| {
            let db = analyzer::spectrum_level_to_db(level);
            let norm = analyzer_db_to_display_norm(db, display_gain_db);
            usable * norm
        })
        .collect()
}

/// Format a LUTFS value for display.
pub fn format_lufs(value: f64) -> String {
    if !value.is_finite() {
        return "-inf LUFS".to_string();
    }
    format!("{:.1} LUFS", value)
}

/// Return whether a LUTFS value is displayable.
pub fn loudness_value_is_displayable(value: f64) -> bool {
    value.is_finite() && value >= -60.0
}
