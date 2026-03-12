// Network capabilities - net.scan, net.connect, etc.

/// Network capability definitions
pub struct NetworkCapability;

impl NetworkCapability {
    /// Network scan capability
    pub const SCAN: &'static str = "net.scan";

    /// Network connect capability
    pub const CONNECT: &'static str = "net.connect";

    /// Network listen capability
    pub const LISTEN: &'static str = "net.listen";

    /// Network send capability
    pub const SEND: &'static str = "net.send";

    /// Network receive capability
    pub const RECEIVE: &'static str = "net.receive";
}
