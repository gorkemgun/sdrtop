mod acc;
mod iq;
mod micro;
mod observer;
mod radio;
mod signal;
mod spectrum;
mod sweep;
mod system;
mod timing;
mod ui;
mod waterfall;

pub(crate) use acc::Accumulators;
pub use iq::IqState;
pub use micro::MicroView;
pub use observer::ObserverState;
pub use radio::RadioState;
pub use signal::SignalState;
pub use spectrum::{SpectrumMarker, SpectrumState};
pub use sweep::{SweepConfig, SweepFrame, SweepState, SWEEP_SETTLING_MS};
pub use system::SystemState;
pub use timing::{TimingQuality, TimingState, HACKRF_SAMPLES_PER_TRANSFER};
pub use ui::{InputMode, UiState};
pub use waterfall::{FftFrame, WaterfallState};

pub const THROUGHPUT_HISTORY_LEN: usize = 64;
/// SNR history depth — ~10 samples at the rx task's ~500 ms cadence ≈ 5 s window.
pub const SNR_HISTORY_LEN: usize = 10;
pub const DEFAULT_LNA_GAIN: u32 = 16;
pub const DEFAULT_VGA_GAIN: u32 = 20;
pub const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
pub const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
pub struct SdrMetrics {
    pub radio:    RadioState,
    pub signal:   SignalState,
    pub iq:       IqState,
    pub observer: ObserverState,
    pub spectrum: SpectrumState,
    pub waterfall: WaterfallState,
    pub system:   SystemState,
    pub timing:   TimingState,
    pub sweep:    SweepState,
    pub ui:       UiState,
    /// Active device's capability descriptor — drives capability-aware UI
    /// rendering (gain model, BB filter / Friis applicability, ranges). Shared
    /// (Arc) so the per-frame `SdrMetrics` clone stays cheap.
    pub caps:     std::sync::Arc<crate::hardware::DeviceCapabilities>,
    pub(crate) acc: Accumulators,
}

impl SdrMetrics {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        self.ui.push_log(msg);
    }
}
