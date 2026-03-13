// WiFi capabilities - string constants for a future PermissionResource::WiFi variant.
//
// STATUS: UNENFORCED STUB — no runtime enforcement in any current code path.
//
// These constants are NOT parsed by `PermissionResource::from_string`.
// Adding them to env.toml or calling `grant_permission` with these strings
// has NO EFFECT. Permission checks for WiFi operations always fail because
// `PermissionResource::WiFi` does not exist yet.
//
// Do not expose these to users as security controls. Before graduation:
//   1. Add `PermissionResource::WiFi(String)` to permissions.rs
//   2. Add WiFi parsing to `PermissionResource::from_string`
//   3. Wire enforcement through `check_permission_with_audit`

/// WiFi capability string constants (UNENFORCED — see module doc above)
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
