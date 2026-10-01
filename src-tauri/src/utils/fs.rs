use std::io::Write;
use std::path::Path;

/// Write `data` to `path` so readers see either the old file or the new one,
/// never a half-written file: write a temp file in the same directory, flush
/// it to disk, then rename it over the target.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_file_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("ss-fs-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("data.json");
        std::fs::write(&path, "old").ok();

        let written = write_atomic(&path, b"new").is_ok();

        let entries = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!((written, content.as_str(), entries), (true, "new", 1));
    }
}
