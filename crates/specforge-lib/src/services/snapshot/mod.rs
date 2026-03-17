// Time Machine - Snapshot Services
// Provides snapshot capture, storage, diff, replay, search, and validation functionality

pub mod capture;
pub mod diff;
pub mod replay;
pub mod search;
pub mod storage;
pub mod validation;

pub use capture::SnapshotCaptureService;
pub use diff::SnapshotDiffService;
pub use replay::SnapshotReplayService;
pub use search::SnapshotSearchService;
pub use storage::SnapshotStorage;
pub use validation::{
    BlockedPackageEntry, LockfileValidationConfig, ValidationEngine, ValidationFailure,
    ValidationResult, ValidationRuleSet, ValidationStrictness, ValidationSummary,
    ValidationWarning,
};
