// Capability module - capability definitions and management
// Network and filesystem capabilities are used by the runtime permission system.
// WiFi and BLE capability modules are experimental stubs (no runtime integration in v0.4).

pub mod ble;
pub mod filesystem;
pub mod manager;
pub mod net;
pub mod wifi;

pub use ble::BLECapability;
pub use filesystem::FilesystemCapability;
pub use manager::{Capability, CapabilityError, CapabilityEvent, CapabilityManager};
pub use net::NetworkCapability;
pub use wifi::WiFiCapability;
