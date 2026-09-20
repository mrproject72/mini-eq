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
pub const EQ_MODE_APO: i32 = 0;

pub const FILTER_TYPE_INDEX_BY_VALUE: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];
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

    pub fn native_label(&self) -> &str {
        match self {
            Self::Off => "bq_peaking",
            Self::Bell => "bq_peaking",
            Self::HiPass => "bq_highpass",
            Self::HiShelf => "bq_highshelf",
            Self::LoPass => "bq_lowpass",
            Self::LoShelf => "bq_lowshelf",
            Self::Notch => "bq_notch",
            Self::Resonance => "bq_resonance",
            Self::Allpass => "bq_allpass",
            Self::Bandpass => "bq_bandpass",
            Self::LadderPass => "bq_ladder_pass",
            Self::LadderRej => "bq_ladder_reject",
        }
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
    pub enabled: bool,
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
            enabled: false,
            solo: false,
            coefficients: BiquadCoefficients::identity(),
        }
    }

    pub fn is_effective(&self) -> bool {
        self.enabled && self.filter_type != FilterType::Off
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

    let wet = 1.0;

    let f0 = band
        .frequency
        .clamp(EQ_FREQUENCY_MIN_HZ, EQ_FREQUENCY_MAX_HZ);
    let gain = band.gain_db.clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB);
    let q = band.q.clamp(EQ_Q_MIN, EQ_Q_MAX);
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
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + 2.0 * a.sqrt() * alpha);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - 2.0 * a.sqrt() * alpha);
            let a0 = (a + 1.0) + (a - 1.0) * cos_omega + 2.0 * a.sqrt() * alpha;
            let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
            let a2 = (a + 1.0) + (a - 1.0) * cos_omega - 2.0 * a.sqrt() * alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::LoShelf => {
            let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + 2.0 * a.sqrt() * alpha);
            let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
            let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - 2.0 * a.sqrt() * alpha);
            let a0 = (a + 1.0) - (a - 1.0) * cos_omega + 2.0 * a.sqrt() * alpha;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
            let a2 = (a + 1.0) - (a - 1.0) * cos_omega - 2.0 * a.sqrt() * alpha;
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
        b0: b0 / norm * wet + (1.0 - wet),
        b1: b1 / norm * wet,
        b2: b2 / norm * wet,
        a0: 1.0,
        a1: a1 / norm * wet,
        a2: a2 / norm * wet,
    }
}

pub fn band_is_effective(band: &EqBand, solo_active: bool) -> bool {
    band.enabled && band.filter_type != FilterType::Off && (!solo_active || band.solo)
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
        return vec![GRAPH_FREQ_MIN.max(1.0)];
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
        return vec![GRAPH_FREQ_MIN.max(1.0)];
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

pub fn estimate_response_peak_db(bands: &[EqBand], preamp_db: f64, sample_rate: f64) -> f64 {
    let frequencies = log_response_frequencies(sample_rate, RESPONSE_PEAK_F_STEP);
    let responses = total_response_db_at_frequencies(bands, preamp_db, sample_rate, &frequencies);
    responses
        .into_iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(preamp_db)
}

// ── Default Band Presets ─────────────────────────────────────────────────────

pub fn default_bands() -> Vec<EqBand> {
    let frequencies = [
        32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    let mut bands = Vec::new();
    for (i, &freq) in frequencies.iter().enumerate().take(MAX_BANDS) {
        let mut band = EqBand::new(i);
        band.frequency = freq;
        band.gain_db = 0.0;
        band.q = 1.0;
        band.filter_type = FilterType::Off;
        band.enabled = i < DEFAULT_ACTIVE_BANDS;
        band.coefficients = BiquadCoefficients::identity();
        bands.push(band);
    }
    bands
}

pub fn eq_band_to_dict(band: &EqBand) -> serde_json::Value {
    serde_json::json!({
        "filter_type": band.filter_type as u8,
        "frequency": band.frequency,
        "gain_db": band.gain_db,
        "q": band.q,
        "enabled": band.enabled,
        "solo": band.solo,
    })
}

pub fn eq_band_from_dict(data: &serde_json::Value, fallback: &EqBand) -> EqBand {
    let filter_type_val = data
        .get("filter_type")
        .and_then(|v| v.as_u64())
        .unwrap_or(fallback.filter_type as u64) as u8;
    let filter_type = FilterType::from_name(match filter_type_val {
        0 => "Off",
        1 => "Bell",
        2 => "Hi-pass",
        3 => "Hi-shelf",
        4 => "Lo-pass",
        5 => "Lo-shelf",
        6 => "Notch",
        7 => "Resonance",
        8 => "Allpass",
        9 => "Bandpass",
        10 => "Ladder-pass",
        11 => "Ladder-rej",
        _ => "Off",
    })
    .unwrap_or(FilterType::Off);

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
        enabled: data
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(fallback.enabled),
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
    let fallback = default_bands();
    let mut bands = fallback.clone();
    for (i, band_data) in bands_data.iter().take(MAX_BANDS).enumerate() {
        bands[i] = eq_band_from_dict(band_data, &fallback[i]);
    }
    Ok(bands)
}

pub fn preset_payload_state_signature(payload: &serde_json::Value) -> String {
    let mut clone = payload.clone();
    if let Some(obj) = clone.as_object_mut() {
        obj.remove("version");
    }
    serde_json::to_string(&clone).unwrap_or_default()
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

    let version = payload.get("version").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if version != PRESET_VERSION {
        anyhow::bail!("unsupported preset version: {}", version);
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

    let fallback = default_bands();
    let mut bands = fallback.clone();
    for (i, band_data) in bands_data.iter().take(MAX_BANDS).enumerate() {
        bands[i] = eq_band_from_dict(band_data, &fallback[i]);
    }

    Ok((preamp_db, bands))
}

pub fn preset_storage_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".config/mini-eq/presets")
}

pub fn preset_path_for_name(name: &str) -> std::path::PathBuf {
    let sanitized = sanitize_preset_name(name);
    preset_storage_dir().join(format!("{}{}", sanitized, PRESET_FILE_SUFFIX))
}

pub fn sanitize_preset_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

pub fn ensure_preset_storage_dir() -> std::path::PathBuf {
    let dir = preset_storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir
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
    let path = output_preset_links_path();
    if !path.exists() {
        return Ok((std::collections::HashMap::new(), None));
    }
    let data = std::fs::read_to_string(&path)?;
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
    std::fs::write(output_preset_links_path(), format!("{}\n", data))?;
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
    fn test_default_bands_count() {
        let bands = default_bands();
        assert_eq!(bands.len(), 10);
    }

    #[test]
    fn test_band_coefficients_bell() {
        let mut band = EqBand::new(0);
        band.frequency = 1000.0;
        band.gain_db = 6.0;
        band.q = 1.0;
        band.filter_type = FilterType::Bell;
        band.enabled = true;
        let coeffs = band_biquad_coefficients(&band, SAMPLE_RATE, false);
        assert!(!coeffs.is_identity());
    }

    #[test]
    fn test_filter_type_names() {
        assert_eq!(FilterType::Bell.name(), "Bell");
        assert_eq!(FilterType::HiPass.name(), "Hi-pass");
        assert_eq!(FilterType::LoPass.name(), "Lo-pass");
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
        bands[0].enabled = true;
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
    fn test_output_preset_config_roundtrip() {
        let mut links = std::collections::HashMap::new();
        links.insert("sink1".to_string(), "preset1".to_string());
        write_output_preset_config(&links, Some("fallback")).unwrap();
        let (loaded_links, default) = load_output_preset_config().unwrap();
        assert_eq!(loaded_links.get("sink1"), Some(&"preset1".to_string()));
        assert_eq!(default, Some("fallback".to_string()));
        clear_output_preset_link(&["sink1".to_string()]).unwrap();
        let (loaded_links, _) = load_output_preset_config().unwrap();
        assert!(!loaded_links.contains_key("sink1"));
    }
}
