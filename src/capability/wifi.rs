// WiFi capabilities - wifi.capture, wifi.deauth, etc.

/// WiFi capability definitions
pub struct WiFiCapability;

impl WiFiCapability {
    /// WiFi scan capability
    pub const SCAN: &'static str = "wifi.scan";
    
    /// WiFi capture capability
    pub const CAPTURE: &'static str = "wifi.capture";
    
    /// WiFi deauth capability
    pub const DEAUTH: &'static str = "wifi.deauth";
    
    /// WiFi inject capability
    pub const INJECT: &'static str = "wifi.inject";
}

