// Capability module - capability token management and capability string constants.
//
// ENFORCED (used by VirtualMachine):
//   manager    — CapabilityManager, Capability, CapabilityError, CapabilityEvent
//   filesystem — FilesystemCapability string constants (PermissionResource::FileSystem)
//   net        — NetworkCapability string constants (PermissionResource::Network)

pub mod filesystem;
pub mod manager;
pub mod net;

pub use filesystem::FilesystemCapability;
pub use manager::{Capability, CapabilityError, CapabilityEvent, CapabilityManager, CapabilityResult};
pub use net::NetworkCapability;
