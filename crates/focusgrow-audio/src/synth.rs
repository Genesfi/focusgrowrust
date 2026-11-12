use focusgrow_core::AmbientConfig;
use std::f32::consts::TAU;

pub const SAMPLE_RATE: f32 = 44_100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Default,
    Chime,
    Reminder,
    Alarm,
    CountdownPip(u8),
}

impl AlertKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "default" => Some(Self::Default),
            "chime" => Some(Self::Chime),
            "reminder" => Some(Self::Reminder),
            "alarm" => Some(Self::Alarm),
            _ => None,
        }
    }
}

pub const TRACK_COUNT: usize = 11;

#[derive(Debug, Clone, Copy)]
pub struct TrackMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

pub const AMBIENT_TRACKS: [TrackMeta; TRACK_COUNT] = [
    TrackMeta { id: "typing", name: "Mechanical Typing", desc: "ASMR fluid keystrokes & cadence" },
    TrackMeta { id: "fire", name: "Cozy Campfire", desc: "Crackling dry wood & warm embers" },
    TrackMeta { id: "cricket", name: "Night Crickets", desc: "Peaceful nocturnal crickets & cicadas" },
    TrackMeta { id: "rain_drizzle", name: "Rain on Window", desc: "Soft drizzle bed & delicate drops" },
    TrackMeta { id: "rain_heavy", name: "Heavy Downpour", desc: "Dynamic storm surges & rain" },
    TrackMeta { id: "night", name: "Night Breeze", desc: "Calm nocturnal wind & atmosphere" },
    TrackMeta { id: "clock", name: "Wall Clock Tick", desc: "Rhythmic mechanical wall clock" },
    TrackMeta { id: "cafe", name: "Crowded Cafe", desc: "Warm coffee shop murmurs & chatter" },
    TrackMeta { id: "cafe_jp", name: "Tokyo Cafe", desc: "Authentic Tokyo cafe ambience & chatter" },
    TrackMeta { id: "ocean", name: "Ocean Tide Waves", desc: "Slow cyclic shore wave swells" },
    TrackMeta { id: "alpha", name: "Zen Alpha 432Hz", desc: "Warm harmonic alpha focus pad" },
];

#[derive(Debug, Clone, Copy)]
pub struct PresetMeta {
    pub id: &'static str,
    pub name: &'static str,
}

pub const AMBIENT_PRESETS: [PresetMeta; 8] = [
    PresetMeta { id: "night_coder", name: "Midnight Coder" },
    PresetMeta { id: "campfire_cozy", name: "Cozy Campfire" },
    PresetMeta { id: "rainy_cafe", name: "Soft Rain Cafe" },
    PresetMeta { id: "heavy_downpour", name: "Heavy Downpour" },
    PresetMeta { id: "office_flow", name: "ASMR Typing" },
    PresetMeta { id: "zen_mind", name: "Zen Alpha 432Hz" },
    PresetMeta { id: "ocean_night", name: "Night Ocean" },
    PresetMeta { id: "focus_clock", name: "Steady Clock" },
];

#[derive(Clone, Copy)]
struct Rng64(u64);

impl Rng64 {
    fn new(seed: u64) -> Self {
        Self(seed.max(0x9E37_79B9_7F4A_7C15))
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    #[inline]
    fn next_unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / 16_777_216.0)
    }

    #[inline]
    fn next_bipolar(&mut self) -> f32 {
        self.next_unit() * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
}

/// Web Audio RBJ Biquad Filter port for natural acoustic shaping.
#[derive(Clone, Copy)]
struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    fn new(kind: FilterKind, freq_hz: f32, q: f32) -> Self {
        let mut f = Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        f.set_params(kind, freq_hz, q);
        f
    }

    fn set_params(&mut self, kind: FilterKind, freq_hz: f32, q: f32) {
        let f = freq_hz.clamp(20.0, SAMPLE_RATE * 0.45);
        let w0 = TAU * f / SAMPLE_RATE;
        let sin_w = w0.sin();
        let cos_w = w0.cos();
        let alpha = sin_w / (2.0 * q.max(0.1));
        let a0 = 1.0 + alpha;
        match kind {
            FilterKind::LowPass => {
                self.b0 = ((1.0 - cos_w) * 0.5) / a0;
                self.b1 = (1.0 - cos_w) / a0;
                self.b2 = self.b0;
            }
            FilterKind::HighPass => {
                self.b0 = ((1.0 + cos_w) * 0.5) / a0;
                self.b1 = -(1.0 + cos_w) / a0;
                self.b2 = self.b0;
            }
            FilterKind::BandPass => {
                self.b0 = alpha / a0;
                self.b1 = 0.0;
                self.b2 = -alpha / a0;
            }
        }
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[derive(Clone, Copy, Default)]
struct PinkState {
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
}

impl PinkState {
    #[inline]
    fn step(&mut self, white: f32) -> f32 {
        self.b0 = 0.99886 * self.b0 + white * 0.0555179;
        self.b1 = 0.99332 * self.b1 + white * 0.0750759;
        self.b2 = 0.96900 * self.b2 + white * 0.1538520;
        self.b3 = 0.86650 * self.b3 + white * 0.3104856;
        self.b4 = 0.55000 * self.b4 + white * 0.5329522;
        self.b5 = -0.7616 * self.b5 - white * 0.0168980;
        let out = (self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362) * 0.11;
        self.b6 = white * 0.115926;
        out
    }
}

#[derive(Clone, Copy)]
struct TransientVoice {
    active: bool,
    age: f32,
    max_age: f32,
    decay_rate: f32,
    gain: f32,
    pan_l: f32,
    pan_r: f32,
    filter: BiquadFilter,
}

impl TransientVoice {
    fn new(kind: FilterKind, freq: f32, q: f32) -> Self {
        Self {
            active: false,
            age: 0.0,
            max_age: 0.02,
            decay_rate: 250.0,
            gain: 0.2,
            pan_l: 0.5,
            pan_r: 0.5,
            filter: BiquadFilter::new(kind, freq, q),
        }
    }
}

/// Active sample voice for Mechanical Keystrokes & Tactile Mouse Microswitch clicks.
#[derive(Clone, Copy)]
struct KeySampleVoice {
    active: bool,
    is_mouse: bool,
    age_sec: f32,
    max_sec: f32,
    decay: f32,
    gain: f32,
    prev_w: f32,
    smooth_w: f32,
    filter: BiquadFilter,
    delay_samples: u32,
}

impl KeySampleVoice {
    fn empty() -> Self {
        Self {
            active: false,
            is_mouse: false,
            age_sec: 0.0,
            max_sec: 0.024,
            decay: 190.0,
            gain: 0.7,
            prev_w: 0.0,
            smooth_w: 0.0,
            filter: BiquadFilter::new(FilterKind::LowPass, 1450.0, 0.7),
            delay_samples: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct CricketOsc {
    phase: f32,
    base_freq: f32,
    cur_freq: f32,
    pan_l: f32,
    pan_r: f32,
    gain: f32,
    state: u8, // 0=idle, 1=syllable, 2=gap
    syllable_count: u8,
    target_syllables: u8,
    time_in_state: f32,
    next_delay: f32,
}

#[derive(Clone, Copy)]
struct AlertTone {
    freq_hz: f32,
    delay_samples: u32,
    elapsed_samples: u32,
    dur_samples: u32,
    amp: f32,
    harmonic: f32,
}

pub struct SynthBank {
    rng: Rng64,

    // Mechanical Keyboard + Mouse Microswitch (matches WebView2 createMechanicalKeyBuffer + createMouseClickBuffer)
    key_voices: [KeySampleVoice; 8],
    typing_wait_samples: u32,
    typing_burst_left: u8,
    pending_space_or_mouse: bool,

    // Cozy Campfire (Authentic Dry Wood & Embers DSP)
    fire_pink_l: PinkState,
    fire_pink_r: PinkState,
    fire_draft_lp_l: BiquadFilter,
    fire_draft_lp_r: BiquadFilter,
    fire_sizzle_bp_l: BiquadFilter,
    fire_sizzle_bp_r: BiquadFilter,
    fire_thermal_phase1: f32,
    fire_thermal_phase2: f32,
    fire_pops: [TransientVoice; 16],
    fire_pop_timer: u32,
    fire_next_pop: u32,
    fire_micro_timer: u32,
    fire_next_micro: u32,

    // Night Crickets (NaturalCricketDSP)
    cricket_pink: PinkState,
    cricket_air_lp: BiquadFilter,
    cricket_1: CricketOsc,
    cricket_2: CricketOsc,

    // Light Drizzle Rain (NaturalLightRainDSP)
    drizzle_pink_l: PinkState,
    drizzle_pink_r: PinkState,
    drizzle_lp_l: BiquadFilter,
    drizzle_lp_r: BiquadFilter,
    drizzle_hp_l: BiquadFilter,
    drizzle_hp_r: BiquadFilter,
    drizzle_warm_l: BiquadFilter,
    drizzle_warm_r: BiquadFilter,
    drizzle_lfo: f32,
    drizzle_drops: [TransientVoice; 14],
    drizzle_drop_timer: u32,
    drizzle_next_drop: u32,

    // Heavy Downpour (NaturalHeavyRainDSP)
    heavy_pink_l: PinkState,
    heavy_pink_r: PinkState,
    heavy_brown_l: f32,
    heavy_brown_r: f32,
    heavy_rumble_l: BiquadFilter,
    heavy_rumble_r: BiquadFilter,
    heavy_mid_l: BiquadFilter,
    heavy_mid_r: BiquadFilter,
    heavy_spray_lp_l: BiquadFilter,
    heavy_spray_lp_r: BiquadFilter,
    heavy_spray_hp_l: BiquadFilter,
    heavy_spray_hp_r: BiquadFilter,
    heavy_lfo: [f32; 3],
    heavy_splatters: [TransientVoice; 16],
    heavy_splat_timer: u32,
    heavy_next_splat: u32,

    // Night Breeze (lowpass 260Hz brown + 0.06Hz LFO)
    night_brown: f32,
    night_lp: BiquadFilter,
    night_lfo: f32,

    // Wall Clock Tick/Tock (room acoustic stereo IR buffer matching getClockTickBuffer)
    clock_tick_ir: Vec<(f32, f32)>,
    clock_tock_ir: Vec<(f32, f32)>,
    clock_sample: u32,
    clock_is_tock: bool,
    clock_play_pos: usize,

    // Ocean Tide Waves
    ocean_pink: PinkState,
    ocean_brown: f32,
    ocean_lp: BiquadFilter,
    ocean_lfo: f32,

    // Zen Alpha 432Hz
    alpha_phase_l: f32,
    alpha_phase_r: f32,
    alpha_lfo: f32,

    // Crowded Cafe Ambience (CC0 Archival Recording PCM Stream)
    cafe_samples: Vec<f32>,
    cafe_pos: f32,

    // Tokyo Cafe Ambience (CC-BY Freesound Tokyo Japanese Restaurant PCM Stream)
    cafe_jp_samples: Vec<f32>,
    cafe_jp_pos: f32,

    // Mixer gains (smoothed)
    track_targets: [f32; TRACK_COUNT],
    track_gains: [f32; TRACK_COUNT],
    master_target: f32,
    master_gain: f32,
    playing: bool,

    // Polyphonic alert voices
    alerts: Vec<AlertTone>,
}

static CAFE_WAV_BYTES: &[u8] = include_bytes!("../assets/cafe_crowd.wav");
static CAFE_JP_WAV_BYTES: &[u8] = include_bytes!("../assets/cafe_jp.wav");

fn decode_wav_pcm16_mono(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Vec::new();
    }
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let chunk_id = &bytes[i..i + 4];
        let chunk_len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap_or([0; 4])) as usize;
        let data_start = i + 8;
        let data_end = (data_start + chunk_len).min(bytes.len());
        if chunk_id == b"data" {
            let sample_bytes = &bytes[data_start..data_end];
            let mut samples = Vec::with_capacity(sample_bytes.len() / 2);
            for chunk in sample_bytes.chunks_exact(2) {
                let s = i16::from_le_bytes([chunk[0], chunk[1]]);
                samples.push(s as f32 / 32768.0);
            }
            return samples;
        }
        i = data_end;
        if i % 2 != 0 {
            i += 1;
        }
    }
    Vec::new()
}

fn build_clock_ir(rng: &mut Rng64, is_tock: bool) -> Vec<(f32, f32)> {
    // Real wall clock escapement: dry, crisp mechanical tooth drop + subtle wood/plastic casing resonance.
    // Total duration ~45ms, intimate and calm without cavernous echo.
    let len = (SAMPLE_RATE * 0.045) as usize;
    let mut out = vec![(0.0f32, 0.0f32); len];

    // Primary pallet strike:
    // Tick: crisp escapement release (~2850 Hz click + 1050 Hz body)
    // Tock: drop onto right pallet (~2250 Hz click + 820 Hz body)
    let (click_f, body_f, body_q) = if is_tock {
        (2250.0, 820.0, 3.2)
    } else {
        (2850.0, 1050.0, 3.6)
    };

    let mut click_bp = BiquadFilter::new(FilterKind::BandPass, click_f, 1.8);
    let mut body_bp = BiquadFilter::new(FilterKind::BandPass, body_f, body_q);

    // Subtle recoil tap delay (~11ms later, escapement tooth settling)
    let recoil_delay = (SAMPLE_RATE * 0.011) as usize;
    let mut recoil_bp = BiquadFilter::new(FilterKind::BandPass, click_f * 0.85, 2.0);

    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE;

        // Primary strike impulse (sharp 1.8ms burst)
        let primary_impulse = if t < 0.0018 {
            rng.next_bipolar() * (-t * 3200.0).exp()
        } else {
            0.0
        };

        let click = click_bp.process(primary_impulse) * 0.65;
        let body = body_bp.process(primary_impulse) * 0.35;

        // Recoil settling impulse
        let recoil_impulse = if i >= recoil_delay && (i - recoil_delay) < (SAMPLE_RATE * 0.0012) as usize {
            let tr = (i - recoil_delay) as f32 / SAMPLE_RATE;
            rng.next_bipolar() * (-tr * 3600.0).exp() * 0.24
        } else {
            0.0
        };
        let recoil = recoil_bp.process(recoil_impulse);

        let dry = click + body + recoil;

        // Intimate near-field stereo spread
        let pan = if is_tock { 0.47 } else { 0.53 };
        out[i].0 = dry * (1.0 - pan);
        out[i].1 = dry * pan;
    }

    // Subtle wall boundary reflection (4.2ms later, gentle and damped)
    let wall_delay = (SAMPLE_RATE * 0.0042) as usize;
    for i in 0..(len - wall_delay) {
        out[i + wall_delay].0 += out[i].0 * 0.12;
        out[i + wall_delay].1 += out[i].1 * 0.12;
    }

    // Calibrated gentle level (calm, warm, and comforting; not loud or piercing)
    let max_amp = out.iter().map(|(l, r)| l.abs().max(r.abs())).fold(0.0f32, f32::max);
    if max_amp > 1e-6 {
        let scale = 0.38 / max_amp;
        for (l, r) in &mut out {
            *l *= scale;
            *r *= scale;
        }
    }
    out
}

impl SynthBank {
    pub fn new(config: &AmbientConfig) -> Self {
        let mut rng = Rng64::new(0xF0C5_6A0E_1234_5678);
        let clock_tick_ir = build_clock_ir(&mut rng, false);
        let clock_tock_ir = build_clock_ir(&mut rng, true);

        let mut bank = Self {
            rng,
            key_voices: [KeySampleVoice::empty(); 8],
            typing_wait_samples: 800,
            typing_burst_left: 6,
            pending_space_or_mouse: false,
            fire_pink_l: PinkState::default(),
            fire_pink_r: PinkState::default(),
            fire_draft_lp_l: BiquadFilter::new(FilterKind::LowPass, 220.0, 0.707),
            fire_draft_lp_r: BiquadFilter::new(FilterKind::LowPass, 220.0, 0.707),
            fire_sizzle_bp_l: BiquadFilter::new(FilterKind::BandPass, 3400.0, 0.6),
            fire_sizzle_bp_r: BiquadFilter::new(FilterKind::BandPass, 3400.0, 0.6),
            fire_thermal_phase1: 0.4,
            fire_thermal_phase2: 1.7,
            fire_pops: [TransientVoice::new(FilterKind::BandPass, 2800.0, 1.8); 16],
            fire_pop_timer: 0,
            fire_next_pop: 2400,
            fire_micro_timer: 0,
            fire_next_micro: 400,
            cricket_pink: PinkState::default(),
            cricket_air_lp: BiquadFilter::new(FilterKind::LowPass, 220.0, 0.707),
            cricket_1: CricketOsc {
                phase: 0.0,
                base_freq: 4520.0,
                cur_freq: 4520.0,
                pan_l: 0.65,
                pan_r: 0.35,
                gain: 0.24,
                state: 0,
                syllable_count: 0,
                target_syllables: 3,
                time_in_state: 0.0,
                next_delay: 0.4,
            },
            cricket_2: CricketOsc {
                phase: 0.0,
                base_freq: 4740.0,
                cur_freq: 4740.0,
                pan_l: 0.30,
                pan_r: 0.70,
                gain: 0.17,
                state: 0,
                syllable_count: 0,
                target_syllables: 2,
                time_in_state: 0.0,
                next_delay: 1.4,
            },
            drizzle_pink_l: PinkState::default(),
            drizzle_pink_r: PinkState::default(),
            drizzle_lp_l: BiquadFilter::new(FilterKind::LowPass, 2400.0, 0.707),
            drizzle_lp_r: BiquadFilter::new(FilterKind::LowPass, 2400.0, 0.707),
            drizzle_hp_l: BiquadFilter::new(FilterKind::HighPass, 180.0, 0.707),
            drizzle_hp_r: BiquadFilter::new(FilterKind::HighPass, 180.0, 0.707),
            drizzle_warm_l: BiquadFilter::new(FilterKind::LowPass, 3600.0, 0.5),
            drizzle_warm_r: BiquadFilter::new(FilterKind::LowPass, 3600.0, 0.5),
            drizzle_lfo: 0.0,
            drizzle_drops: [TransientVoice::new(FilterKind::BandPass, 3200.0, 0.9); 14],
            drizzle_drop_timer: 0,
            drizzle_next_drop: 2400,
            heavy_pink_l: PinkState::default(),
            heavy_pink_r: PinkState::default(),
            heavy_brown_l: 0.0,
            heavy_brown_r: 0.0,
            heavy_rumble_l: BiquadFilter::new(FilterKind::LowPass, 200.0, 0.707),
            heavy_rumble_r: BiquadFilter::new(FilterKind::LowPass, 200.0, 0.707),
            heavy_mid_l: BiquadFilter::new(FilterKind::BandPass, 1250.0, 0.6),
            heavy_mid_r: BiquadFilter::new(FilterKind::BandPass, 1250.0, 0.6),
            heavy_spray_lp_l: BiquadFilter::new(FilterKind::LowPass, 3800.0, 0.6),
            heavy_spray_lp_r: BiquadFilter::new(FilterKind::LowPass, 3800.0, 0.6),
            heavy_spray_hp_l: BiquadFilter::new(FilterKind::HighPass, 2000.0, 0.6),
            heavy_spray_hp_r: BiquadFilter::new(FilterKind::HighPass, 2000.0, 0.6),
            heavy_lfo: [0.3, 1.4, 2.8],
            heavy_splatters: [TransientVoice::new(FilterKind::BandPass, 1500.0, 0.8); 16],
            heavy_splat_timer: 0,
            heavy_next_splat: 3200,
            night_brown: 0.0,
            night_lp: BiquadFilter::new(FilterKind::LowPass, 260.0, 0.707),
            night_lfo: 0.0,
            clock_tick_ir,
            clock_tock_ir,
            clock_sample: 0,
            clock_is_tock: false,
            clock_play_pos: usize::MAX,
            ocean_pink: PinkState::default(),
            ocean_brown: 0.0,
            ocean_lp: BiquadFilter::new(FilterKind::LowPass, 520.0, 0.6),
            ocean_lfo: 0.0,
            alpha_phase_l: 0.0,
            alpha_phase_r: 0.0,
            alpha_lfo: 0.0,
            cafe_samples: decode_wav_pcm16_mono(CAFE_WAV_BYTES),
            cafe_pos: 0.0,
            cafe_jp_samples: decode_wav_pcm16_mono(CAFE_JP_WAV_BYTES),
            cafe_jp_pos: 0.0,
            track_targets: [0.0; TRACK_COUNT],
            track_gains: [0.0; TRACK_COUNT],
            master_target: 0.8,
            master_gain: 0.0,
            playing: false,
            alerts: Vec::with_capacity(16),
        };
        bank.apply_config(config, false, false);
        bank
    }

    pub fn apply_config(&mut self, cfg: &AmbientConfig, playing: bool, master_muted: bool) {
        self.playing = playing;
        let master = if !playing || master_muted || !cfg.enabled {
            0.0
        } else {
            (cfg.master_volume as f32 / 100.0).clamp(0.0, 1.0)
        };
        self.master_target = master;
        for (i, meta) in AMBIENT_TRACKS.iter().enumerate() {
            let t = cfg.track(meta.id);
            self.track_targets[i] = if t.active {
                (t.volume as f32 / 100.0).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
    }

    pub fn trigger_alert(&mut self, kind: AlertKind) {
        let mut push = |freq_hz: f32, delay_ms: f32, dur_ms: f32, amp: f32, harmonic: f32| {
            if self.alerts.len() >= 24 {
                self.alerts.remove(0);
            }
            self.alerts.push(AlertTone {
                freq_hz,
                delay_samples: ((delay_ms / 1000.0) * SAMPLE_RATE) as u32,
                elapsed_samples: 0,
                dur_samples: (((dur_ms / 1000.0) * SAMPLE_RATE) as u32).max(1),
                amp,
                harmonic,
            });
        };

        match kind {
            AlertKind::Default => {
                push(1046.50, 0.0, 420.0, 0.24, 0.25);
                push(1318.51, 130.0, 620.0, 0.28, 0.20);
            }
            AlertKind::Chime => {
                push(440.00, 0.0, 750.0, 0.16, 0.28);
                push(554.37, 90.0, 750.0, 0.16, 0.24);
                push(659.25, 180.0, 800.0, 0.18, 0.20);
                push(880.00, 270.0, 950.0, 0.20, 0.18);
            }
            AlertKind::Reminder => {
                push(783.99, 0.0, 280.0, 0.24, 0.30);
                push(987.77, 170.0, 420.0, 0.24, 0.22);
            }
            AlertKind::Alarm => {
                push(880.0, 0.0, 220.0, 0.28, 0.35);
                push(1174.66, 200.0, 220.0, 0.28, 0.35);
                push(880.0, 440.0, 220.0, 0.28, 0.35);
                push(1174.66, 640.0, 420.0, 0.30, 0.30);
            }
            AlertKind::CountdownPip(step) => {
                let freq = match step {
                    3 => 440.0,
                    2 => 523.25,
                    1 => 659.25,
                    _ => 880.0,
                };
                push(freq, 0.0, 140.0, 0.18, 0.12);
            }
        }
    }

    pub fn has_active_alerts(&self) -> bool {
        !self.alerts.is_empty()
    }

    pub fn is_idle(&self) -> bool {
        !self.playing && self.master_gain < 0.0005 && self.alerts.is_empty()
    }

    pub fn render_stereo(&mut self, interleaved: &mut [f32]) {
        let frames = interleaved.len() / 2;
        for f in 0..frames {
            self.master_gain += (self.master_target - self.master_gain) * 0.0015;
            for i in 0..TRACK_COUNT {
                self.track_gains[i] += (self.track_targets[i] - self.track_gains[i]) * 0.0015;
            }

            let mut mix_l = 0.0f32;
            let mut mix_r = 0.0f32;

            if self.master_gain > 0.0005 {
                let g0 = self.track_gains[0];
                let g1 = self.track_gains[1];
                let g2 = self.track_gains[2];
                let g3 = self.track_gains[3];
                let g4 = self.track_gains[4];
                let g5 = self.track_gains[5];
                let g6 = self.track_gains[6];
                let g7 = self.track_gains[7];
                let g8 = self.track_gains[8];
                let g9 = self.track_gains[9];
                let g10 = self.track_gains[10];

                if g0 > 0.001 {
                    let (l, r) = self.step_typing();
                    mix_l += l * g0;
                    mix_r += r * g0;
                }
                if g1 > 0.001 {
                    let (l, r) = self.step_fire();
                    mix_l += l * g1;
                    mix_r += r * g1;
                }
                if g2 > 0.001 {
                    let (l, r) = self.step_cricket();
                    mix_l += l * g2;
                    mix_r += r * g2;
                }
                if g3 > 0.001 {
                    let (l, r) = self.step_rain_drizzle();
                    mix_l += l * g3;
                    mix_r += r * g3;
                }
                if g4 > 0.001 {
                    let (l, r) = self.step_rain_heavy();
                    mix_l += l * g4;
                    mix_r += r * g4;
                }
                if g5 > 0.001 {
                    let (l, r) = self.step_night_breeze();
                    mix_l += l * g5;
                    mix_r += r * g5;
                }
                if g6 > 0.001 {
                    let (l, r) = self.step_clock();
                    mix_l += l * g6;
                    mix_r += r * g6;
                }
                if g7 > 0.001 {
                    let (l, r) = self.step_cafe();
                    mix_l += l * g7;
                    mix_r += r * g7;
                }
                if g8 > 0.001 {
                    let (l, r) = self.step_cafe_jp();
                    mix_l += l * g8;
                    mix_r += r * g8;
                }
                if g9 > 0.001 {
                    let (l, r) = self.step_ocean();
                    mix_l += l * g9;
                    mix_r += r * g9;
                }
                if g10 > 0.001 {
                    let (l, r) = self.step_alpha();
                    mix_l += l * g10;
                    mix_r += r * g10;
                }

                mix_l *= self.master_gain;
                mix_r *= self.master_gain;
            }

            if !self.alerts.is_empty() {
                let alert = self.step_alerts();
                mix_l += alert;
                mix_r += alert;
            }

            interleaved[f * 2] = (mix_l * 0.85).tanh();
            interleaved[f * 2 + 1] = (mix_r * 0.85).tanh();
        }
    }

    fn spawn_key_voice(&mut self, is_space: bool) {
        if let Some(v) = self.key_voices.iter_mut().find(|v| !v.active) {
            v.active = true;
            v.is_mouse = false;
            v.age_sec = 0.0;
            v.max_sec = if is_space { 0.038 } else { 0.024 };
            v.decay = if is_space { 120.0 } else { 190.0 };
            v.gain = (0.5 + self.rng.next_unit() * 0.3) * if is_space { 1.2 } else { 1.0 };
            v.prev_w = 0.0;
            v.smooth_w = 0.0;
            v.delay_samples = 0;
            let lp_freq = if is_space { 1100.0 } else { 1450.0 + self.rng.next_unit() * 250.0 };
            v.filter = BiquadFilter::new(FilterKind::LowPass, lp_freq, 0.7);
        }
    }

    fn spawn_mouse_click(&mut self, is_double: bool) {
        let mut add = |is_up: bool, delay_ms: f32| {
            if let Some(v) = self.key_voices.iter_mut().find(|v| !v.active) {
                v.active = true;
                v.is_mouse = true;
                v.age_sec = 0.0;
                v.max_sec = if is_up { 0.012 } else { 0.018 };
                v.decay = if is_up { 750.0 } else { 480.0 };
                v.gain = 0.42 + self.rng.next_unit() * 0.16;
                v.prev_w = 0.0;
                v.smooth_w = 0.0;
                v.delay_samples = ((delay_ms / 1000.0) * SAMPLE_RATE) as u32;
                let f = 2350.0 + self.rng.next_unit() * 300.0;
                v.filter = BiquadFilter::new(FilterKind::BandPass, f, 1.4);
            }
        };
        add(false, 0.0);
        add(true, 22.0);
        if is_double {
            add(false, 95.0);
            add(true, 117.0);
        }
    }

    /// Mechanical Keyboard + Omron Mouse Switch matching WebView2's `AmbientAudioEngine.startTypingTrack`.
    #[inline]
    fn step_typing(&mut self) -> (f32, f32) {
        if self.typing_wait_samples == 0 {
            if self.typing_burst_left > 0 {
                let is_space = self.typing_burst_left == 1 || self.rng.next_unit() < 0.18;
                self.spawn_key_voice(is_space);
                self.typing_burst_left -= 1;
                // Human cadence: 70ms - 150ms per keystroke
                let ms = 70.0 + self.rng.next_unit() * 80.0;
                self.typing_wait_samples = ((ms / 1000.0) * SAMPLE_RATE) as u32;
                self.pending_space_or_mouse = self.typing_burst_left == 0 && self.rng.next_unit() < 0.35;
            } else if self.pending_space_or_mouse {
                self.pending_space_or_mouse = false;
                let dbl = self.rng.next_unit() < 0.30;
                self.spawn_mouse_click(dbl);
                let pause_ms = if self.rng.next_unit() < 0.3 {
                    1100.0 + self.rng.next_unit() * 1200.0
                } else {
                    380.0 + self.rng.next_unit() * 600.0
                };
                self.typing_wait_samples = ((pause_ms / 1000.0) * SAMPLE_RATE) as u32;
                self.typing_burst_left = 3 + (self.rng.next_unit() * 9.0) as u8;
            } else if self.rng.next_unit() < 0.25 {
                let dbl = self.rng.next_unit() < 0.35;
                self.spawn_mouse_click(dbl);
                let delay_ms = 500.0 + self.rng.next_unit() * 900.0;
                self.typing_wait_samples = ((delay_ms / 1000.0) * SAMPLE_RATE) as u32;
                self.typing_burst_left = 3 + (self.rng.next_unit() * 9.0) as u8;
            } else {
                self.typing_burst_left = 3 + (self.rng.next_unit() * 9.0) as u8;
                self.typing_wait_samples = 1;
            }
        } else {
            self.typing_wait_samples -= 1;
        }

        let dt = 1.0 / SAMPLE_RATE;
        let mut sum = 0.0f32;
        for v in &mut self.key_voices {
            if !v.active {
                continue;
            }
            if v.delay_samples > 0 {
                v.delay_samples -= 1;
                continue;
            }
            let white = self.rng.next_bipolar();
            let raw = if v.is_mouse {
                let click = (white - v.prev_w) * (-v.age_sec * v.decay).exp() * 1.2;
                v.prev_w = white;
                click * 0.7
            } else {
                let attack = (v.age_sec / 0.0022).min(1.0);
                let env = attack * (-v.age_sec * v.decay).exp();
                v.smooth_w = v.smooth_w * 0.7 + white * 0.3;
                let tactile = v.smooth_w * env * 1.6;
                let tap = (white - v.prev_w) * (-v.age_sec * 900.0).exp() * 0.85;
                v.prev_w = white;
                (tactile * 0.9 + tap * 0.35) * (0.85 + self.rng.next_unit() * 0.25)
            };
            sum += v.filter.process(raw) * v.gain * 0.65;
            v.age_sec += dt;
            if v.age_sec >= v.max_sec {
                v.active = false;
            }
        }
        (sum * 0.96, sum * 1.04)
    }

    /// Authentic Dry Wood Campfire & Glowing Embers DSP (rich natural snaps, resin pops & micro-crackles; no periodic drafts).
    #[inline]
    fn step_fire(&mut self) -> (f32, f32) {
        let dt = 1.0 / SAMPLE_RATE;

        // Incommensurate slow chaotic thermal modulation (0.11 Hz and 0.27 Hz) -> zero periodic looping
        self.fire_thermal_phase1 = (self.fire_thermal_phase1 + (TAU * 0.11) / SAMPLE_RATE) % TAU;
        self.fire_thermal_phase2 = (self.fire_thermal_phase2 + (TAU * 0.27) / SAMPLE_RATE) % TAU;
        let heat_shimmer = 0.88 + 0.08 * self.fire_thermal_phase1.sin() + 0.04 * self.fire_thermal_phase2.sin();

        let n_l = self.fire_pink_l.step(self.rng.next_bipolar());
        let n_r = self.fire_pink_r.step(self.rng.next_bipolar());

        // 1. Subtle, gentle warm ember air: gentle ~220Hz low-pass (low volume 0.05, NOT a roaring 150Hz train draft!)
        let warm_l = self.fire_draft_lp_l.process(n_l) * 0.05 * heat_shimmer;
        let warm_r = self.fire_draft_lp_r.process(n_r) * 0.05 * heat_shimmer;

        // 2. High-frequency hot coal / resin sizzle around ~3400 Hz
        let sizzle_l = self.fire_sizzle_bp_l.process(n_l) * 0.10 * heat_shimmer;
        let sizzle_r = self.fire_sizzle_bp_r.process(n_r) * 0.10 * heat_shimmer;

        // 3. Dense micro-crackle granular sparks (continuous delicate snapping of dry wood fibers)
        self.fire_micro_timer += 1;
        if self.fire_micro_timer >= self.fire_next_micro {
            self.fire_micro_timer = 0;
            if let Some(p) = self.fire_pops.iter_mut().find(|v| !v.active) {
                p.active = true;
                p.age = 0.0;
                p.max_age = 0.003 + self.rng.next_unit() * 0.005;
                p.decay_rate = 600.0 + self.rng.next_unit() * 500.0;
                p.gain = 0.04 + self.rng.next_unit() * 0.08;
                let f = 3000.0 + self.rng.next_unit() * 3200.0;
                p.filter.set_params(FilterKind::BandPass, f, 2.2);
                let pan = self.rng.next_unit() * 1.6 - 0.8;
                p.pan_l = (1.0 - pan) * 0.5;
                p.pan_r = (1.0 + pan) * 0.5;
            }
            // Fire micro crackles ~40 to 120 times per second
            let micro_sec = 0.008 + self.rng.next_unit() * 0.022;
            self.fire_next_micro = (SAMPLE_RATE * micro_sec) as u32;
        }

        // 4. Natural wood pops & resin pocket snaps
        self.fire_pop_timer += 1;
        if self.fire_pop_timer >= self.fire_next_pop {
            self.fire_pop_timer = 0;
            // Burst of 1-3 crackles
            let burst = if self.rng.next_unit() < 0.45 {
                2 + (self.rng.next_unit() * 2.0) as usize
            } else {
                1
            };
            for _ in 0..burst {
                if let Some(p) = self.fire_pops.iter_mut().find(|v| !v.active) {
                    p.active = true;
                    p.age = 0.0;
                    let roll = self.rng.next_unit();
                    if roll < 0.70 {
                        // Crisp dry pine wood snap
                        p.max_age = 0.008 + self.rng.next_unit() * 0.016;
                        p.decay_rate = 320.0 + self.rng.next_unit() * 350.0;
                        p.gain = 0.22 + self.rng.next_unit() * 0.28;
                        let f = 2000.0 + self.rng.next_unit() * 2600.0;
                        p.filter.set_params(FilterKind::BandPass, f, 1.8);
                    } else if roll < 0.85 {
                        // High-pitched bright spark pop
                        p.max_age = 0.004 + self.rng.next_unit() * 0.008;
                        p.decay_rate = 650.0 + self.rng.next_unit() * 600.0;
                        p.gain = 0.15 + self.rng.next_unit() * 0.20;
                        let f = 4500.0 + self.rng.next_unit() * 2500.0;
                        p.filter.set_params(FilterKind::BandPass, f, 2.5);
                    } else {
                        // Hollow log ember thump
                        p.max_age = 0.022 + self.rng.next_unit() * 0.025;
                        p.decay_rate = 180.0 + self.rng.next_unit() * 120.0;
                        p.gain = 0.18 + self.rng.next_unit() * 0.22;
                        let f = 500.0 + self.rng.next_unit() * 450.0;
                        p.filter.set_params(FilterKind::BandPass, f, 2.0);
                    }
                    let pan = self.rng.next_unit() * 1.6 - 0.8;
                    p.pan_l = (1.0 - pan) * 0.5;
                    p.pan_r = (1.0 + pan) * 0.5;
                }
            }
            // Natural pause between wood snaps (0.05s to 0.65s) -> keeps wood continuously alive
            let pause_sec = if self.rng.next_unit() < 0.65 {
                0.05 + self.rng.next_unit() * 0.25
            } else {
                0.25 + self.rng.next_unit() * 0.40
            };
            self.fire_next_pop = (SAMPLE_RATE * pause_sec) as u32;
        }

        let mut pop_l = 0.0f32;
        let mut pop_r = 0.0f32;
        for pop in &mut self.fire_pops {
            if !pop.active {
                continue;
            }
            let white = self.rng.next_bipolar();
            let env = (-pop.decay_rate * pop.age).exp();
            let snap = pop.filter.process(white) * env * pop.gain;
            pop_l += snap * pop.pan_l;
            pop_r += snap * pop.pan_r;
            pop.age += dt;
            if pop.age >= pop.max_age {
                pop.active = false;
            }
        }

        ((warm_l + sizzle_l + pop_l) * 0.95, (warm_r + sizzle_r + pop_r) * 0.95)
    }

    fn step_one_cricket(c: &mut CricketOsc, rng: &mut Rng64, dt: f32) -> f32 {
        c.time_in_state += dt;
        if c.state == 0 {
            if c.time_in_state >= c.next_delay {
                c.state = 1;
                c.time_in_state = 0.0;
                c.syllable_count = 0;
                c.target_syllables = 2 + (rng.next_unit() * 2.0) as u8;
                c.cur_freq = c.base_freq + (rng.next_unit() * 60.0 - 30.0);
            }
            return 0.0;
        }
        let syllable_dur = 0.022f32;
        let gap_dur = 0.024f32;
        if c.state == 1 {
            if c.time_in_state >= syllable_dur {
                c.syllable_count += 1;
                if c.syllable_count >= c.target_syllables {
                    c.state = 0;
                    c.time_in_state = 0.0;
                    c.next_delay = 1.5 + rng.next_unit() * 2.3;
                } else {
                    c.state = 2;
                    c.time_in_state = 0.0;
                }
                return 0.0;
            }
            c.phase = (c.phase + TAU * c.cur_freq * dt) % TAU;
            let env = 0.5 * (1.0 - ((TAU * c.time_in_state) / syllable_dur).cos());
            return c.phase.sin() * env * c.gain;
        }
        if c.time_in_state >= gap_dur {
            c.state = 1;
            c.time_in_state = 0.0;
        }
        0.0
    }

    /// Natural Field Crickets matching WebView2's `NaturalCricketDSP`.
    #[inline]
    fn step_cricket(&mut self) -> (f32, f32) {
        let dt = 1.0 / SAMPLE_RATE;
        let n = self.cricket_pink.step(self.rng.next_bipolar());
        let air = self.cricket_air_lp.process(n) * 0.25;
        let s1 = Self::step_one_cricket(&mut self.cricket_1, &mut self.rng, dt);
        let s2 = Self::step_one_cricket(&mut self.cricket_2, &mut self.rng, dt);
        (
            air + s1 * self.cricket_1.pan_l + s2 * self.cricket_2.pan_l,
            air + s1 * self.cricket_1.pan_r + s2 * self.cricket_2.pan_r,
        )
    }

    /// Light Drizzle Rain matching WebView2's `NaturalLightRainDSP`.
    #[inline]
    fn step_rain_drizzle(&mut self) -> (f32, f32) {
        let dt = 1.0 / SAMPLE_RATE;
        self.drizzle_lfo = (self.drizzle_lfo + (TAU * 0.04) / SAMPLE_RATE) % TAU;
        let breath = 0.96 + 0.04 * self.drizzle_lfo.sin();

        let raw_l = self.drizzle_pink_l.step(self.rng.next_bipolar());
        let raw_r = self.drizzle_pink_r.step(self.rng.next_bipolar());
        let hp_l = self.drizzle_hp_l.process(self.drizzle_lp_l.process(raw_l));
        let hp_r = self.drizzle_hp_r.process(self.drizzle_lp_r.process(raw_r));
        let bed_l = self.drizzle_warm_l.process(hp_l) * 0.42 * breath;
        let bed_r = self.drizzle_warm_r.process(hp_r) * 0.42 * breath;

        self.drizzle_drop_timer += 1;
        if self.drizzle_drop_timer >= self.drizzle_next_drop {
            self.drizzle_drop_timer = 0;
            if let Some(d) = self.drizzle_drops.iter_mut().find(|v| !v.active) {
                d.active = true;
                d.age = 0.0;
                d.max_age = 0.006 + self.rng.next_unit() * 0.008;
                d.decay_rate = 350.0 + self.rng.next_unit() * 200.0;
                d.gain = 0.08 + self.rng.next_unit() * 0.12;
                let f = 2600.0 + self.rng.next_unit() * 1800.0;
                d.filter.set_params(FilterKind::BandPass, f, 1.0);
                let pan = self.rng.next_unit() * 1.6 - 0.8;
                d.pan_l = (1.0 - pan) * 0.5;
                d.pan_r = (1.0 + pan) * 0.5;
            }
            self.drizzle_next_drop = (SAMPLE_RATE / (10.0 + self.rng.next_unit() * 16.0)) as u32;
        }

        let (mut dl, mut dr) = (0.0f32, 0.0f32);
        for d in &mut self.drizzle_drops {
            if !d.active {
                continue;
            }
            let white = self.rng.next_bipolar();
            let env = (-d.decay_rate * d.age).exp();
            let shaped = d.filter.process(white) * env * d.gain;
            dl += shaped * d.pan_l;
            dr += shaped * d.pan_r;
            d.age += dt;
            if d.age >= d.max_age {
                d.active = false;
            }
        }
        ((bed_l + dl) * 0.95, (bed_r + dr) * 0.95)
    }

    /// Heavy Downpour matching WebView2's `NaturalHeavyRainDSP`.
    #[inline]
    fn step_rain_heavy(&mut self) -> (f32, f32) {
        let dt = 1.0 / SAMPLE_RATE;
        self.heavy_lfo[0] = (self.heavy_lfo[0] + (TAU * 0.07) / SAMPLE_RATE) % TAU;
        self.heavy_lfo[1] = (self.heavy_lfo[1] + (TAU * 0.13) / SAMPLE_RATE) % TAU;
        self.heavy_lfo[2] = (self.heavy_lfo[2] + (TAU * 0.21) / SAMPLE_RATE) % TAU;
        let surge = 0.82
            + 0.10 * self.heavy_lfo[0].sin()
            + 0.05 * self.heavy_lfo[1].sin()
            + 0.03 * self.heavy_lfo[2].sin();

        let n_l = self.heavy_pink_l.step(self.rng.next_bipolar());
        let n_r = self.heavy_pink_r.step(self.rng.next_bipolar());
        self.heavy_brown_l = (self.heavy_brown_l + 0.02 * n_l) / 1.02;
        self.heavy_brown_r = (self.heavy_brown_r + 0.02 * n_r) / 1.02;

        let r_l = self.heavy_rumble_l.process(self.heavy_brown_l) * 0.65;
        let r_r = self.heavy_rumble_r.process(self.heavy_brown_r) * 0.65;
        let m_l = self.heavy_mid_l.process(n_l) * 0.44;
        let m_r = self.heavy_mid_r.process(n_r) * 0.44;
        let s_l = self.heavy_spray_lp_l.process(self.heavy_spray_hp_l.process(n_l)) * 0.28;
        let s_r = self.heavy_spray_lp_r.process(self.heavy_spray_hp_r.process(n_r)) * 0.28;

        self.heavy_splat_timer += 1;
        if self.heavy_splat_timer >= self.heavy_next_splat {
            self.heavy_splat_timer = 0;
            if let Some(sp) = self.heavy_splatters.iter_mut().find(|v| !v.active) {
                sp.active = true;
                sp.age = 0.0;
                sp.max_age = 0.010 + self.rng.next_unit() * 0.014;
                sp.decay_rate = 180.0 + self.rng.next_unit() * 180.0;
                sp.gain = 0.14 + self.rng.next_unit() * 0.18;
                sp.filter.set_params(FilterKind::BandPass, 1100.0 + self.rng.next_unit() * 1400.0, 0.9);
                let pan = self.rng.next_unit() * 1.4 - 0.7;
                sp.pan_l = (1.0 - pan) * 0.5;
                sp.pan_r = (1.0 + pan) * 0.5;
            }
            self.heavy_next_splat = (SAMPLE_RATE / (18.0 + self.rng.next_unit() * 24.0)) as u32;
        }

        let (mut pl, mut pr) = (0.0f32, 0.0f32);
        for sp in &mut self.heavy_splatters {
            if !sp.active {
                continue;
            }
            let env = (-sp.decay_rate * sp.age).exp();
            let v = sp.filter.process(self.rng.next_bipolar()) * env * sp.gain;
            pl += v * sp.pan_l;
            pr += v * sp.pan_r;
            sp.age += dt;
            if sp.age >= sp.max_age {
                sp.active = false;
            }
        }
        (((r_l + m_l + s_l) * surge + pl) * 0.9, ((r_r + m_r + s_r) * surge + pr) * 0.9)
    }

    #[inline]
    fn step_night_breeze(&mut self) -> (f32, f32) {
        self.night_lfo = (self.night_lfo + (TAU * 0.06) / SAMPLE_RATE) % TAU;
        let swell = 0.32 + 0.08 * self.night_lfo.sin();
        let w = self.rng.next_bipolar();
        self.night_brown = (self.night_brown + 0.02 * w) / 1.02;
        let s = self.night_lp.process(self.night_brown) * swell * 2.2;
        (s * 0.96, s * 1.04)
    }

    #[inline]
    fn step_clock(&mut self) -> (f32, f32) {
        self.clock_sample += 1;
        if self.clock_sample >= 44_100 {
            self.clock_sample = 0;
            self.clock_is_tock = !self.clock_is_tock;
            self.clock_play_pos = 0;
        }
        let ir = if self.clock_is_tock {
            &self.clock_tock_ir
        } else {
            &self.clock_tick_ir
        };
        if self.clock_play_pos < ir.len() {
            let sample = ir[self.clock_play_pos];
            self.clock_play_pos += 1;
            sample
        } else {
            (0.0, 0.0)
        }
    }

    #[inline]
    fn step_cafe(&mut self) -> (f32, f32) {
        if self.cafe_samples.is_empty() {
            return (0.0, 0.0);
        }
        let idx = self.cafe_pos as usize;
        let next_idx = if idx + 1 < self.cafe_samples.len() { idx + 1 } else { 0 };
        let frac = self.cafe_pos - idx as f32;
        let s0 = self.cafe_samples[idx];
        let s1 = self.cafe_samples[next_idx];
        let s = s0 + frac * (s1 - s0);

        self.cafe_pos += 22_050.0 / SAMPLE_RATE;
        if self.cafe_pos >= self.cafe_samples.len() as f32 {
            self.cafe_pos -= self.cafe_samples.len() as f32;
        }

        (s * 0.94, s * 1.06)
    }

    #[inline]
    fn step_cafe_jp(&mut self) -> (f32, f32) {
        if self.cafe_jp_samples.is_empty() {
            return (0.0, 0.0);
        }
        let idx = self.cafe_jp_pos as usize;
        let next_idx = if idx + 1 < self.cafe_jp_samples.len() { idx + 1 } else { 0 };
        let frac = self.cafe_jp_pos - idx as f32;
        let s0 = self.cafe_jp_samples[idx];
        let s1 = self.cafe_jp_samples[next_idx];
        let s = s0 + frac * (s1 - s0);

        self.cafe_jp_pos += 22_050.0 / SAMPLE_RATE;
        if self.cafe_jp_pos >= self.cafe_jp_samples.len() as f32 {
            self.cafe_jp_pos -= self.cafe_jp_samples.len() as f32;
        }

        (s * 0.96, s * 1.04)
    }

    #[inline]
    fn step_ocean(&mut self) -> (f32, f32) {
        self.ocean_lfo = (self.ocean_lfo + (TAU * 0.11) / SAMPLE_RATE) % TAU;
        let swell = 0.25 + 0.75 * (0.5 + 0.5 * self.ocean_lfo.sin()).powf(1.8);
        let p = self.ocean_pink.step(self.rng.next_bipolar());
        let w = self.rng.next_bipolar();
        self.ocean_brown = (self.ocean_brown + 0.015 * w) / 1.015;
        let surf = self.ocean_lp.process(p * 0.35 + self.ocean_brown * 0.95) * swell;
        (surf * 0.98, surf * 1.02)
    }

    #[inline]
    fn step_alpha(&mut self) -> (f32, f32) {
        self.alpha_lfo = (self.alpha_lfo + (TAU * 0.12) / SAMPLE_RATE) % TAU;
        let breath = 0.82 + 0.18 * self.alpha_lfo.sin();
        self.alpha_phase_l = (self.alpha_phase_l + TAU * 216.0 / SAMPLE_RATE) % TAU;
        self.alpha_phase_r = (self.alpha_phase_r + TAU * 221.0 / SAMPLE_RATE) % TAU;
        let l = (self.alpha_phase_l.sin() * 0.65 + (self.alpha_phase_l * 2.0).sin() * 0.35) * 0.14 * breath;
        let r = (self.alpha_phase_r.sin() * 0.65 + (self.alpha_phase_r * 2.0).sin() * 0.35) * 0.14 * breath;
        (l, r)
    }

    fn step_alerts(&mut self) -> f32 {
        let mut sum = 0.0f32;
        self.alerts.retain_mut(|tone| {
            if tone.delay_samples > 0 {
                tone.delay_samples -= 1;
                return true;
            }
            if tone.elapsed_samples >= tone.dur_samples {
                return false;
            }
            let t = tone.elapsed_samples as f32 / SAMPLE_RATE;
            let attack = (t / 0.006).min(1.0);
            let decay = (1.0 - tone.elapsed_samples as f32 / tone.dur_samples as f32).powf(2.2);
            let env = attack * decay * tone.amp;
            let s1 = (TAU * tone.freq_hz * t).sin();
            let s2 = (TAU * (tone.freq_hz * 2.0) * t).sin() * tone.harmonic;
            sum += (s1 + s2) * env;
            tone.elapsed_samples += 1;
            true
        });
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_renders_bounded_samples_across_all_presets() {
        for preset in AMBIENT_PRESETS {
            let mut cfg = AmbientConfig::default();
            assert!(cfg.apply_preset(preset.id), "missing preset {}", preset.id);
            let mut bank = SynthBank::new(&cfg);
            bank.apply_config(&cfg, true, false);
            let mut buf = vec![0.0f32; 4096 * 2];
            bank.render_stereo(&mut buf);
            let peak = buf.iter().copied().map(f32::abs).fold(0.0f32, f32::max);
            assert!(peak > 0.001, "preset {} rendered silence", preset.id);
            assert!(buf.iter().all(|s| s.is_finite() && *s >= -1.0 && *s <= 1.0));
        }
    }

    #[test]
    fn muted_mixer_settles_to_silence_and_alerts_still_sound() {
        let cfg = AmbientConfig::default();
        let mut bank = SynthBank::new(&cfg);
        bank.apply_config(&cfg, false, true);
        let mut buf = vec![0.0f32; 512 * 2];
        bank.render_stereo(&mut buf);
        assert!(buf.iter().all(|s| s.abs() < 1e-4));

        bank.trigger_alert(AlertKind::Chime);
        assert!(bank.has_active_alerts());
        bank.render_stereo(&mut buf);
        let peak = buf.iter().copied().map(f32::abs).fold(0.0f32, f32::max);
        assert!(peak > 0.02);
    }
}
