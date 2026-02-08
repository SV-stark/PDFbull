//! FFI bindings for archive operations
//!
//! Provides C-compatible API for ZIP, TAR, and directory archives.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::LazyLock;

use super::{Handle, HandleStore};
use crate::fitz::archive::{Archive, ArchiveFormat};

/// Global storage for archives
pub static ARCHIVES: LazyLock<HandleStore<Archive>> = LazyLock::new(HandleStore::new);

/// Open an archive from a file path
///
/// # Arguments
/// * `path` - Path to archive file or directory (null-terminated C string)
///
/// # Returns
/// Handle to the archive, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_archive(_ctx: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    unsafe {
        if let Ok(path_str) = CStr::from_ptr(path).to_str() {
            if let Ok(archive) = Archive::open(path_str) {
                return ARCHIVES.insert(archive);
            }
        }
    }
    0
}

/// Open an archive from a buffer
///
/// # Arguments
/// * `data` - Pointer to buffer data
/// * `size` - Size of buffer in bytes
///
/// # Returns
/// Handle to the archive, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_archive_with_buffer(
    _ctx: Handle,
    data: *const u8,
    size: usize,
) -> Handle {
    if data.is_null() || size == 0 {
        return 0;
    }

    unsafe {
        let data_slice = std::slice::from_raw_parts(data, size);
        let data_vec = data_slice.to_vec();

        if let Ok(archive) = Archive::from_buffer(data_vec) {
            return ARCHIVES.insert(archive);
        }
    }
    0
}

/// Keep an archive (increment ref count)
///
/// # Arguments
/// * `archive` - Handle to the archive
///
/// # Returns
/// The same handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_archive(_ctx: Handle, archive: Handle) -> Handle {
    archive
}

/// Drop an archive
///
/// # Arguments
/// * `archive` - Handle to the archive
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_archive(_ctx: Handle, archive: Handle) {
    ARCHIVES.remove(archive);
}

/// Get the number of entries in an archive
///
/// # Arguments
/// * `archive` - Handle to the archive
///
/// # Returns
/// Number of entries, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_archive_entries(_ctx: Handle, archive: Handle) -> i32 {
    if let Some(a) = ARCHIVES.get(archive) {
        if let Ok(guard) = a.lock() {
            if let Ok(count) = guard.count_entries() {
                return count as i32;
            }
        }
    }
    -1
}

/// List an archive entry by index
///
/// # Arguments
/// * `archive` - Handle to the archive
/// * `idx` - Entry index
/// * `buf` - Buffer to write entry name into
/// * `bufsize` - Size of buffer
///
/// # Returns
/// Number of bytes written (excluding null terminator), or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_list_archive_entry(
    _ctx: Handle,
    archive: Handle,
    idx: i32,
    buf: *mut c_char,
    bufsize: i32,
) -> i32 {
    if buf.is_null() || bufsize <= 0 || idx < 0 {
        return -1;
    }

    if let Some(a) = ARCHIVES.get(archive) {
        if let Ok(guard) = a.lock() {
            if let Ok(name) = guard.list_entry(idx as usize) {
                let name_bytes = name.as_bytes();
                let copy_len = name_bytes.len().min((bufsize - 1) as usize);

                unsafe {
                    std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), buf as *mut u8, copy_len);
                    *buf.add(copy_len) = 0; // Null terminate
                }

                return copy_len as i32;
            }
        }
    }
    -1
}

/// Check if an archive has a specific entry
///
/// # Arguments
/// * `archive` - Handle to the archive
/// * `name` - Entry name (null-terminated C string)
///
/// # Returns
/// 1 if entry exists, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_has_archive_entry(_ctx: Handle, archive: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return 0;
    }

    unsafe {
        if let Ok(name_str) = CStr::from_ptr(name).to_str() {
            if let Some(a) = ARCHIVES.get(archive) {
                if let Ok(guard) = a.lock() {
                    return if guard.has_entry(name_str) { 1 } else { 0 };
                }
            }
        }
    }
    0
}

/// Read an entry from an archive
///
/// # Arguments
/// * `archive` - Handle to the archive
/// * `name` - Entry name (null-terminated C string)
///
/// # Returns
/// Handle to a buffer containing the entry data, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_archive_entry(
    _ctx: Handle,
    archive: Handle,
    name: *const c_char,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    unsafe {
        if let Ok(name_str) = CStr::from_ptr(name).to_str() {
            if let Some(a) = ARCHIVES.get(archive) {
                if let Ok(mut guard) = a.lock() {
                    if let Ok(data) = guard.read_entry(name_str) {
                        // Create FFI buffer from data
                        let buffer = super::buffer::Buffer::from_data(&data);
                        return super::BUFFERS.insert(buffer);
                    }
                }
            }
        }
    }
    0
}

/// Get the format of an archive
///
/// # Arguments
/// * `archive` - Handle to the archive
///
/// # Returns
/// Format code: 0=unknown, 1=zip, 2=tar, 3=directory
#[unsafe(no_mangle)]
pub extern "C" fn fz_archive_format(_ctx: Handle, archive: Handle) -> i32 {
    if let Some(a) = ARCHIVES.get(archive) {
        if let Ok(guard) = a.lock() {
            return match guard.format() {
                ArchiveFormat::Unknown => 0,
                ArchiveFormat::Zip => 1,
                ArchiveFormat::Tar => 2,
                ArchiveFormat::Directory => 3,
            };
        }
    }
    0
}

/// Get all entry names from an archive
///
/// # Arguments
/// * `archive` - Handle to the archive
/// * `buf` - Buffer to write entry names into (one per line, newline-separated)
/// * `bufsize` - Size of buffer
///
/// # Returns
/// Number of bytes written, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_archive_entry_names(
    _ctx: Handle,
    archive: Handle,
    buf: *mut c_char,
    bufsize: i32,
) -> i32 {
    if buf.is_null() || bufsize <= 0 {
        return -1;
    }

    if let Some(a) = ARCHIVES.get(archive) {
        if let Ok(guard) = a.lock() {
            let names = guard.entry_names();
            let all_names = names.join("\n");
            let name_bytes = all_names.as_bytes();
            let copy_len = name_bytes.len().min((bufsize - 1) as usize);

            unsafe {
                std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0; // Null terminate
            }

            return copy_len as i32;
        }
    }
    -1
}

/// Check if an archive is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_archive_is_valid(_ctx: Handle, archive: Handle) -> i32 {
    if ARCHIVES.get(archive).is_some() {
        1
    } else {
        0
    }
}

/// Clone an archive (increase ref count)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_archive(_ctx: Handle, archive: Handle) -> Handle {
    fz_keep_archive(_ctx, archive)
}

/// Get archive entry size
#[unsafe(no_mangle)]
pub extern "C" fn fz_archive_entry_size(_ctx: Handle, archive: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    let c_name = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    if let Some(a) = ARCHIVES.get(archive) {
        if let Ok(mut guard) = a.lock() {
            if guard.has_entry(c_name) {
                if let Ok(data) = guard.read_entry(c_name) {
                    return data.len() as i32;
                }
            }
        }
    }
    -1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // Helper to create a test directory with unique name
    fn create_test_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let base_dir = std::env::temp_dir();
        // Ensure temp dir exists
        let _ = fs::create_dir_all(&base_dir);

        // Use timestamp and thread ID for uniqueness
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = std::thread::current().id();
        let dir_name = format!("micropdf_test_archive_{}_{:?}", timestamp, thread_id);
        let dir = base_dir.join(dir_name);

        fs::create_dir_all(&dir).unwrap();

        // Create test files
        fs::write(dir.join("file1.txt"), b"Hello, World!").unwrap();
        fs::write(dir.join("file2.txt"), b"Test data").unwrap();

        dir
    }

    fn cleanup_test_dir(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_open_archive_directory() {
        let test_dir = create_test_dir();
        let path_str = test_dir.to_str().unwrap();
        let path_cstr = std::ffi::CString::new(path_str).unwrap();

        let archive = fz_open_archive(0, path_cstr.as_ptr());
        assert_ne!(archive, 0);

        let format = fz_archive_format(0, archive);
        assert_eq!(format, 3); // Directory format

        fz_drop_archive(0, archive);
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_has_archive_entry() {
        let test_dir = create_test_dir();
        let path_str = test_dir.to_str().unwrap();
        let path_cstr = std::ffi::CString::new(path_str).unwrap();

        let archive = fz_open_archive(0, path_cstr.as_ptr());
        assert_ne!(archive, 0);

        // Check for existing file
        let file_name = std::ffi::CString::new("file1.txt").unwrap();
        let has_entry = fz_has_archive_entry(0, archive, file_name.as_ptr());
        assert_eq!(has_entry, 1);

        // Check for non-existing file
        let missing_file = std::ffi::CString::new("nonexistent.txt").unwrap();
        let has_missing = fz_has_archive_entry(0, archive, missing_file.as_ptr());
        assert_eq!(has_missing, 0);

        fz_drop_archive(0, archive);
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_read_archive_entry() {
        let test_dir = create_test_dir();
        let path_str = test_dir.to_str().unwrap();
        let path_cstr = std::ffi::CString::new(path_str).unwrap();

        let archive = fz_open_archive(0, path_cstr.as_ptr());
        assert_ne!(archive, 0);

        // Read file
        let file_name = std::ffi::CString::new("file1.txt").unwrap();
        let buffer_handle = fz_read_archive_entry(0, archive, file_name.as_ptr());
        assert_ne!(buffer_handle, 0);

        // Get buffer size
        let size = super::super::buffer::fz_buffer_storage(0, buffer_handle, std::ptr::null_mut());
        assert_eq!(size, 13); // "Hello, World!" length

        // Clean up
        super::super::buffer::fz_drop_buffer(0, buffer_handle);
        fz_drop_archive(0, archive);
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_open_archive_with_null_path() {
        let archive = fz_open_archive(0, std::ptr::null());
        assert_eq!(archive, 0);
    }

    #[test]
    fn test_open_archive_with_buffer() {
        // Create simple test data (not a valid archive, but tests the API)
        let data = [0u8; 100];
        let archive = fz_open_archive_with_buffer(0, data.as_ptr(), data.len());
        // Should return 0 since data is not a valid archive
        assert_eq!(archive, 0);
    }

    #[test]
    fn test_count_entries_invalid_archive() {
        let count = fz_count_archive_entries(0, 9999);
        assert_eq!(count, -1);
    }

    #[test]
    fn test_list_entry_invalid_archive() {
        let mut buf = [0i8; 256];
        let len = fz_list_archive_entry(0, 9999, 0, buf.as_mut_ptr(), 256);
        assert_eq!(len, -1);
    }

    #[test]
    fn test_archive_format_invalid() {
        let format = fz_archive_format(0, 9999);
        assert_eq!(format, 0); // Unknown
    }

    #[test]
    fn test_entry_names() {
        let test_dir = create_test_dir();
        let path_str = test_dir.to_str().unwrap();
        let path_cstr = std::ffi::CString::new(path_str).unwrap();

        let archive = fz_open_archive(0, path_cstr.as_ptr());
        assert_ne!(archive, 0);

        let mut buf = [0i8; 1024];
        let len = fz_archive_entry_names(0, archive, buf.as_mut_ptr(), 1024);
        assert!(len > 0);

        // Verify we got some names
        let names = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_str().unwrap();
        assert!(names.contains("file1.txt") || names.contains("file2.txt"));

        fz_drop_archive(0, archive);
        cleanup_test_dir(&test_dir);
    }
}
