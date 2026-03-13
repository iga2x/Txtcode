// BLE capabilities - string constants for a future PermissionResource::BLE variant.
//
// STATUS: UNENFORCED STUB — no runtime enforcement in any current code path.
//
// These constants are NOT parsed by `PermissionResource::from_string`.
// Adding them to env.toml or calling `grant_permission` with these strings
// has NO EFFECT. Permission checks for BLE operations always fail because
// `PermissionResource::BLE` does not exist yet.
//
// Do not expose these to users as security controls. Before graduation:
//   1. Add `PermissionResource::BLE(String)` to permissions.rs
//   2. Add BLE parsing to `PermissionResource::from_string`
//   3. Wire enforcement through `check_permission_with_audit`

/// BLE capability string constants (UNENFORCED — see module doc above)
pub struct BLECapability;

impl BLECapability {
    /// BLE scan capability
    pub const SCAN: &'static str = "ble.scan";

    /// BLE connect capability
    pub const CONNECT: &'static str = "ble.connect";

    /// BLE fuzz capability
    pub const FUZZ: &'static str = "ble.fuzz";

    /// BLE read capability
    pub const READ: &'static str = "ble.read";

    /// BLE write capability
    pub const WRITE: &'static str = "ble.write";
}
