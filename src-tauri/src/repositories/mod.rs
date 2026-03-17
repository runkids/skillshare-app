// Repository Layer
// Provides data access abstractions for SQLite database

// Re-export repositories from specforge-lib
pub use specforge_lib::repositories::*;

// Tauri-dependent repository (local)
pub mod execution_repo;
pub use execution_repo::ExecutionRepository;
