use crate::runtime::migration::MigrationFramework;
use crate::runtime::compatibility::Version;
use crate::runtime::errors::RuntimeError;
use std::path::PathBuf;

/// CLI command for migrating code between versions
pub fn migrate_command(
    files: Vec<String>,
    from_version: Option<String>,
    to_version: Option<String>,
    dry_run: bool,
    strict: bool,
    directory: Option<String>,
) -> Result<(), RuntimeError> {
    let framework = MigrationFramework::new()
        .with_dry_run(dry_run)
        .with_strict_mode(strict);

    let source_version = from_version
        .as_ref()
        .and_then(|v| Version::from_string(v).ok());
    let target_version = to_version
        .as_ref()
        .and_then(|v| Version::from_string(v).ok());

    if let Some(dir) = directory {
        // Migrate entire directory
        let dir_path = PathBuf::from(dir);
        let reports = framework.migrate_directory(&dir_path, source_version, target_version, Some("*.tc"))?;
        
        let summary = MigrationFramework::generate_summary(&reports);
        println!("{}", summary);
        
        // Check if any errors occurred
        let has_errors = reports.iter().any(|(_, report)| report.has_errors());
        if has_errors {
            return Err(RuntimeError::new("Migration completed with errors. Review the report above.".to_string()));
        }
    } else if !files.is_empty() {
        // Migrate specific files
        let mut all_reports = Vec::new();
        
        for file in files {
            let file_path = PathBuf::from(&file);
            match framework.migrate_file(&file_path, source_version.clone(), target_version.clone()) {
                Ok(report) => {
                    println!("{}\n", report.to_string());
                    all_reports.push((file_path, report));
                }
                Err(e) => {
                    eprintln!("Error migrating {}: {}", file, e);
                    return Err(e);
                }
            }
        }
        
        // Generate summary
        if all_reports.len() > 1 {
            let summary = MigrationFramework::generate_summary(&all_reports);
            println!("\n{}", summary);
        }
        
        // Check for errors
        let has_errors = all_reports.iter().any(|(_, report)| report.has_errors());
        if has_errors && strict {
            return Err(RuntimeError::new("Migration completed with errors. Use --no-strict to allow warnings.".to_string()));
        }
    } else {
        return Err(RuntimeError::new("No files or directory specified. Use --files or --directory.".to_string()));
    }

    Ok(())
}

