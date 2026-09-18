use std::f64::consts::PI;

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

// ── Filter Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy)]
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

// ── EQ Band ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EqBand {
    pub index: usize,
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
    pub filter_type: FilterType,
    pub enabled: bool,
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
    if !band.is_effective() || !band.filter_type.is_selectable() {
        return BiquadCoefficients::identity();
    }

    let wet =
        if band.enabled && band_is_effective(band, solo_active) && band.filter_type.is_selectable()
        {
            1.0
        } else {
            0.0
        };

    if wet == 0.0 {
        return BiquadCoefficients::identity();
    }

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

pub fn band_is_effective(band: &EqBand, _solo_active: bool) -> bool {
    band.enabled && band.filter_type != FilterType::Off
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
}
