// Capability module - capability definitions and management
// Network, WiFi, BLE, filesystem capabilities

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

