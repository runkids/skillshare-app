// Snapshot Storage Service
// Handles file-based storage for snapshot data with zstd compression

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Snapshot storage service for file operations
pub struct SnapshotStorage {
    base_path: PathBuf,
}

impl SnapshotStorage {
    /// Create a new SnapshotStorage with the given base path
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Get the storage path for a snapshot
    pub fn get_snapshot_path(&self, snapshot_id: &str) -> PathBuf {
        self.base_path.join("snapshots").join(snapshot_id)
    }

    /// Ensure the snapshot directory exists
    pub fn ensure_snapshot_dir(&self, snapshot_id: &str) -> Result<PathBuf, String> {
        let path = self.get_snapshot_path(snapshot_id);
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create snapshot directory: {}", e))?;
        Ok(path)
    }

    /// Store compressed lockfile data
    pub fn store_lockfile(
        &self,
        snapshot_id: &str,
        lockfile_name: &str,
        content: &[u8],
    ) -> Result<(PathBuf, u64), String> {
        let dir = self.ensure_snapshot_dir(snapshot_id)?;
        let compressed_name = format!("{}.zst", lockfile_name);
        let path = dir.join(&compressed_name);

        let compressed = self.compress(content)?;
        let size = compressed.len() as u64;

        fs::write(&path, &compressed)
            .map_err(|e| format!("Failed to write compressed lockfile: {}", e))?;

        Ok((path, size))
    }

    /// Store package.json
    pub fn store_package_json(
        &self,
        snapshot_id: &str,
        content: &[u8],
    ) -> Result<(PathBuf, u64), String> {
        let dir = self.ensure_snapshot_dir(snapshot_id)?;
        let path = dir.join("package.json.zst");

        let compressed = self.compress(content)?;
        let size = compressed.len() as u64;

        fs::write(&path, &compressed)
            .map_err(|e| format!("Failed to write package.json: {}", e))?;

        Ok((path, size))
    }

    /// Store dependency tree as JSON
    pub fn store_dependency_tree(
        &self,
        snapshot_id: &str,
        tree: &serde_json::Value,
    ) -> Result<(PathBuf, u64), String> {
        let dir = self.ensure_snapshot_dir(snapshot_id)?;
        let path = dir.join("dependency-tree.json.zst");

        let json = serde_json::to_vec(tree)
            .map_err(|e| format!("Failed to serialize dependency tree: {}", e))?;
        let compressed = self.compress(&json)?;
        let size = compressed.len() as u64;

        fs::write(&path, &compressed)
            .map_err(|e| format!("Failed to write dependency tree: {}", e))?;

        Ok((path, size))
    }

    /// Store postinstall scripts manifest
    pub fn store_postinstall_manifest(
        &self,
        snapshot_id: &str,
        manifest: &serde_json::Value,
    ) -> Result<(PathBuf, u64), String> {
        let dir = self.ensure_snapshot_dir(snapshot_id)?;
        let path = dir.join("postinstall-manifest.json.zst");

        let json = serde_json::to_vec(manifest)
            .map_err(|e| format!("Failed to serialize postinstall manifest: {}", e))?;
        let compressed = self.compress(&json)?;
        let size = compressed.len() as u64;

        fs::write(&path, &compressed)
            .map_err(|e| format!("Failed to write postinstall manifest: {}", e))?;

        Ok((path, size))
    }

    /// Read and decompress a stored file
    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        let compressed =
            fs::read(path).map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
        self.decompress(&compressed)
    }

    /// Read lockfile from snapshot
    pub fn read_lockfile(&self, snapshot_id: &str, lockfile_name: &str) -> Result<Vec<u8>, String> {
        let path = self
            .get_snapshot_path(snapshot_id)
            .join(format!("{}.zst", lockfile_name));
        self.read_file(&path)
    }

    /// Read package.json from snapshot
    pub fn read_package_json(&self, snapshot_id: &str) -> Result<Vec<u8>, String> {
        let path = self
            .get_snapshot_path(snapshot_id)
            .join("package.json.zst");
        self.read_file(&path)
    }

    /// Read dependency tree from snapshot
    pub fn read_dependency_tree(&self, snapshot_id: &str) -> Result<serde_json::Value, String> {
        let path = self
            .get_snapshot_path(snapshot_id)
            .join("dependency-tree.json.zst");
        let data = self.read_file(&path)?;
        serde_json::from_slice(&data)
            .map_err(|e| format!("Failed to parse dependency tree: {}", e))
    }

    /// Delete a snapshot directory
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), String> {
        let path = self.get_snapshot_path(snapshot_id);
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|e| format!("Failed to delete snapshot directory: {}", e))?;
        }
        Ok(())
    }

    /// Get total storage size for a snapshot
    pub fn get_snapshot_size(&self, snapshot_id: &str) -> Result<u64, String> {
        let path = self.get_snapshot_path(snapshot_id);
        if !path.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        for entry in
            fs::read_dir(&path).map_err(|e| format!("Failed to read snapshot directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let metadata = entry
                .metadata()
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;
            total_size += metadata.len();
        }

        Ok(total_size)
    }

    /// Compress data using zstd
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut encoder = zstd::Encoder::new(Vec::new(), 3)
            .map_err(|e| format!("Failed to create zstd encoder: {}", e))?;
        encoder
            .write_all(data)
            .map_err(|e| format!("Failed to compress data: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to finish compression: {}", e))
    }

    /// Decompress zstd data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decoder = zstd::Decoder::new(data)
            .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("Failed to decompress data: {}", e))?;
        Ok(decompressed)
    }

    /// Clean up orphaned snapshot directories
    pub fn cleanup_orphaned(&self, valid_ids: &[String]) -> Result<usize, String> {
        let snapshots_dir = self.base_path.join("snapshots");
        if !snapshots_dir.exists() {
            return Ok(0);
        }

        let mut removed = 0;
        for entry in fs::read_dir(&snapshots_dir)
            .map_err(|e| format!("Failed to read snapshots directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();

                if !valid_ids.contains(&dir_name.to_string()) {
                    fs::remove_dir_all(&path)
                        .map_err(|e| format!("Failed to remove orphaned directory: {}", e))?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compress_decompress() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(temp_dir.path().to_path_buf());

        let original = b"Hello, World! This is a test of zstd compression.";
        let compressed = storage.compress(original).unwrap();
        let decompressed = storage.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_store_and_read_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(temp_dir.path().to_path_buf());

        let content = b"{\"name\": \"test\", \"lockfileVersion\": 3}";
        let snapshot_id = "test-snapshot-123";

        let (path, size) = storage
            .store_lockfile(snapshot_id, "package-lock.json", content)
            .unwrap();

        assert!(path.exists());
        assert!(size > 0);

        let read_content = storage
            .read_lockfile(snapshot_id, "package-lock.json")
            .unwrap();
        assert_eq!(content.as_slice(), read_content.as_slice());
    }

    #[test]
    fn test_delete_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(temp_dir.path().to_path_buf());

        let snapshot_id = "test-snapshot-to-delete";
        let content = b"test content";

        storage
            .store_lockfile(snapshot_id, "package-lock.json", content)
            .unwrap();

        let path = storage.get_snapshot_path(snapshot_id);
        assert!(path.exists());

        storage.delete_snapshot(snapshot_id).unwrap();
        assert!(!path.exists());
    }
}
