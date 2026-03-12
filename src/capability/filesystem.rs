// Filesystem capabilities - fs.read, fs.write, etc.

/// Filesystem capability definitions
pub struct FilesystemCapability;

impl FilesystemCapability {
    /// Filesystem read capability
    pub const READ: &'static str = "fs.read";

    /// Filesystem write capability
    pub const WRITE: &'static str = "fs.write";

    /// Filesystem delete capability
    pub const DELETE: &'static str = "fs.delete";

    /// Filesystem execute capability
    pub const EXECUTE: &'static str = "fs.execute";

    /// Filesystem list capability
    pub const LIST: &'static str = "fs.list";
}
