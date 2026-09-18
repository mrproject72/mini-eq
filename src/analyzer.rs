//! Spectrum analyzer DSP core.
//!
//! Pure-Rust port of the original Python `analyzer.py` math: log-band
//! FFT spectrum computation, power smoothing, and dB conversion. Also
//! provides LUFS loudness snapshot types backed by the `ebur128` crate.
//!
//! The PipeWire audio-capture plumbing (`OutputSpectrumAnalyzer` in the
//! Python original) is handled by the `pipewire_backend` module; this file
//! focuses on the testable DSP pipeline.

use crate::core::clamp;
use rustfft::FftPlanner;

// ---------------------------------------------------------------------------
// Constants (mirror the Python analyzer.py)
// ---------------------------------------------------------------------------

pub const ANALYZER_BAND_FREQUENCIES: [f64; 30] = [
    25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0,
    630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0, 8000.0,
    10000.0, 12500.0, 16000.0, 20000.0,
];

pub const ANALYZER_BIN_COUNT: usize = ANALYZER_BAND_FREQUENCIES.len();
pub const ANALYZER_DB_FLOOR: f64 = -100.0;
pub const ANALYZER_INTERVAL_MS: f64 = 33.0;
pub const ANALYZER_FFT_WINDOW_SECONDS: f64 = 0.085;
pub const ANALYZER_FFT_MIN_SIZE: usize = 8192;
pub const ANALYZER_FFT_MAX_SIZE: usize = 32768;
pub const ANALYZER_SAMPLE_WIDTH_BYTES: usize = 4;
pub const ANALYZER_QUEUE_WAIT_SECONDS: f64 = 0.005;
pub const ANALYZER_READER_JOIN_TIMEOUT_SECONDS: f64 = 0.2;
pub const ANALYZER_DISPLAY_GAIN_MIN: f64 = -12.0;
pub const ANALYZER_DISPLAY_GAIN_MAX: f64 = 32.0;
pub const ANALYZER_DISPLAY_GAIN_DEFAULT: f64 = 0.0;
pub const ANALYZER_CAPTURE_QUEUE_BLOCKS: usize = 128;
pub const ANALYZER_NODE_NAME: &str = "mini-eq-analyzer";
pub const ANALYZER_NODE_DESCRIPTION: &str = "Mini EQ Monitor";
pub const ANALYZER_APPLICATION_ID: &str = "io.github.bhack.mini-eq";
pub const ANALYZER_MEDIA_CLASS: &str = "Stream/Input/Audio/Internal";

pub const ANALYZER_RESPONSE_MIN: f64 = 0.02;
pub const ANALYZER_RESPONSE_MAX: f64 = 15.0;
pub const ANALYZER_RESPONSE_DEFAULT: f64 = 2.0;
pub const ANALYZER_POWER_FLOOR: f64 = 1e-10; // 10^(-100/10)

pub const LOUDNESS_EMIT_INTERVAL_SECONDS: f64 = 0.25;

/// Loudness levels reported by the analyzer.
#[derive(Debug, Clone, Default)]
pub struct AnalyzerLoudnessSnapshot {
    pub momentary_lufs: f64,
    pub shortterm_lufs: f64,
    pub integrated_lufs: f64,
}

// ---------------------------------------------------------------------------
// Display / dB helpers
// ---------------------------------------------------------------------------

pub fn normalize_spectrum_db(db_value: f64) -> f64 {
    clamp(
        (db_value - ANALYZER_DB_FLOOR) / ANALYZER_DB_FLOOR.abs(),
        0.0,
        1.0,
    )
}

pub fn spectrum_level_to_db(level: f64) -> f64 {
    ANALYZER_DB_FLOOR + (clamp(level, 0.0, 1.0) * ANALYZER_DB_FLOOR.abs())
}

/// Map a raw dB level to a normalized [0,1] display deflection.
///
/// Uses the x42-style meter shape: hides very low noise and expands the
/// musical range.
pub fn analyzer_db_to_display_norm(db_value: f64, display_gain_db: f64) -> f64 {
    let display_db = db_value + display_gain_db;

    let deflection = if display_db < -70.0 {
        0.0
    } else if display_db < -60.0 {
        (display_db + 70.0) * 0.25
    } else if display_db < -50.0 {
        ((display_db + 60.0) * 0.5) + 2.5
    } else if display_db < -40.0 {
        ((display_db + 50.0) * 0.75) + 7.5
    } else if display_db < -30.0 {
        ((display_db + 40.0) * 1.5) + 15.0
    } else if display_db < -20.0 {
        ((display_db + 30.0) * 2.0) + 30.0
    } else if display_db < 6.0 {
        ((display_db + 20.0) * 2.5) + 50.0
    } else {
        115.0
    };

    clamp(deflection / 115.0, 0.0, 1.0)
}

pub fn analyzer_level_to_display_norm(level: f64, display_gain_db: f64) -> f64 {
    analyzer_db_to_display_norm(spectrum_level_to_db(level), display_gain_db)
}

pub fn spectrum_db_values_to_levels(db_values: &[f64]) -> Vec<f64> {
    db_values
        .iter()
        .map(|&v| normalize_spectrum_db(v))
        .collect()
}

// ---------------------------------------------------------------------------
// Sizing helpers
// ---------------------------------------------------------------------------

pub fn analyzer_frame_count(sample_rate: f64) -> usize {
    (1.0_f64.max(sample_rate) * ANALYZER_INTERVAL_MS / 1000.0) as usize
}

pub fn next_power_of_two(value: usize) -> usize {
    if value == 0 {
        return 1;
    }
    let mut n = value - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    n + 1
}

pub fn analyzer_fft_size(sample_rate: f64) -> usize {
    let target = (1.0_f64.max(sample_rate) * ANALYZER_FFT_WINDOW_SECONDS) as usize;
    let sized = next_power_of_two(target);
    sized.clamp(ANALYZER_FFT_MIN_SIZE, ANALYZER_FFT_MAX_SIZE)
}

pub fn analyzer_smoothing_alpha(response_speed: f64, frame_count: usize, sample_rate: f64) -> f64 {
    let speed = clamp(response_speed, ANALYZER_RESPONSE_MIN, ANALYZER_RESPONSE_MAX);
    1.0 - (-2.0 * std::f64::consts::PI * speed * (1.max(frame_count) as f64)
        / 1.0_f64.max(sample_rate))
    .exp()
}

// ---------------------------------------------------------------------------
// Frequency bin layout
// ---------------------------------------------------------------------------

/// Center frequencies for the analyzer bands.
pub fn analyzer_bin_center_frequencies(
    level_count: usize,
    freq_min: f64,
    freq_max: f64,
) -> Vec<f64> {
    if level_count == ANALYZER_BIN_COUNT
        && (freq_min - crate::core::GRAPH_FREQ_MIN).abs() < 1e-9
        && (freq_max - crate::core::GRAPH_FREQ_MAX).abs() < 1e-9
    {
        return ANALYZER_BAND_FREQUENCIES.to_vec();
    }

    let log_min = freq_min.ln();
    let log_span = (freq_max / freq_min).ln();
    (0..level_count)
        .map(|index| log_min + log_span * (index as f64 + 0.5) / level_count as f64)
        .map(|v| v.exp())
        .collect()
}

/// Band edges (one more than the number of bands).
pub fn analyzer_band_edges(center_frequencies: &[f64]) -> Vec<f64> {
    if center_frequencies.is_empty() {
        return Vec::new();
    }
    if center_frequencies.len() == 1 {
        let center = center_frequencies[0];
        return vec![center / 2.0_f64.sqrt(), center * 2.0_f64.sqrt()];
    }

    let middle_edges: Vec<f64> = center_frequencies
        .windows(2)
        .map(|w| (w[0] * w[1]).sqrt())
        .collect();
    let first_edge = center_frequencies[0] * center_frequencies[0] / middle_edges[0];
    let last_edge = center_frequencies[center_frequencies.len() - 1]
        * center_frequencies[center_frequencies.len() - 1]
        / middle_edges[middle_edges.len() - 1];

    let mut edges = Vec::with_capacity(center_frequencies.len() + 1);
    edges.push(first_edge);
    edges.extend(middle_edges);
    edges.push(last_edge);
    edges
}

// ---------------------------------------------------------------------------
// Sample / byte conversion helpers
// ---------------------------------------------------------------------------

/// Parse little-endian f32 bytes into samples (DC-removed).
pub fn pcm_f32le_bytes_to_samples(payload: &[u8]) -> Vec<f32> {
    let usable = payload.len() - (payload.len() % ANALYZER_SAMPLE_WIDTH_BYTES);
    let sample_count = usable / ANALYZER_SAMPLE_WIDTH_BYTES;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let offset = i * ANALYZER_SAMPLE_WIDTH_BYTES;
        let bytes = payload[offset..offset + 4].try_into().unwrap();
        samples.push(f32::from_le_bytes(bytes));
    }
    samples
}

/// Convert interleaved f32 bytes to mono by averaging channels.
pub fn interleaved_f32le_bytes_to_mono(payload: &[u8], channels: usize) -> Vec<f32> {
    let usable = payload.len() - (payload.len() % (ANALYZER_SAMPLE_WIDTH_BYTES * channels.max(1)));
    let frame_count = usable / (ANALYZER_SAMPLE_WIDTH_BYTES * channels.max(1));
    let mut mono = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let base = i * ANALYZER_SAMPLE_WIDTH_BYTES * channels.max(1);
        let mut sum = 0.0_f32;
        for c in 0..channels.max(1) {
            let offset = base + c * ANALYZER_SAMPLE_WIDTH_BYTES;
            let bytes = payload[offset..offset + 4].try_into().unwrap();
            sum += f32::from_le_bytes(bytes);
        }
        mono.push(sum / channels.max(1) as f32);
    }
    mono
}

/// Split interleaved f32 bytes into left/right channel payloads.
pub fn interleaved_f32le_bytes_to_channel_payloads(
    payload: &[u8],
    channels: usize,
) -> (Vec<f32>, Vec<f32>) {
    let channel_count = channels.max(1);
    let frame_size = ANALYZER_SAMPLE_WIDTH_BYTES * channel_count;
    let usable = payload.len() - (payload.len() % frame_size);
    let frame_count = usable / frame_size;

    if channel_count == 1 {
        let mono = interleaved_f32le_bytes_to_mono(payload, 1);
        return (mono.clone(), mono);
    }

    let mut left = Vec::with_capacity(frame_count);
    let mut right = Vec::with_capacity(frame_count);

    for i in 0..frame_count {
        let base = i * frame_size;
        let l_bytes = payload[base..base + 4].try_into().unwrap();
        let r_bytes = payload[base + 4..base + 8].try_into().unwrap();
        left.push(f32::from_le_bytes(l_bytes));
        right.push(f32::from_le_bytes(r_bytes));
    }

    (left, right)
}

// ---------------------------------------------------------------------------
// FFT computation
// ---------------------------------------------------------------------------

fn hanning_window(size: usize) -> Vec<f32> {
    let n = size as f32;
    (0..size)
        .map(|i| {
            // numpy's hanning: 0.5 - 0.5*cos(2*pi*n/(size))
            0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n).cos()
        })
        .collect()
}

fn amplitude_normalizer(window: &[f32]) -> f32 {
    window.iter().sum::<f32>() / 2.0
}

/// Overlap-weight structure for mapping FFT bins to log-band powers.
pub struct BandOverlapWeights {
    pub band_indexes: Vec<usize>,
    pub bin_indexes: Vec<usize>,
    pub weights: Vec<f64>,
    pub band_count: usize,
}

pub fn analyzer_fft_band_overlap_weights(
    fft_size: usize,
    sample_rate: f64,
    center_frequencies: &[f64],
) -> BandOverlapWeights {
    let size = fft_size.max(2);
    let sr = sample_rate.max(1.0);
    let band_count = center_frequencies.len();

    if band_count == 0 {
        return BandOverlapWeights {
            band_indexes: Vec::new(),
            bin_indexes: Vec::new(),
            weights: Vec::new(),
            band_count: 0,
        };
    }

    // rfftfreq equivalent: frequency of each bin
    let bin_count = size / 2 + 1; // rfft returns this many bins
    let frequencies: Vec<f64> = (0..bin_count)
        .map(|i| i as f64 * sr / size as f64)
        .collect();

    let bin_width = (sr / size as f64).max(1e-12);
    let mut bin_left: Vec<f64> = frequencies.iter().map(|&f| f - bin_width * 0.5).collect();
    let mut bin_right: Vec<f64> = frequencies.iter().map(|&f| f + bin_width * 0.5).collect();
    bin_left[0] = 0.0;
    let last = bin_right.len() - 1;
    bin_right[last] = (sr * 0.5).min(bin_right[last]);

    let bin_widths: Vec<f64> = bin_right
        .iter()
        .zip(bin_left.iter())
        .map(|(r, l)| (*r - *l).max(1e-12))
        .collect();

    let edges = analyzer_band_edges(center_frequencies);
    let nyquist = sr * 0.5;
    let band_left: Vec<f64> = edges[..edges.len() - 1]
        .iter()
        .map(|&e| e.clamp(0.0, nyquist))
        .collect();
    let band_right: Vec<f64> = edges[1..].iter().map(|&e| e.clamp(0.0, nyquist)).collect();

    let mut band_indexes = Vec::new();
    let mut bin_indexes = Vec::new();
    let mut weights = Vec::new();

    for (b_idx, (&b_left, &b_right)) in band_left.iter().zip(band_right.iter()).enumerate() {
        for (bin_idx, (bl, br)) in bin_left.iter().zip(bin_right.iter()).enumerate() {
            let overlap_left = b_left.max(*bl);
            let overlap_right = b_right.min(*br);
            let overlap = (overlap_right - overlap_left).max(0.0);
            if overlap > 0.0 {
                let weight = overlap / bin_widths[bin_idx];
                band_indexes.push(b_idx);
                bin_indexes.push(bin_idx);
                weights.push(weight);
            }
        }
    }

    BandOverlapWeights {
        band_indexes,
        bin_indexes,
        weights,
        band_count,
    }
}

/// Compute log-band power levels from a sample buffer.
pub fn samples_to_log_band_powers(samples: &[f32], sample_rate: f64, fft_size: usize) -> Vec<f64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let size = fft_size.max(2);
    let window = hanning_window(size);
    let normalizer = amplitude_normalizer(&window);

    // Take last `size` samples, zero-padded
    let mut input: Vec<f32> = vec![0.0; size];
    let copy_len = samples.len().min(size);
    input[size - copy_len..].copy_from_slice(&samples[samples.len() - copy_len..]);

    // Apply window
    let windowed: Vec<f32> = input
        .iter()
        .zip(window.iter())
        .map(|(s, w)| s * w)
        .collect();

    // rfft
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(size);

    let mut complex: Vec<num_complex::Complex<f32>> = windowed
        .iter()
        .map(|&s| num_complex::Complex::new(s, 0.0))
        .collect();
    fft.process(&mut complex);

    // bin_powers = (|spectrum| / normalizer)^2, DC = 0
    let bin_count = size / 2 + 1;
    let mut bin_powers: Vec<f64> = vec![0.0; bin_count];
    for (i, c) in complex.iter().enumerate().take(bin_count) {
        if i == 0 {
            bin_powers[i] = 0.0;
        } else {
            let mag = c.norm() as f64 / normalizer as f64;
            bin_powers[i] = mag * mag;
        }
    }

    // Map to log bands
    let overlap = analyzer_fft_band_overlap_weights(size, sample_rate, &ANALYZER_BAND_FREQUENCIES);

    if overlap.band_count == 0 || overlap.bin_indexes.is_empty() {
        return vec![0.0; overlap.band_count];
    }

    let mut band_powers = vec![0.0_f64; overlap.band_count];
    for i in 0..overlap.bin_indexes.len() {
        let bin_idx = overlap.bin_indexes[i];
        let band_idx = overlap.band_indexes[i];
        let weight = overlap.weights[i];
        band_powers[band_idx] += bin_powers[bin_idx] * weight;
    }

    band_powers
}

// ---------------------------------------------------------------------------
// Post-processing
// ---------------------------------------------------------------------------

pub fn smooth_power_values(previous: &[f64], current: &[f64], alpha: f64) -> Vec<f64> {
    if current.is_empty() {
        return previous.to_vec();
    }
    let mix = clamp(alpha, 0.0, 1.0);
    let prev_vec: Vec<f64> = if previous.len() == current.len() {
        previous.to_vec()
    } else {
        vec![0.0; current.len()]
    };
    prev_vec
        .iter()
        .zip(current.iter())
        .map(|(&old, &new)| old + mix * (new - old))
        .collect()
}

pub fn power_values_to_db_values(power_values: &[f64]) -> Vec<f64> {
    power_values
        .iter()
        .map(|&power| 10.0 * (power.max(ANALYZER_POWER_FLOOR)).log10())
        .map(|db| db.max(ANALYZER_DB_FLOOR))
        .collect()
}

pub fn samples_to_log_band_db_values(
    samples: &[f32],
    sample_rate: f64,
    fft_size: usize,
) -> Vec<f64> {
    let powers = samples_to_log_band_powers(samples, sample_rate, fft_size);
    power_values_to_db_values(&powers)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SAMPLE_RATE;

    #[test]
    fn test_analyzer_bin_count() {
        assert_eq!(ANALYZER_BIN_COUNT, 30);
    }

    #[test]
    fn test_analyzer_fft_size() {
        // 48000 * 0.085 = 4080, next pow2 = 4096, clamped to [8192, 32768] = 8192
        let size = analyzer_fft_size(SAMPLE_RATE);
        assert!(size >= ANALYZER_FFT_MIN_SIZE);
        assert!(size <= ANALYZER_FFT_MAX_SIZE);
        assert!(size.is_power_of_two());
    }

    #[test]
    fn test_next_power_of_two() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(7), 8);
        assert_eq!(next_power_of_two(8), 8);
        assert_eq!(next_power_of_two(9), 16);
        assert_eq!(next_power_of_two(4080), 4096);
    }

    #[test]
    fn test_analyzer_frame_count() {
        assert_eq!(analyzer_frame_count(SAMPLE_RATE), 1584);
    }

    #[test]
    fn test_bin_center_frequencies() {
        let freqs = analyzer_bin_center_frequencies(
            ANALYZER_BIN_COUNT,
            crate::core::GRAPH_FREQ_MIN,
            crate::core::GRAPH_FREQ_MAX,
        );
        assert_eq!(freqs.len(), 30);
        assert!((freqs[0] - 25.0).abs() < 0.01);
        assert!((freqs[29] - 20000.0).abs() < 0.01);
    }

    #[test]
    fn test_band_edges() {
        let edges = analyzer_band_edges(&ANALYZER_BAND_FREQUENCIES.to_vec());
        assert_eq!(edges.len(), 31);
        assert!(edges.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn test_normalize_spectrum_db() {
        assert_eq!(normalize_spectrum_db(0.0), 1.0);
        assert_eq!(normalize_spectrum_db(ANALYZER_DB_FLOOR), 0.0);
        assert_eq!(normalize_spectrum_db(-50.0), 0.5);
    }

    #[test]
    fn test_spectrum_level_to_db() {
        assert_eq!(spectrum_level_to_db(1.0), 0.0);
        assert_eq!(spectrum_level_to_db(0.0), ANALYZER_DB_FLOOR);
        assert_eq!(spectrum_level_to_db(0.5), -50.0);
    }

    #[test]
    fn test_analyzer_db_to_display_norm() {
        assert_eq!(analyzer_db_to_display_norm(-70.0, 0.0), 0.0);
        assert_eq!(analyzer_db_to_display_norm(6.0, 0.0), 1.0);
        // with display gain
        assert!(analyzer_db_to_display_norm(-70.0, 10.0) > 0.0);
    }

    #[test]
    fn test_smoothing_alpha() {
        let alpha = analyzer_smoothing_alpha(ANALYZER_RESPONSE_DEFAULT, 1584, SAMPLE_RATE);
        assert!(alpha > 0.0 && alpha < 1.0);
    }

    #[test]
    fn test_silence_powers_zero() {
        let samples = vec![0.0_f32; 16000];
        let powers =
            samples_to_log_band_powers(&samples, SAMPLE_RATE, analyzer_fft_size(SAMPLE_RATE));
        assert!(powers.iter().all(|&p| p <= 1e-10));
    }

    #[test]
    fn test_sine_waves_peak_at_frequency() {
        // Generate a 1000 Hz sine at 48 kHz, sample for one FFT window
        let fft_size = analyzer_fft_size(SAMPLE_RATE);
        let mut samples = Vec::with_capacity(fft_size);
        for i in 0..fft_size {
            let t = i as f32 / SAMPLE_RATE as f32;
            samples.push((2.0 * std::f32::consts::PI * 1000.0 * t).sin());
        }
        let db_values = samples_to_log_band_db_values(&samples, SAMPLE_RATE, fft_size);
        assert_eq!(db_values.len(), ANALYZER_BIN_COUNT);
        // The 1000 Hz band should have the peak response
        let max_db = db_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let max_idx = db_values
            .iter()
            .position(|&v| (v - max_db).abs() < 1e-9)
            .unwrap();
        // 1000 Hz is in the 17th band (index 16) of the standard analyzer frequencies
        assert_eq!(max_idx, 16);
    }

    #[test]
    fn test_pcm_f32le_bytes_to_samples() {
        let bytes: Vec<u8> = vec![0x00, 0x00, 0x80, 0x3f]; // 1.0 in f32 LE
        let samples = pcm_f32le_bytes_to_samples(&bytes);
        assert_eq!(samples.len(), 1);
        assert!((samples[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_interleaved_to_mono() {
        // 2 channels, 2 samples: L=[1.0, 2.0], R=[3.0, 4.0]
        let mut bytes = Vec::new();
        for v in [1.0_f32, 3.0, 2.0, 4.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let mono = interleaved_f32le_bytes_to_mono(&bytes, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 2.0).abs() < 1e-6); // (1+3)/2
        assert!((mono[1] - 3.0).abs() < 1e-6); // (2+4)/2
    }
}
