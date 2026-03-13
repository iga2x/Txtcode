// Security module — bytecode encryption, script signing, anti-debug, and integrity checking.
//
// MODULE OVERVIEW:
//
//   auth        — Ed25519 script signing and verification (ScriptAuth, ScriptSignature, KeyStore)
//                 Protect scripts from tampering between author and execution.
//
//   encryptor   — AES-256-GCM bytecode encryption (BytecodeEncryptor, EncryptedBytecode)
//                 Key sources: generate (random), from_passphrase (PBKDF2), from_runtime (weak).
//
//   integrity   — SHA-256 checksum + HMAC signing for code/data (IntegritySystem)
//                 Version compatibility checking and migration scripts.
//
//   protector   — Runtime anti-debug and environment integrity (RuntimeProtector)
//                 Multi-technique on Linux; timing-based + env check on all platforms.
//
//   obfuscator  — AST-level identifier obfuscation (Obfuscator) — STUB

pub mod auth;
pub mod encryptor;
pub mod integrity;
pub mod obfuscator;
pub mod protector;

#[allow(unused_imports)]
pub use auth::{KeyStore, ScriptAuth, ScriptSignature};
#[allow(unused_imports)]
pub use encryptor::{BytecodeEncryptor, EncryptedBytecode};
#[allow(unused_imports)]
pub use integrity::{CompatibilityResult, IntegritySystem, MigrationScript};
#[allow(unused_imports)]
pub use obfuscator::Obfuscator;
#[allow(unused_imports)]
pub use protector::{
    EnvironmentCheckResult, EnvironmentRisk, RuntimeProtector, SecurityCheckResult,
};
