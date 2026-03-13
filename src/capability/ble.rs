// BLE capabilities — string constants for PermissionResource::Bluetooth.
//
// STATUS: ENFORCED — PermissionResource::Bluetooth(String) is a real variant in
// permissions.rs. Calling any `ble_*` function without `ble.<action>` granted
// will be rejected by `check_permission_with_audit` and logged to the audit trail.
//
// Capability strings accepted in env.toml and `grant_permission`:
//   ble.scan, ble.connect, ble.fuzz, ble.read, ble.write
//
// Required by stdlib functions: ble_scan, ble_connect, ble_fuzz, ble_read, ble_write.

/// BLE capability string constants.
pub struct BLECapability;

impl BLECapability {
    /// BLE scan capability (passive device discovery / advertisement sniffing)
    pub const SCAN: &'static str = "ble.scan";

    /// BLE connect capability (GATT connection to a remote device)
    pub const CONNECT: &'static str = "ble.connect";

    /// BLE fuzz capability (malformed PDU injection — requires authorisation)
    pub const FUZZ: &'static str = "ble.fuzz";

    /// BLE read capability (read GATT characteristic values)
    pub const READ: &'static str = "ble.read";

    /// BLE write capability (write GATT characteristic values)
    pub const WRITE: &'static str = "ble.write";
}
