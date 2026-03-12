// Security module — bytecode encryption, obfuscation, and integrity checking.
// These features are experimental in v0.4: they compile but are not wired to any CLI
// command or runtime path. Planned for exposure via a future `txtcode compile --encrypt` flag.
pub mod encryptor;
pub mod integrity;
pub mod obfuscator;
pub mod protector;

#[allow(unused_imports)]
pub use encryptor::*;
#[allow(unused_imports)]
pub use integrity::*;
#[allow(unused_imports)]
pub use obfuscator::*;
#[allow(unused_imports)]
pub use protector::*;
