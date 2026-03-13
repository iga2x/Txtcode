// Capability module - capability token management and capability string constants.
//
// ENFORCED (used by VirtualMachine):
//   manager    — CapabilityManager, Capability, CapabilityError, CapabilityEvent
//   filesystem — FilesystemCapability string constants (PermissionResource::FileSystem)
//   net        — NetworkCapability string constants (PermissionResource::Network)
//   wifi       — WiFiCapability string constants (PermissionResource::WiFi)
//   ble        — BLECapability string constants (PermissionResource::Bluetooth)

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
