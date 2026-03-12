use crate::compiler::bytecode::Bytecode;
use crate::runtime::bytecode_vm::BytecodeVM;
use std::fs;

/// Load and execute bytecode file
pub fn run_bytecode_file(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Read bytecode file
    let data = fs::read(path)?;

    // Try to deserialize as JSON first, then binary
    let bytecode: Bytecode = if let Ok(json_str) = String::from_utf8(data.clone()) {
        serde_json::from_str(&json_str)?
    } else {
        bincode::deserialize(&data)?
    };

    // Execute bytecode
    let mut vm = BytecodeVM::new();
    vm.execute(&bytecode)?;

    Ok(())
}

/// Load encrypted bytecode and execute
pub fn run_encrypted_bytecode(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use crate::security::encryptor::{BytecodeEncryptor, EncryptedBytecode};

    // Read encrypted file
    let data = fs::read(path)?;
    let encrypted = EncryptedBytecode::deserialize(&data)?;

    // Decrypt (using runtime-derived key)
    let encryptor = BytecodeEncryptor::from_runtime();
    let decrypted = encryptor.decrypt(&encrypted)?;

    // Deserialize bytecode
    let bytecode: Bytecode = bincode::deserialize(&decrypted)?;

    // Execute
    let mut vm = BytecodeVM::new();
    vm.execute(&bytecode)?;

    Ok(())
}
