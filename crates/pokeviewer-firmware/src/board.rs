//! Boundary around the pinned ESP HAL.

/// Initialized resources for the supported board.
///
/// Hardware-specific capabilities are added here rather than exposing ESP HAL
/// types to the application.
pub struct Board {
    _peripherals: esp_hal::peripherals::Peripherals,
}

impl Board {
    /// Initialize the ESP32-S3 with the project's pinned HAL configuration.
    pub fn initialize() -> Self {
        Self {
            _peripherals: esp_hal::init(esp_hal::Config::default()),
        }
    }
}
