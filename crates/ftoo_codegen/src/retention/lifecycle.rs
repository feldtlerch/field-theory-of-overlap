use std::{fs::File, io, path::Path};

fn name(path: &str) -> io::Result<std::ffi::CString> {
    std::ffi::CString::new(path.rsplit('/').next().unwrap_or(path))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}
pub(super) fn create(directory: &File, _root: &Path, path: &str) -> io::Result<File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let name = name(path)?;
    // The borrowed directory and name remain live; success transfers ownership
    // of the new descriptor to File exactly once.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o666,
        )
    };
    if descriptor < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}
pub(super) fn replace(directory: &File, _root: &Path, path: &str) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let source = name(path)?;
    let target = c"audit-report.json";
    // Both names are relative to the same live, no-follow directory descriptor.
    let result = unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            source.as_ptr(),
            directory.as_raw_fd(),
            target.as_ptr(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
pub(super) fn remove(directory: &File, _root: &Path, path: &str) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let name = name(path)?;
    // The name is relative to the held directory, including after a refusal.
    let result = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
