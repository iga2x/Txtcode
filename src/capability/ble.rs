// BLE capabilities - ble.scan, ble.fuzz, etc.

/// BLE capability definitions
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

