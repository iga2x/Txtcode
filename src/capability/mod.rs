// Capability module - capability definitions and management
// Network and filesystem capabilities are used by the runtime permission system.
// WiFi and BLE capability modules are experimental stubs (no runtime integration in v0.4).

pub mod net;
pub mod wifi;
pub mod ble;
pub mod filesystem;
pub mod manager;

pub use manager::{CapabilityManager, Capability, CapabilityError, CapabilityEvent};
pub use net::NetworkCapability;
pub use wifi::WiFiCapability;
pub use ble::BLECapability;
pub use filesystem::FilesystemCapability;

