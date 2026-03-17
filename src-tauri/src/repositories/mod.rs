// Repository Layer
// Provides data access abstractions for SQLite database

// Re-export repositories from specforge-lib
pub use specforge_lib::repositories::*;

// Local repositories (spec and schema index)
pub mod schema_repo;
pub mod spec_repo;
pub mod workflow_instance_repo;
