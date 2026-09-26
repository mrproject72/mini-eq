use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};

/// Clamp a level to [0.0, 1.0].
pub fn clamp_level(level: f64) -> f64 {
    level.clamp(0.0, 1.0)
}

/// Clamp a value to the range [lower, upper].
pub fn clamp<T: PartialOrd>(value: T, lower: T, upper: T) -> T {
    if value < lower {
        lower
    } else if value > upper {
        upper
    } else {
        value
    }
}

// ── Application ──────────────────────────────────────────────────────────────

pub const APP_NAME: &str = "Mini EQ";
pub const OUTPUT_CLIENT_NAME: &str = "Mini EQ Output";
pub const VIRTUAL_SINK_BASE: &str = "mini_eq_sink";
pub const VIRTUAL_SINK_DESCRIPTION: &str = "Mini-EQ-Sink";
pub const FILTER_OUTPUT_SUFFIX: &str = "_output";

// ── Band Configuration ───────────────────────────────────────────────────────

pub const MAX_BANDS: usize = 32;
pub const DEFAULT_ACTIVE_BANDS: usize = 10;
pub const PRESET_VERSION: i32 = 1;
pub const PRESET_FILE_SUFFIX: &str = ".json";
pub const OUTPUT_PRESET_LINKS_VERSION: i32 = 1;
pub const OUTPUT_PRESET_LINKS_FILE: &str = "output-presets.json";
pub const OUTPUT_PRESET_ROUTE_KEY_PREFIX: &str = "pipewire-route:v1:";

// ── EQ Modes ─────────────────────────────────────────────────────────────────

pub const EQ_MODES: [&str; 1] = ["Live PipeWire"];
// Upstream `EQ_MODE_APO = 6` (the index into the full 12-entry `FILTER_TYPES`
// map); it is *not* the index into the one-entry `EQ_MODES` map. Band `mode`
// fields persisted by the Python original use this value.
pub const EQ_MODE_APO: i32 = 6;

/// Combo-box index for a selectable filter type, mirroring upstream
/// `FILTER_TYPE_INDEX_BY_VALUE`.
///
/// This is *not* the enum discriminant: `Resonance` (7) is absent from
/// `SELECTABLE_FILTER_TYPES`, so the higher values shift down (`Allpass` 8 -> 7,
/// `Bandpass` 9 -> 8). Indexing a table by discriminant would mis-select
/// `Allpass` and run off the end for `Bandpass`.
pub fn filter_type_combo_index(filter_type: FilterType) -> usize {
    SELECTABLE_FILTER_TYPES
        .iter()
        .position(|candidate| *candidate == filter_type)
        .unwrap_or(0)
}

/// Inverse of [`filter_type_combo_index`].
pub fn filter_type_from_combo_index(index: usize) -> FilterType {
    SELECTABLE_FILTER_TYPES
        .get(index)
        .copied()
        .unwrap_or(FilterType::Off)
}

pub const MODE_ORDER: [&str; 1] = ["Live PipeWire"];
pub const MODE_INDEX_BY_VALUE: &[usize] = &[0];

// ── File paths ───────────────────────────────────────────────────────────────

pub const APP_ID: &str = "io.github.bhack.mini-eq";

/// XDG config home or `~/.config`.
pub fn user_config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && Path::new(&xdg).is_absolute()
    {
        return PathBuf::from(xdg);
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".config")
}

/// `~/.config/mini-eq`
pub fn app_config_dir() -> PathBuf {
    user_config_dir().join("mini-eq")
}

/// `$XDG_CONFIG_HOME/mini-eq/{file_name}`
pub fn app_config_file_path(file_name: &str) -> PathBuf {
    app_config_dir().join(file_name)
}

/// `$XDG_DATA_HOME/mini-eq/{file_name}` or `~/.local/share/mini-eq/{file_name}`
pub fn app_data_file_path(file_name: &str) -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|p| Path::new(p).is_absolute())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
                .join(".local/share")
        });
    data_home.join("mini-eq").join(file_name)
}

// ── Filter Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterType {
    Off = 0,
    Bell = 1,
    HiPass = 2,
    HiShelf = 3,
    LoPass = 4,
    LoShelf = 5,
    Notch = 6,
    Resonance = 7,
    Allpass = 8,
    Bandpass = 9,
    LadderPass = 10,
    LadderRej = 11,
}

impl FilterType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Off" => Some(Self::Off),
            "Bell" => Some(Self::Bell),
            "Hi-pass" => Some(Self::HiPass),
            "Hi-shelf" => Some(Self::HiShelf),
            "Lo-pass" => Some(Self::LoPass),
            "Lo-shelf" => Some(Self::LoShelf),
            "Notch" => Some(Self::Notch),
            "Resonance" => Some(Self::Resonance),
            "Allpass" => Some(Self::Allpass),
            "Bandpass" => Some(Self::Bandpass),
            "Ladder-pass" => Some(Self::LadderPass),
            "Ladder-rej" => Some(Self::LadderRej),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Off => "Off",
            Self::Bell => "Bell",
            Self::HiPass => "Hi-pass",
            Self::HiShelf => "Hi-shelf",
            Self::LoPass => "Lo-pass",
            Self::LoShelf => "Lo-shelf",
            Self::Notch => "Notch",
            Self::Resonance => "Resonance",
            Self::Allpass => "Allpass",
            Self::Bandpass => "Bandpass",
            Self::LadderPass => "Ladder-pass",
            Self::LadderRej => "Ladder-rej",
        }
    }

    /// SPA biquad label for this filter type.
    ///
    /// Mirrors upstream `NATIVE_BIQUAD_LABELS`; types without a native label
    /// fall back to `bq_peaking` and are bypassed via the mixer wet/dry gain.
    pub fn native_label(&self) -> &'static str {
        match self {
            Self::Off | Self::Bell | Self::Resonance | Self::LadderPass | Self::LadderRej => {
                "bq_peaking"
            }
            Self::HiPass => "bq_highpass",
            Self::HiShelf => "bq_highshelf",
            Self::LoPass => "bq_lowpass",
            Self::LoShelf => "bq_lowshelf",
            Self::Notch => "bq_notch",
            Self::Allpass => "bq_allpass",
            Self::Bandpass => "bq_bandpass",
        }
    }

    /// Whether this type maps to a native SPA biquad (upstream
    /// `filter_type in NATIVE_BIQUAD_LABELS`). Types that do not are fully
    /// bypassed by the mixer rather than processed.
    pub fn has_native_biquad(&self) -> bool {
        matches!(
            self,
            Self::Off
                | Self::Bell
                | Self::HiPass
                | Self::HiShelf
                | Self::LoPass
                | Self::LoShelf
                | Self::Notch
                | Self::Allpass
                | Self::Bandpass
        )
    }
}

pub const SELECTABLE_FILTER_TYPES: [FilterType; 9] = [
    FilterType::Off,
    FilterType::Bell,
    FilterType::HiPass,
    FilterType::HiShelf,
    FilterType::LoPass,
    FilterType::LoShelf,
    FilterType::Notch,
    FilterType::Allpass,
    FilterType::Bandpass,
];

// ── EQ Parameters ────────────────────────────────────────────────────────────

pub const SAMPLE_RATE: f64 = 48000.0;
pub const GRAPH_FREQ_MIN: f64 = 20.0;
pub const GRAPH_FREQ_MAX: f64 = 20000.0;
pub const GRAPH_DB_MIN: f64 = -24.0;
pub const GRAPH_DB_MAX: f64 = 24.0;
pub const RESPONSE_PEAK_F_STEP: f64 = 1.02;

pub const EQ_FREQUENCY_MIN_HZ: f64 = 20.0;
pub const EQ_FREQUENCY_MAX_HZ: f64 = 20000.0;
pub const EQ_GAIN_MIN_DB: f64 = -20.0;
pub const EQ_GAIN_MAX_DB: f64 = 20.0;
pub const EQ_Q_MIN: f64 = 0.18248;
pub const EQ_Q_MAX: f64 = 6.0;
/// Upstream `DEFAULT_BAND_Q = 1.0 / math.sqrt(2.0)`.
pub const DEFAULT_BAND_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;
pub const EQ_PREAMP_MIN_DB: f64 = -24.0;
pub const EQ_PREAMP_MAX_DB: f64 = 6.0;

// ── Biquad Coefficients ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiquadCoefficients {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a0: f64,
    pub a1: f64,
    pub a2: f64,
}

impl BiquadCoefficients {
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    pub fn is_identity(&self) -> bool {
        (self.b0 - 1.0).abs() < 1e-12
            && self.b1.abs() < 1e-12
            && self.b2.abs() < 1e-12
            && (self.a0 - 1.0).abs() < 1e-12
            && self.a1.abs() < 1e-12
            && self.a2.abs() < 1e-12
    }

    pub fn as_tuple(&self) -> (f64, f64, f64, f64, f64, f64) {
        (self.b0, self.b1, self.b2, self.a0, self.a1, self.a2)
    }

    pub fn as_array(&self) -> [f64; 6] {
        [self.b0, self.b1, self.b2, self.a0, self.a1, self.a2]
    }

    /// Mirror of upstream `scaled_for_control_range`: PipeWire's `bq_raw`
    /// controls must stay within +/-10 or the node fails to configure.
    pub fn scaled_for_control_range(&self, limit: f64) -> Self {
        let max_abs = self
            .as_array()
            .iter()
            .fold(0.0_f64, |acc, v| acc.max(v.abs()));
        if max_abs <= limit || max_abs == 0.0 {
            return self.clone();
        }

        let scale = limit / max_abs;
        Self {
            b0: self.b0 * scale,
            b1: self.b1 * scale,
            b2: self.b2 * scale,
            a0: self.a0 * scale,
            a1: self.a1 * scale,
            a2: self.a2 * scale,
        }
    }
}

pub fn identity_biquad_coefficients(gain: f64) -> BiquadCoefficients {
    if gain == 1.0 {
        return BiquadCoefficients::identity();
    }
    BiquadCoefficients {
        b0: gain,
        b1: 0.0,
        b2: 0.0,
        a0: 1.0,
        a1: 0.0,
        a2: 0.0,
    }
}

// ── EQ Band ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EqBand {
    pub index: usize,
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
    pub filter_type: FilterType,
    pub mute: bool,
    pub solo: bool,
    pub coefficients: BiquadCoefficients,
}

impl EqBand {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            frequency: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            filter_type: FilterType::Off,
            mute: false,
            solo: false,
            coefficients: BiquadCoefficients::identity(),
        }
    }

    pub fn is_effective(&self) -> bool {
        !self.mute && self.filter_type != FilterType::Off
    }
}

// ── Biquad Coefficient Calculation ───────────────────────────────────────────

pub fn band_biquad_coefficients(
    band: &EqBand,
    sample_rate: f64,
    solo_active: bool,
) -> BiquadCoefficients {
    if !band_is_effective(band, solo_active) || !band.filter_type.is_selectable() {
        return BiquadCoefficients::identity();
    }

    // Upstream clamps to the Nyquist limit (`sample_rate / 2 - 1`) and floors Q
    // at 0.0001, not at the UI's EQ_Q_MIN.
    let center_max = EQ_FREQUENCY_MAX_HZ.min((sample_rate * 0.5) - 1.0);
    let f0 = band.frequency.clamp(EQ_FREQUENCY_MIN_HZ, center_max);
    let gain = band.gain_db.clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB);
    let q = band.q.max(0.0001);
    let a = 10.0_f64.powf(gain / 40.0);
    let omega = 2.0 * PI * f0 / sample_rate;
    let sin_omega = omega.sin();
    let cos_omega = omega.cos();
    let alpha = sin_omega / (2.0 * q);

    let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
        FilterType::Bell => {
            let b0 = 1.0 + alpha * a;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0 - alpha * a;
            let a0 = 1.0 + alpha / a;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha / a;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::HiPass => {
            let b0 = (1.0 + cos_omega) / 2.0;
            let b1 = -(1.0 + cos_omega);
            let b2 = (1.0 + cos_omega) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::LoPass => {
            let b0 = (1.0 - cos_omega) / 2.0;
            let b1 = 1.0 - cos_omega;
            let b2 = (1.0 - cos_omega) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::HiShelf => {
            let beta = 2.0 * a.sqrt() * alpha;
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + beta);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - beta);
            let a0 = (a + 1.0) - (a - 1.0) * cos_omega + beta;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
            let a2 = (a + 1.0) - (a - 1.0) * cos_omega - beta;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::LoShelf => {
            let beta = 2.0 * a.sqrt() * alpha;
            let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + beta);
            let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
            let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - beta);
            let a0 = (a + 1.0) + (a - 1.0) * cos_omega + beta;
            let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
            let a2 = (a + 1.0) + (a - 1.0) * cos_omega - beta;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Notch => {
            let b0 = 1.0;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Allpass => {
            let b0 = 1.0 - alpha;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0 + alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Bandpass => {
            let b0 = alpha;
            let b1 = 0.0;
            let b2 = -alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Resonance => {
            let b0 = 1.0;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0;
            let a0 = 1.0 + alpha / q;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha / q;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::LadderPass => {
            let b0 = (1.0 - cos_omega) / 2.0;
            let b1 = 1.0 - cos_omega;
            let b2 = (1.0 - cos_omega) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::LadderRej => {
            let b0 = 1.0 + alpha;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0 - alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Off => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    };

    // Normalize by a0
    let norm = a0;
    BiquadCoefficients {
        b0: b0 / norm,
        b1: b1 / norm,
        b2: b2 / norm,
        a0: 1.0,
        a1: a1 / norm,
        a2: a2 / norm,
    }
}

pub fn bands_have_solo(bands: &[EqBand]) -> bool {
    bands.iter().any(|band| band.solo)
}

pub fn db_to_linear(value_db: f64) -> f64 {
    10.0_f64.powf(value_db / 20.0)
}

pub fn band_is_effective(band: &EqBand, solo_active: bool) -> bool {
    !band.mute && band.filter_type != FilterType::Off && (!solo_active || band.solo)
}

impl FilterType {
    fn is_selectable(&self) -> bool {
        matches!(
            self,
            FilterType::Off
                | FilterType::Bell
                | FilterType::HiPass
                | FilterType::HiShelf
                | FilterType::LoPass
                | FilterType::LoShelf
                | FilterType::Notch
                | FilterType::Allpass
                | FilterType::Bandpass
        )
    }
}

// ── DSP / Response Helpers ────────────────────────────────────────────────────

pub fn format_frequency(value: f64) -> String {
    if value >= 1000.0 {
        format!("{:.1}k", value / 1000.0)
    } else {
        format!("{}", value.round() as i64)
    }
}

pub fn log_response_frequencies(sample_rate: f64, f_step: f64) -> Vec<f64> {
    let max_frequency = GRAPH_FREQ_MAX.min((sample_rate * 0.5) - 1.0);
    if max_frequency <= GRAPH_FREQ_MIN {
        return vec![1.0_f64.max(max_frequency)];
    }

    let f_step = f_step.max(1.0001);
    let mut frequencies = Vec::new();
    let mut frequency = GRAPH_FREQ_MIN;
    while frequency <= max_frequency {
        frequencies.push(frequency);
        frequency *= f_step;
    }

    if let Some(&last) = frequencies.last() {
        if last < max_frequency {
            frequencies.push(max_frequency);
        }
    }

    frequencies
}

pub fn stepped_response_frequencies(sample_rate: f64, steps: i32) -> Vec<f64> {
    let max_frequency = GRAPH_FREQ_MAX.min((sample_rate * 0.5) - 1.0);
    if max_frequency <= GRAPH_FREQ_MIN {
        return vec![1.0_f64.max(max_frequency)];
    }

    (0..steps.max(2))
        .map(|i| {
            let t = i as f64 / (steps.max(2) - 1) as f64;
            GRAPH_FREQ_MIN * (max_frequency / GRAPH_FREQ_MIN).powf(t)
        })
        .collect()
}

pub fn biquad_response_at_frequency(
    coefficients: &BiquadCoefficients,
    sample_rate: f64,
    frequency: f64,
) -> Complex64 {
    let frequency = frequency.clamp(1.0, (sample_rate * 0.5) - 1.0);
    let omega = 2.0 * PI * frequency / sample_rate;

    let z1 = Complex64::new(omega.cos(), -omega.sin());
    let z2 = z1 * z1;

    let numerator = coefficients.b0 + coefficients.b1 * z1 + coefficients.b2 * z2;
    let denominator = coefficients.a0 + coefficients.a1 * z1 + coefficients.a2 * z2;

    if denominator.norm() < 1e-12 {
        Complex64::new(1.0, 0.0)
    } else {
        numerator / denominator
    }
}

pub fn total_response_db(
    bands: &[EqBand],
    preamp_db: f64,
    sample_rate: f64,
    frequency: f64,
) -> f64 {
    let mut response = Complex64::new(1.0, 0.0);
    let solo_active = bands.iter().any(|b| b.solo);

    for band in bands {
        let coeffs = band_biquad_coefficients(band, sample_rate, solo_active);
        response *= biquad_response_at_frequency(&coeffs, sample_rate, frequency);
    }

    let magnitude = response.norm().max(1e-12);
    let db = preamp_db + 20.0 * magnitude.log10();
    db.clamp(GRAPH_DB_MIN - 12.0, GRAPH_DB_MAX + 12.0)
}

pub fn total_response_db_at_frequencies(
    bands: &[EqBand],
    preamp_db: f64,
    sample_rate: f64,
    frequencies: &[f64],
) -> Vec<f64> {
    frequencies
        .iter()
        .map(|&f| total_response_db(bands, preamp_db, sample_rate, f))
        .collect()
}

/// Frequencies used by [`estimate_response_peak_db`], mirroring upstream
/// `response_peak_frequencies`.
///
/// The dense log sweep can straddle a narrow filter's centre and miss its peak,
/// so every effective band's centre frequency is unioned in.
pub fn response_peak_frequencies(bands: &[EqBand], sample_rate: f64) -> Vec<f64> {
    let mut frequencies = log_response_frequencies(sample_rate, RESPONSE_PEAK_F_STEP);

    let solo_active = bands_have_solo(bands);
    let max_frequency = GRAPH_FREQ_MAX.min((sample_rate * 0.5) - 1.0);
    for band in bands {
        if band_is_effective(band, solo_active) {
            frequencies.push(band.frequency.clamp(GRAPH_FREQ_MIN, max_frequency));
        }
    }

    frequencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    frequencies.dedup();
    frequencies
}

pub fn estimate_response_peak_db(bands: &[EqBand], preamp_db: f64, sample_rate: f64) -> f64 {
    let frequencies = response_peak_frequencies(bands, sample_rate);
    let responses = total_response_db_at_frequencies(bands, preamp_db, sample_rate, &frequencies);
    responses
        .into_iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(preamp_db)
}

// ── Default Band Presets ─────────────────────────────────────────────────────

/// Log-spaced band defaults, mirroring upstream
/// `compute_log_spaced_band_defaults(num_bands)`.
///
/// Returns `(frequency, q)` pairs. Note that the spacing — and therefore the
/// frequencies — depend on `num_bands`, so the active-band defaults are *not*
/// a prefix of the `MAX_BANDS` defaults.
pub fn compute_log_spaced_band_defaults(num_bands: usize) -> Vec<(f64, f64)> {
    if num_bands == 0 {
        return Vec::new();
    }

    let freq_min = GRAPH_FREQ_MIN;
    let freq_max = GRAPH_FREQ_MAX;
    let mut freq0 = freq_min;
    let step = (freq_max / freq_min).powf(1.0 / num_bands as f64);

    let mut defaults = Vec::with_capacity(num_bands);
    for _ in 0..num_bands {
        let freq1 = freq0 * step;
        let freq = freq0 + 0.5 * (freq1 - freq0);
        let width = freq1 - freq0;
        let q_value = freq / width;
        defaults.push((freq, q_value));
        freq0 = freq1;
    }

    defaults
}

/// All `MAX_BANDS` bands, inactive (`Off`), mirroring upstream `inactive_eq_bands()`.
pub fn inactive_eq_bands() -> Vec<EqBand> {
    compute_log_spaced_band_defaults(MAX_BANDS)
        .into_iter()
        .enumerate()
        .map(|(index, (frequency, q_value))| {
            let mut band = EqBand::new(index);
            band.frequency = frequency;
            band.gain_db = 0.0;
            band.q = q_value;
            band.filter_type = FilterType::Off;
            band.coefficients = BiquadCoefficients::identity();
            band
        })
        .collect()
}

/// The default band set: `MAX_BANDS` bands with the first
/// `DEFAULT_ACTIVE_BANDS` enabled as bells, mirroring upstream `default_eq_bands()`.
pub fn default_bands() -> Vec<EqBand> {
    let mut bands = inactive_eq_bands();

    for (index, (frequency, q_value)) in compute_log_spaced_band_defaults(DEFAULT_ACTIVE_BANDS)
        .into_iter()
        .enumerate()
    {
        if let Some(band) = bands.get_mut(index) {
            band.filter_type = FilterType::Bell;
            band.frequency = frequency;
            band.q = q_value;
        }
    }

    bands
}

pub fn eq_band_to_dict(band: &EqBand) -> serde_json::Value {
    serde_json::json!({
        "filter_type": band.filter_type as u8,
        "frequency": band.frequency,
        "gain_db": band.gain_db,
        "q": band.q,
        "mute": band.mute,
        "solo": band.solo,
    })
}

pub fn eq_band_from_dict(data: &serde_json::Value, fallback: &EqBand) -> EqBand {
    let raw_filter_type = data
        .get("filter_type")
        .and_then(|v| v.as_f64())
        .unwrap_or(fallback.filter_type as u8 as f64)
        .clamp(0.0, 11.0) as u8;
    // Upstream only accepts the selectable types; Resonance/Ladder-* are
    // coerced to Off when loaded.
    let mut filter_type = match raw_filter_type {
        1 => FilterType::Bell,
        2 => FilterType::HiPass,
        3 => FilterType::HiShelf,
        4 => FilterType::LoPass,
        5 => FilterType::LoShelf,
        6 => FilterType::Notch,
        8 => FilterType::Allpass,
        9 => FilterType::Bandpass,
        _ => FilterType::Off,
    };
    if !filter_type.is_selectable() {
        filter_type = FilterType::Off;
    }

    // Upstream persists `mute`; `enabled` is still accepted (inverted) for
    // presets written by earlier builds of this port.
    let mute = match (
        data.get("mute").and_then(|v| v.as_bool()),
        data.get("enabled").and_then(|v| v.as_bool()),
    ) {
        (Some(mute), _) => mute,
        (None, Some(enabled)) => !enabled,
        (None, None) => fallback.mute,
    };

    EqBand {
        index: fallback.index,
        frequency: data
            .get("frequency")
            .and_then(|v| v.as_f64())
            .unwrap_or(fallback.frequency)
            .clamp(EQ_FREQUENCY_MIN_HZ, EQ_FREQUENCY_MAX_HZ),
        gain_db: data
            .get("gain_db")
            .and_then(|v| v.as_f64())
            .unwrap_or(fallback.gain_db)
            .clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB),
        q: data
            .get("q")
            .and_then(|v| v.as_f64())
            .unwrap_or(fallback.q)
            .clamp(EQ_Q_MIN, EQ_Q_MAX),
        filter_type,
        mute,
        solo: data
            .get("solo")
            .and_then(|v| v.as_bool())
            .unwrap_or(fallback.solo),
        coefficients: BiquadCoefficients::identity(),
    }
}

pub fn preset_payload(bands: &[EqBand], preamp_db: f64) -> serde_json::Value {
    let bands_array: Vec<serde_json::Value> = bands.iter().map(eq_band_to_dict).collect();
    serde_json::json!({
        "version": PRESET_VERSION,
        "preamp_db": preamp_db,
        "bands": bands_array,
    })
}

pub fn preset_payload_bands(payload: &serde_json::Value) -> anyhow::Result<Vec<EqBand>> {
    let bands_data = payload
        .get("bands")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    // Base on the full MAX_BANDS set: `bands_data` may carry up to MAX_BANDS
    // entries, so seeding from `default_bands()` (DEFAULT_ACTIVE_BANDS long)
    // would index out of bounds.
    let fallback = inactive_eq_bands();
    let mut bands = fallback.clone();
    for (i, band_data) in bands_data.iter().take(MAX_BANDS).enumerate() {
        bands[i] = eq_band_from_dict(band_data, &fallback[i]);
    }
    Ok(bands)
}

/// Canonical signature of a preset payload, used to detect "modified" state.
///
/// Mirrors upstream `preset_payload_state_signature`: bands are normalised
/// through the same from_dict/to_dict round trip, values are clamped, and keys
/// are sorted with compact separators so two equivalent presets compare equal.
pub fn preset_payload_state_signature(payload: &serde_json::Value) -> String {
    let bands_data = payload
        .get("bands")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut bands = inactive_eq_bands();
    for (index, band_data) in bands_data.iter().take(MAX_BANDS).enumerate() {
        if !band_data.is_object() {
            continue;
        }
        bands[index] = eq_band_from_dict(band_data, &bands[index]);
    }

    let preamp_db = payload
        .get("preamp_db")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(EQ_PREAMP_MIN_DB, EQ_PREAMP_MAX_DB);
    let signature_payload = serde_json::json!({
        "version": PRESET_VERSION,
        "preamp_db": preamp_db,
        "bands": bands.iter().map(eq_band_to_dict).collect::<Vec<_>>(),
    });
    canonical_json(&signature_payload)
}

/// Serialize JSON with sorted object keys and no insignificant whitespace,
/// matching Python's `json.dumps(..., sort_keys=True, separators=(",", ":"))`.
fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::Value::String((*k).clone()),
                        canonical_json(&map[*k])
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", inner)
        }
        serde_json::Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => other.to_string(),
    }
}

pub fn save_preset_to_file(path: &Path, bands: &[EqBand], preamp_db: f64) -> anyhow::Result<()> {
    let payload = preset_payload(bands, preamp_db);
    let data = serde_json::to_string_pretty(&payload)?;
    std::fs::write(path, format!("{}\n", data))?;
    Ok(())
}

pub fn load_preset_from_file(path: &Path) -> anyhow::Result<(f64, Vec<EqBand>)> {
    let data = std::fs::read_to_string(path)?;
    let payload: serde_json::Value = serde_json::from_str(&data)?;

    // Upstream `json_document_version` treats a missing version as 0 and only
    // rejects presets newer than this build.
    let version = payload.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    if version < 0 {
        anyhow::bail!("preset version must be a non-negative integer");
    }
    if version > PRESET_VERSION as i64 {
        anyhow::bail!(
            "preset version {} is newer than this Mini EQ build",
            version
        );
    }

    let preamp_db = payload
        .get("preamp_db")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let bands_data = payload
        .get("bands")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let fallback = inactive_eq_bands();
    let mut bands = fallback.clone();
    for (i, band_data) in bands_data.iter().take(MAX_BANDS).enumerate() {
        bands[i] = eq_band_from_dict(band_data, &fallback[i]);
    }

    Ok((preamp_db, bands))
}

/// Upstream stores presets under `$XDG_CONFIG_HOME/mini-eq/output`
/// (`default_preset_storage_dir`), so keep the same location to stay
/// interoperable with the Python original.
pub fn preset_storage_dir() -> std::path::PathBuf {
    app_config_dir().join("output")
}

pub fn preset_path_for_name(name: &str) -> std::path::PathBuf {
    let sanitized = sanitize_preset_name(name);
    preset_storage_dir().join(format!("{}{}", sanitized, PRESET_FILE_SUFFIX))
}

/// Sanitize a preset name exactly as upstream `sanitize_preset_name`:
/// invalid characters (``<>:"/\|?*`` and control chars) collapse to a single
/// space, whitespace runs collapse, and leading/trailing spaces and dots are
/// stripped. Preserving this mapping keeps preset file names interchangeable
/// with the Python original.
pub fn sanitize_preset_name(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len());
    let mut last_was_space = false;
    for c in name.chars() {
        let invalid =
            matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20;
        let c = if invalid { ' ' } else { c };
        if c.is_whitespace() {
            if last_was_space {
                continue;
            }
            last_was_space = true;
            cleaned.push(' ');
        } else {
            last_was_space = false;
            cleaned.push(c);
        }
    }
    let cleaned = cleaned.trim();
    cleaned.trim_matches([' ', '.']).chars().take(100).collect()
}

pub fn ensure_preset_storage_dir() -> std::path::PathBuf {
    let dir = preset_storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// List stored preset names, de-duplicated and case-insensitively sorted
/// (upstream `list_preset_names`).
pub fn list_preset_names() -> Vec<String> {
    let dir = ensure_preset_storage_dir();
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_preset = path.is_file()
                && path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    e.eq_ignore_ascii_case(PRESET_FILE_SUFFIX.trim_start_matches('.'))
                });
            if is_preset && let Some(stem) = path.file_stem() {
                let name = stem.to_string_lossy().to_string();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
}

pub fn delete_preset_file(name: &str) -> anyhow::Result<()> {
    let preset_name = sanitize_preset_name(name);
    if preset_name.is_empty() {
        anyhow::bail!("preset name is empty");
    }
    let path = preset_path_for_name(&preset_name);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn output_preset_links_path() -> std::path::PathBuf {
    app_config_file_path(OUTPUT_PRESET_LINKS_FILE)
}

pub fn load_output_preset_config()
-> anyhow::Result<(std::collections::HashMap<String, String>, Option<String>)> {
    load_output_preset_config_at(&output_preset_links_path())
}

/// Path-injectable variant of [`load_output_preset_config`].
pub fn load_output_preset_config_at(
    path: &Path,
) -> anyhow::Result<(std::collections::HashMap<String, String>, Option<String>)> {
    if !path.exists() {
        return Ok((std::collections::HashMap::new(), None));
    }
    let data = std::fs::read_to_string(path)?;
    let payload: serde_json::Value = serde_json::from_str(&data)?;
    let version = payload.get("version").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if version != OUTPUT_PRESET_LINKS_VERSION {
        anyhow::bail!("unsupported output preset links version: {}", version);
    }
    let mut links = std::collections::HashMap::new();
    if let Some(links_obj) = payload.get("links").and_then(|v| v.as_object()) {
        for (key, value) in links_obj {
            if let Some(preset_name) = value.as_str() {
                links.insert(key.clone(), preset_name.to_string());
            }
        }
    }
    let default_preset = payload
        .get("default")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    Ok((links, default_preset.map(|s| s.to_string())))
}

pub fn write_output_preset_config(
    links: &std::collections::HashMap<String, String>,
    default_preset: Option<&str>,
) -> anyhow::Result<()> {
    write_output_preset_config_at(&output_preset_links_path(), links, default_preset)
}

/// Path-injectable variant of [`write_output_preset_config`].
pub fn write_output_preset_config_at(
    path: &Path,
    links: &std::collections::HashMap<String, String>,
    default_preset: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut obj = serde_json::json!({
        "version": OUTPUT_PRESET_LINKS_VERSION,
        "links": links,
    });
    if let Some(default) = default_preset {
        obj.as_object_mut()
            .unwrap()
            .insert("default".to_string(), serde_json::json!(default));
    }
    let data = serde_json::to_string_pretty(&obj)?;
    std::fs::write(path, format!("{}\n", data))?;
    Ok(())
}

pub fn get_output_preset_fallback_name() -> anyhow::Result<Option<String>> {
    let (_links, default) = load_output_preset_config()?;
    Ok(default)
}

pub fn set_output_preset_fallback_name(name: &str) -> anyhow::Result<()> {
    let (links, _default) = load_output_preset_config()?;
    let sanitized = sanitize_preset_name(name);
    if sanitized.is_empty() {
        anyhow::bail!("preset name is empty");
    }
    write_output_preset_config(&links, Some(&sanitized))
}

pub fn clear_output_preset_fallback_name() -> anyhow::Result<bool> {
    let (links, default) = load_output_preset_config()?;
    let had = default.is_some();
    write_output_preset_config(&links, None)?;
    Ok(had)
}

pub fn get_output_preset_link_match(output_keys: &[String]) -> Option<(String, String)> {
    let Ok((links, _)) = load_output_preset_config() else {
        return None;
    };
    for key in output_keys {
        if let Some(preset) = links.get(key) {
            return Some((key.clone(), preset.clone()));
        }
    }
    None
}

pub fn set_output_preset_link(key: &str, preset_name: &str) -> anyhow::Result<()> {
    let (mut links, default) = load_output_preset_config()?;
    links.insert(key.to_string(), preset_name.to_string());
    write_output_preset_config(&links, default.as_deref())
}

pub fn clear_output_preset_link(keys: &[String]) -> anyhow::Result<bool> {
    let (mut links, default) = load_output_preset_config()?;
    let mut removed = false;
    for key in keys {
        if links.remove(key).is_some() {
            removed = true;
        }
    }
    write_output_preset_config(&links, default.as_deref())?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_coefficients() {
        let coeffs = BiquadCoefficients::identity();
        assert!((coeffs.b0 - 1.0).abs() < 1e-12);
        assert!((coeffs.b1).abs() < 1e-12);
        assert!((coeffs.b2).abs() < 1e-12);
        assert!((coeffs.a0 - 1.0).abs() < 1e-12);
        assert!((coeffs.a1).abs() < 1e-12);
        assert!((coeffs.a2).abs() < 1e-12);
    }

    #[test]
    fn test_shelf_biquad_matches_upstream_reference_values() {
        // Reference values produced by the upstream Python
        // `band_biquad_coefficients` (1 kHz, +6 dB, Q=1, 48 kHz), a0-normalised.
        let cases = [
            (
                FilterType::LoShelf,
                [
                    1.0243599982,
                    -1.8785520980,
                    0.8771300926,
                    1.0,
                    -1.8842729798,
                    0.8957692090,
                ],
            ),
            (
                FilterType::HiShelf,
                [
                    1.9478135796,
                    -3.6702124978,
                    1.7447914294,
                    1.0,
                    -1.8338788134,
                    0.8562713246,
                ],
            ),
        ];

        for (filter_type, expected) in cases {
            let mut band = EqBand::new(0);
            band.filter_type = filter_type;
            band.frequency = 1000.0;
            band.gain_db = 6.0;
            band.q = 1.0;
            let c = band_biquad_coefficients(&band, 48000.0, false);
            let actual = [c.b0, c.b1, c.b2, c.a0, c.a1, c.a2];

            for (i, (got, want)) in actual.iter().zip(expected.iter()).enumerate() {
                assert!(
                    (got - want).abs() < 1e-8,
                    "{:?} coeff[{}]: got {} want {}",
                    filter_type,
                    i,
                    got,
                    want
                );
            }
        }
    }

    #[test]
    fn test_biquad_matches_upstream_reference_values() {
        // Bell @ 1 kHz, +6 dB, Q=1, 48 kHz — computed from the upstream Python
        // `band_biquad_coefficients` formula; values are a0-normalised.
        let mut band = EqBand::new(0);
        band.filter_type = FilterType::Bell;
        band.frequency = 1000.0;
        band.gain_db = 6.0;
        band.q = 1.0;
        let c = band_biquad_coefficients(&band, 48000.0, false);

        let a = 10.0_f64.powf(6.0 / 40.0);
        let omega = 2.0 * PI * 1000.0 / 48000.0;
        let cos_w0 = omega.cos();
        let alpha = omega.sin() / 2.0;
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        assert!((c.b0 - b0 / a0).abs() < 1e-12);
        assert!((c.b1 - b1 / a0).abs() < 1e-12);
        assert!((c.b2 - b2 / a0).abs() < 1e-12);
        assert!((c.a0 - 1.0).abs() < 1e-12);
        assert!((c.a1 - a1 / a0).abs() < 1e-12);
        assert!((c.a2 - a2 / a0).abs() < 1e-12);
    }

    #[test]
    fn test_biquad_clamps_frequency_below_nyquist() {
        // Upstream clamps centre to `min(20000, sample_rate/2 - 1)`, so at an
        // 8 kHz sample rate a 20 kHz request must land at 3999 Hz, not 20000.
        let mut band = EqBand::new(0);
        band.filter_type = FilterType::Bell;
        band.frequency = 20000.0;
        band.gain_db = 6.0;
        band.q = 1.0;

        let at_nyquist = band_biquad_coefficients(&band, 8000.0, false);
        band.frequency = 3999.0;
        let at_limit = band_biquad_coefficients(&band, 8000.0, false);
        assert!((at_nyquist.b0 - at_limit.b0).abs() < 1e-12);
        assert!((at_nyquist.a1 - at_limit.a1).abs() < 1e-12);

        // The clamped result must differ from an unclamped 20 kHz evaluation.
        band.frequency = 20000.0;
        let high_rate = band_biquad_coefficients(&band, 48000.0, false);
        assert!((at_nyquist.b0 - high_rate.b0).abs() > 1e-6);
    }

    #[test]
    fn test_biquad_q_floor_matches_upstream() {
        // Upstream uses `max(band.q, 0.0001)`; a Q below the UI minimum must not
        // be raised to EQ_Q_MIN.
        let mut band = EqBand::new(0);
        band.filter_type = FilterType::Bell;
        band.frequency = 1000.0;
        band.gain_db = 6.0;
        band.q = 0.0;

        let floored = band_biquad_coefficients(&band, 48000.0, false);
        band.q = 0.0001;
        let explicit = band_biquad_coefficients(&band, 48000.0, false);
        assert!((floored.b0 - explicit.b0).abs() < 1e-12);

        band.q = EQ_Q_MIN;
        let ui_min = band_biquad_coefficients(&band, 48000.0, false);
        assert!((floored.b0 - ui_min.b0).abs() > 1e-6);
    }

    #[test]
    fn test_default_bands_count() {
        // Upstream `default_eq_bands()` returns all MAX_BANDS bands, with the
        // first DEFAULT_ACTIVE_BANDS enabled.
        let bands = default_bands();
        assert_eq!(bands.len(), MAX_BANDS);
        assert_eq!(
            bands
                .iter()
                .filter(|b| b.filter_type == FilterType::Bell)
                .count(),
            DEFAULT_ACTIVE_BANDS
        );
        assert!(
            bands[..DEFAULT_ACTIVE_BANDS]
                .iter()
                .all(|b| b.filter_type == FilterType::Bell && !b.mute)
        );
        assert!(
            bands[DEFAULT_ACTIVE_BANDS..]
                .iter()
                .all(|b| b.filter_type == FilterType::Off)
        );
    }

    #[test]
    fn test_band_coefficients_bell() {
        let mut band = EqBand::new(0);
        band.frequency = 1000.0;
        band.gain_db = 6.0;
        band.q = 1.0;
        band.filter_type = FilterType::Bell;
        band.mute = false;
        let coeffs = band_biquad_coefficients(&band, SAMPLE_RATE, false);
        assert!(!coeffs.is_identity());
    }

    #[test]
    fn test_filter_type_names() {
        assert_eq!(FilterType::Bell.name(), "Bell");
        assert_eq!(FilterType::HiPass.name(), "Hi-pass");
        assert_eq!(FilterType::LoPass.name(), "Lo-pass");
    }

    /// Pins the combo index mapping to upstream `FILTER_TYPE_INDEX_BY_VALUE`,
    /// which is keyed by filter-type *value* rather than by enum position:
    /// `Resonance` (7) is not selectable, so `Allpass` and `Bandpass` shift down.
    #[test]
    fn test_filter_type_combo_index_matches_upstream() {
        let expected = [
            (FilterType::Off, 0),
            (FilterType::Bell, 1),
            (FilterType::HiPass, 2),
            (FilterType::HiShelf, 3),
            (FilterType::LoPass, 4),
            (FilterType::LoShelf, 5),
            (FilterType::Notch, 6),
            (FilterType::Allpass, 7),
            (FilterType::Bandpass, 8),
        ];
        for (filter_type, index) in expected {
            assert_eq!(filter_type_combo_index(filter_type), index);
            assert_eq!(filter_type_from_combo_index(index), filter_type);
        }
    }

    /// `Resonance` is not selectable upstream, so it must not resolve to a combo
    /// index rather than silently landing on a neighbouring type.
    #[test]
    fn test_unselectable_filter_type_falls_back_to_off() {
        assert_eq!(filter_type_combo_index(FilterType::Resonance), 0);
        assert_eq!(filter_type_from_combo_index(99), FilterType::Off);
    }

    #[test]
    fn test_format_frequency_above_khz() {
        assert_eq!(format_frequency(1000.0), "1.0k");
        assert_eq!(format_frequency(2000.0), "2.0k");
    }

    #[test]
    fn test_format_frequency_below_khz() {
        assert_eq!(format_frequency(500.0), "500");
        assert_eq!(format_frequency(999.9), "1000");
    }

    #[test]
    fn test_total_response_db_neutral() {
        let bands = default_bands();
        let db = total_response_db(&bands, 0.0, SAMPLE_RATE, 1000.0);
        assert!((db - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_total_response_db_with_bell() {
        let mut bands = default_bands();
        bands[0].mute = false;
        bands[0].filter_type = FilterType::Bell;
        bands[0].frequency = 1000.0;
        bands[0].gain_db = 6.0;
        let db = total_response_db(&bands, 0.0, SAMPLE_RATE, 1000.0);
        assert!(db > 3.0);
    }

    #[test]
    fn test_estimate_response_peak_db() {
        let bands = default_bands();
        let peak = estimate_response_peak_db(&bands, 0.0, SAMPLE_RATE);
        assert!((peak - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_estimate_response_peak_db_hits_narrow_band_centre() {
        // A Q=6 bell at 1 kHz is narrower than the 1.02 log sweep, so the peak
        // is only exact when the band centre is unioned in. Upstream returns
        // 12.0 for this input; the log sweep alone yields 11.818651.
        let mut bands = default_bands();
        for band in bands.iter_mut() {
            band.filter_type = FilterType::Off;
        }
        bands[0].filter_type = FilterType::Bell;
        bands[0].frequency = 1000.0;
        bands[0].gain_db = 12.0;
        bands[0].q = 6.0;

        let peak = estimate_response_peak_db(&bands, 0.0, SAMPLE_RATE);
        assert!(
            (peak - 12.0).abs() < 1e-9,
            "expected exact 12.0 dB peak, got {peak}"
        );

        let frequencies = response_peak_frequencies(&bands, SAMPLE_RATE);
        assert!(frequencies.contains(&1000.0));
    }

    #[test]
    fn test_log_response_frequencies() {
        let freqs = log_response_frequencies(SAMPLE_RATE, 1.02);
        assert!(!freqs.is_empty());
        assert_eq!(freqs[0], GRAPH_FREQ_MIN);
    }

    #[test]
    fn test_stepped_response_frequencies() {
        let freqs = stepped_response_frequencies(SAMPLE_RATE, 192);
        assert_eq!(freqs.len(), 192);
        assert_eq!(freqs[0], GRAPH_FREQ_MIN);
    }

    #[test]
    fn test_stepped_response_frequencies_degenerate_falls_back_to_max_frequency() {
        // Upstream returns `max(1.0, max_frequency)`, not `GRAPH_FREQ_MIN`.
        // With a sample rate low enough that Nyquist-1 <= GRAPH_FREQ_MIN, the
        // single returned frequency must track `max_frequency` (here 15.0).
        let freqs = stepped_response_frequencies(32.0, 192);
        assert_eq!(freqs, vec![15.0]);
    }

    #[test]
    fn test_eq_mode_apo_matches_upstream_value() {
        assert_eq!(EQ_MODE_APO, 6);
    }

    #[test]
    fn test_preset_storage_dir_uses_upstream_output_subdir() {
        // Upstream `default_preset_storage_dir` is `app_config_dir() / "output"`,
        // so presets remain interchangeable with the Python original.
        assert_eq!(preset_storage_dir(), app_config_dir().join("output"));
    }

    #[test]
    fn test_load_preset_accepts_missing_and_legacy_version() {
        let dir = std::env::temp_dir().join("mini_eq_preset_version_test");
        let _ = std::fs::create_dir_all(&dir);

        // Version-less payload (implicitly version 0) is accepted, as upstream.
        let path = dir.join("noversion.json");
        std::fs::write(
            &path,
            r#"{"preamp_db": -2.0, "bands": [{"filter_type": 1, "frequency": 1000.0, "gain_db": 3.0, "q": 1.0, "mute": false}]}"#,
        )
        .unwrap();
        let (preamp, bands) = load_preset_from_file(&path).unwrap();
        assert_eq!(preamp, -2.0);
        assert_eq!(bands[0].filter_type, FilterType::Bell);
        assert!(!bands[0].mute);

        // A preset from a newer build is rejected.
        let newer = dir.join("newer.json");
        std::fs::write(&newer, r#"{"version": 99, "bands": []}"#).unwrap();
        assert!(load_preset_from_file(&newer).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sanitize_preset_name_matches_upstream() {
        // Reference values from upstream `sanitize_preset_name`.
        let cases = [
            ("My Preset", "My Preset"),
            ("a/b:c*d?e", "a b c d e"),
            ("  ..Name..  ", "Name"),
            ("foo<bar>|baz", "foo bar baz"),
            ("a\tb\nc", "a b c"),
            ("", ""),
            ("....", ""),
            ("Flat  Bass", "Flat Bass"),
        ];
        for (input, expected) in cases {
            assert_eq!(sanitize_preset_name(input), expected, "input {input:?}");
        }

        let long = "x".repeat(150);
        assert_eq!(sanitize_preset_name(&long).len(), 100);
    }

    #[test]
    fn test_preset_payload_roundtrip() {
        let bands = default_bands();
        let payload = preset_payload(&bands, -3.0);
        let parsed = preset_payload_bands(&payload).unwrap();
        assert_eq!(parsed.len(), bands.len());
        assert!(
            (payload
                .get("preamp_db")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                - (-3.0))
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn test_band_dict_uses_upstream_mute_key() {
        let mut band = EqBand::new(0);
        band.mute = true;
        let dict = eq_band_to_dict(&band);
        assert_eq!(dict.get("mute").and_then(|v| v.as_bool()), Some(true));
        assert!(dict.get("enabled").is_none());

        band.mute = false;
        let dict = eq_band_to_dict(&band);
        assert_eq!(dict.get("mute").and_then(|v| v.as_bool()), Some(false));
    }

    /// A muted band must survive a save/load cycle. Modelling `mute` as the
    /// inverse of a single "enabled" flag lost it, because `Off` bands and
    /// muted bands both serialised to the same value.
    #[test]
    fn test_muted_band_roundtrips_through_preset() {
        let mut bands = default_bands();
        bands[0].filter_type = FilterType::Bell;
        bands[0].mute = true;

        let payload = preset_payload(&bands, 0.0);
        let parsed = preset_payload_bands(&payload).unwrap();
        assert!(parsed[0].mute);
        assert_eq!(parsed[0].filter_type, FilterType::Bell);
    }

    #[test]
    fn test_band_from_dict_reads_upstream_mute_and_rejects_unsupported_types() {
        let fallback = EqBand::new(0);

        // Upstream JSON with `mute: true` must load as disabled.
        let upstream = serde_json::json!({
            "filter_type": 1, "frequency": 1000.0, "gain_db": 3.0,
            "q": 1.0, "mute": true, "solo": false,
        });
        let band = eq_band_from_dict(&upstream, &fallback);
        assert!(band.mute);
        assert_eq!(band.filter_type, FilterType::Bell);

        // Resonance (7) is not a selectable type upstream -> coerced to Off.
        let unsupported = serde_json::json!({
            "filter_type": 7, "frequency": 1000.0, "gain_db": 3.0, "q": 1.0,
        });
        let band = eq_band_from_dict(&unsupported, &fallback);
        assert_eq!(band.filter_type, FilterType::Off);

        // `enabled` from presets written by this port still loads, inverted.
        let legacy = serde_json::json!({ "enabled": true, "filter_type": 1 });
        assert!(!eq_band_from_dict(&legacy, &fallback).mute);
        let legacy = serde_json::json!({ "enabled": false, "filter_type": 1 });
        assert!(eq_band_from_dict(&legacy, &fallback).mute);
    }

    #[test]
    fn test_preset_payload_bands_with_more_than_default_bands() {
        // Upstream presets serialize all MAX_BANDS bands; loading one must not
        // panic even though only DEFAULT_ACTIVE_BANDS are active by default.
        let bands: Vec<EqBand> = (0..MAX_BANDS)
            .map(|index| {
                let mut band = EqBand::new(index);
                band.filter_type = FilterType::Bell;
                band.gain_db = 1.5;
                band
            })
            .collect();
        assert_eq!(bands.len(), MAX_BANDS);

        let payload = preset_payload(&bands, -2.0);
        let parsed = preset_payload_bands(&payload).expect("must parse 32 bands");
        assert_eq!(parsed.len(), MAX_BANDS);
        assert!(parsed.iter().all(|b| b.filter_type == FilterType::Bell));
        assert!(parsed.iter().all(|b| (b.gain_db - 1.5).abs() < 1e-9));
    }

    #[test]
    fn test_default_bands_match_upstream_log_spacing() {
        // Mirrors upstream `compute_log_spaced_band_defaults(DEFAULT_ACTIVE_BANDS)`.
        let bands = default_bands();
        let expected = [
            (29.9526, 1.5048),
            (59.7633, 1.5048),
            (119.2435, 1.5048),
            (237.9221, 1.5048),
            (474.7171, 1.5048),
            (947.1851, 1.5048),
            (1889.8828, 1.5048),
            (3770.8118, 1.5048),
            (7523.7588, 1.5048),
            (15011.8723, 1.5048),
        ];
        for (band, (freq, q)) in bands.iter().zip(expected.iter()) {
            assert!(
                (band.frequency - freq).abs() < 0.01,
                "band {} frequency {} != {}",
                band.index,
                band.frequency,
                freq
            );
            assert!(
                (band.q - q).abs() < 0.001,
                "band {} q {} != {}",
                band.index,
                band.q,
                q
            );
            assert_eq!(band.filter_type, FilterType::Bell);
            assert!(!band.mute);
        }
    }

    #[test]
    fn test_output_preset_config_roundtrip() {
        let path = std::env::temp_dir()
            .join("mini-eq-test")
            .join("output-presets-roundtrip.json");
        let _ = std::fs::remove_file(&path);

        let mut links = std::collections::HashMap::new();
        links.insert("sink1".to_string(), "preset1".to_string());
        write_output_preset_config_at(&path, &links, Some("fallback")).unwrap();

        let (loaded_links, default) = load_output_preset_config_at(&path).unwrap();
        assert_eq!(loaded_links.get("sink1"), Some(&"preset1".to_string()));
        assert_eq!(default, Some("fallback".to_string()));

        // Clearing a link preserves the fallback preset.
        let (mut links, default) = load_output_preset_config_at(&path).unwrap();
        assert!(links.remove("sink1").is_some());
        write_output_preset_config_at(&path, &links, default.as_deref()).unwrap();

        let (loaded_links, default) = load_output_preset_config_at(&path).unwrap();
        assert!(!loaded_links.contains_key("sink1"));
        assert_eq!(default, Some("fallback".to_string()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_output_preset_config_missing_file_is_empty() {
        let path = std::env::temp_dir()
            .join("mini-eq-test")
            .join("does-not-exist-output-presets.json");
        let _ = std::fs::remove_file(&path);
        let (links, default) = load_output_preset_config_at(&path).unwrap();
        assert!(links.is_empty());
        assert!(default.is_none());
    }

    #[test]
    fn test_write_output_preset_config_creates_parent_dir() {
        let dir = std::env::temp_dir()
            .join("mini-eq-test")
            .join("nested")
            .join("deeper");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("output-presets.json");

        write_output_preset_config_at(&path, &std::collections::HashMap::new(), None).unwrap();
        assert!(path.is_file());

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("mini-eq-test").join("nested"));
    }
}
