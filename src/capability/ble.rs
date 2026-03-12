// BLE capabilities - ble.scan, ble.fuzz, etc.
//
// NOTE: These constants are NOT parsed by `PermissionResource::from_string` and
// have no runtime enforcement path in v0.4. They are placeholders for a future
// `PermissionResource::BLE` variant. Do not use in env.toml — the string will
// be silently ignored by the permission loader.

/// BLE capability definitions (experimental — no runtime enforcement in v0.4)
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
