use std::path::{Path, PathBuf};

pub fn build_stored_path(
    library_dir: &Path,
    filename: &str,
    _file_hash: &str,
    folder_type: &str,
) -> PathBuf {
    let safe_name = sanitize_filename(filename);
    library_dir.join(folder_type).join(safe_name)
}

pub fn is_old_library_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() <= 20 {
        return false;
    }

    is_digit(bytes[0])
        && is_digit(bytes[1])
        && is_digit(bytes[2])
        && is_digit(bytes[3])
        && bytes[4] == b'-'
        && is_digit(bytes[5])
        && is_digit(bytes[6])
        && bytes[7] == b'-'
        && is_digit(bytes[8])
        && is_digit(bytes[9])
        && bytes[10] == b'_'
        && bytes[11..19].iter().all(|b| b.is_ascii_hexdigit())
        && bytes[19] == b'_'
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
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
        assert_eq!(s, "library/public/note.md");
        assert!(!s.contains("a81f39c2"));
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
        assert_eq!(s, "library/private/secret.md");
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
        assert_eq!(s, "library/public/evil_.._.._etc_passwd.md");
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
        assert_eq!(s, "library/public/unnamed");
    }

    #[test]
    fn old_library_filename_detection_matches_legacy_names() {
        assert!(is_old_library_filename("2026-05-23_b8184ef2_test.txt"));
        assert!(is_old_library_filename(
            "2026-05-23_c4b391be_AI使用方法.txt"
        ));
        assert!(!is_old_library_filename("test.txt"));
        assert!(!is_old_library_filename("2026-05-23_test.txt"));
        assert!(!is_old_library_filename("2026-05-23_nothexzz_test.txt"));
    }
}
