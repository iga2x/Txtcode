// WiFi capabilities - wifi.capture, wifi.deauth, etc.
//
// NOTE: These constants are NOT parsed by `PermissionResource::from_string` and
// have no runtime enforcement path in v0.4. They are placeholders for a future
// `PermissionResource::WiFi` variant. Do not use in env.toml — the string will
// be silently ignored by the permission loader.

/// WiFi capability definitions (experimental — no runtime enforcement in v0.4)
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
