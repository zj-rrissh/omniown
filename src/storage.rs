use chrono::Local;
use std::path::{Path, PathBuf};

pub fn build_stored_path(
    library_dir: &Path,
    filename: &str,
    file_hash: &str,
    folder_type: &str,
) -> PathBuf {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let hash8 = &file_hash[..8.min(file_hash.len())];
    let safe_name = sanitize_filename(filename);

    let mut path = library_dir
        .join(folder_type)
        .join(format!("{}_{}_{}", date, hash8, safe_name));

    if path.exists() {
        let stem = safe_name
            .rsplit_once('.')
            .map(|(s, _)| s.to_string())
            .unwrap_or_else(|| safe_name.clone());
        let ext = safe_name
            .rsplit_once('.')
            .map(|(_, e)| e.to_string())
            .unwrap_or_default();

        for i in 1..1000u32 {
            let candidate = if ext.is_empty() {
                format!("{}_{}", stem, i)
            } else {
                format!("{}_{}.{}", stem, i, ext)
            };
            let candidate_path = library_dir
                .join(folder_type)
                .join(format!("{}_{}_{}", date, hash8, candidate));
            if !candidate_path.exists() {
                path = candidate_path;
                break;
            }
        }
    }

    path
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            other => other,
        })
        .collect();

    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_stored_path_has_correct_prefix() {
        let path = build_stored_path(
            Path::new("library"),
            "note.md",
            "a81f39c2abcdef1234567890abcdef1234567890",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert!(s.starts_with("library/public/"));
        assert!(s.contains("a81f39c2"));
        assert!(s.ends_with("note.md"));
    }

    #[test]
    fn private_file_has_private_prefix() {
        let path = build_stored_path(
            Path::new("library"),
            "secret.md",
            "bbbbbbbb0000000000000000000000000000000000",
            "private",
        );
        let s = path.to_string_lossy().to_string();
        assert!(s.starts_with("library/private/"));
        assert!(s.ends_with("secret.md"));
    }

    #[test]
    fn sanitize_removes_path_separators() {
        let path = build_stored_path(
            Path::new("library"),
            "evil/../../etc/passwd.md",
            "aaaaaaaa0000000000000000000000000000000000",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert!(!s.contains("../"));
        assert!(s.starts_with("library/public/"));
    }

    #[test]
    fn empty_filename_gets_default() {
        let path = build_stored_path(
            Path::new("library"),
            "",
            "aaaaaaaa0000000000000000000000000000000000",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert!(s.contains("unnamed"));
    }
}
