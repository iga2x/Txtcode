// WiFi capabilities — string constants for PermissionResource::WiFi.
//
// STATUS: ENFORCED — PermissionResource::WiFi(String) is a real variant in
// permissions.rs. Calling any `wifi_*` function without `wifi.<action>` granted
// will be rejected by `check_permission_with_audit` and logged to the audit trail.
//
// Capability strings accepted in env.toml and `grant_permission`:
//   wifi.scan, wifi.capture, wifi.deauth, wifi.inject
//
// Required by stdlib functions: wifi_scan, wifi_capture, wifi_deauth, wifi_inject.

/// WiFi capability string constants.
pub struct WiFiCapability;

impl WiFiCapability {
    /// WiFi scan capability (passive interface enumeration / probe response capture)
    pub const SCAN: &'static str = "wifi.scan";

    /// WiFi capture capability (raw frame capture via monitor mode)
    pub const CAPTURE: &'static str = "wifi.capture";

    /// WiFi deauthentication capability (send deauth frames — requires authorisation)
    pub const DEAUTH: &'static str = "wifi.deauth";

    /// WiFi packet injection capability (raw frame injection — requires authorisation)
    pub const INJECT: &'static str = "wifi.inject";
}
