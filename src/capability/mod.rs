// Capability module - capability token management and capability string constants.
//
// ENFORCED (used by VirtualMachine):
//   manager  — CapabilityManager, Capability, CapabilityError, CapabilityEvent
//   filesystem, net — FilesystemCapability / NetworkCapability string constants
//
// UNENFORCED STUBS (no PermissionResource variant, no runtime check):
//   wifi — WiFiCapability string constants only; see wifi.rs for graduation checklist
//   ble  — BLECapability  string constants only; see ble.rs  for graduation checklist

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
