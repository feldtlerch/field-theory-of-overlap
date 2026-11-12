//! Descriptor-relative observations anchored to complete support verification.
#![allow(clippy::result_large_err)] // Preserve both complete identity observations.
use std::{
    collections::BTreeMap,
    ffi::{CStr, CString, OsString},
    fs::{File, Metadata},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::MetadataExt,
        },
    },
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupportIdentity {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub mode: u32,
    pub mtime_seconds: i64,
    pub mtime_nanoseconds: i64,
    pub ctime_seconds: i64,
    pub ctime_nanoseconds: i64,
}

impl From<&Metadata> for SupportIdentity {
    fn from(value: &Metadata) -> Self {
        Self {
            device: value.dev(),
            inode: value.ino(),
            size: value.size(),
            mode: value.mode(),
            mtime_seconds: value.mtime(),
            mtime_nanoseconds: value.mtime_nsec(),
            ctime_seconds: value.ctime(),
            ctime_nanoseconds: value.ctime_nsec(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Fault {
    Io {
        path: PathBuf,
        kind: io::ErrorKind,
    },
    Changed {
        path: PathBuf,
        expected: SupportIdentity,
        actual: Option<SupportIdentity>,
    },
    Inventory {
        missing: Vec<PathBuf>,
        unexpected: Vec<PathBuf>,
    },
}
fn io_fault(path: &Path, error: &io::Error) -> Fault {
    Fault::Io {
        path: path.into(),
        kind: error.kind(),
    }
}

fn child(parent: &File, name: &std::ffi::OsStr, directory: bool) -> io::Result<File> {
    let name = CString::new(name.as_bytes())?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    // SAFETY: name is terminated and parent remains open for this syscall.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn opened(
    path: &Path,
    mut observe: impl FnMut(&Path, &File) -> io::Result<()>,
) -> io::Result<File> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut file = File::open("/")?;
    let mut current = PathBuf::from("/");
    observe(&current, &file)?;
    let components: Vec<_> = path.components().collect();
    for (index, component) in components.iter().enumerate() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                file = child(&file, name, index + 1 != components.len())?;
                current.push(name);
                observe(&current, &file)?;
            }
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        }
    }
    Ok(file)
}

pub(crate) fn open_file(path: &Path) -> io::Result<File> {
    let file = opened(path, |_, _| Ok(()))?;
    if !file.metadata()?.is_file() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(file)
}

pub(crate) fn open_directory(path: &Path) -> io::Result<File> {
    let file = opened(path, |_, _| Ok(()))?;
    if !file.metadata()?.is_dir() {
        return Err(io::ErrorKind::NotADirectory.into());
    }
    Ok(file)
}

pub(crate) fn file_beneath(directory: &File, relative: &Path) -> io::Result<File> {
    let components: Vec<_> = relative.components().collect();
    let mut parent = None;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        let file = child(
            parent.as_ref().unwrap_or(directory),
            name,
            index + 1 != components.len(),
        )?;
        parent = Some(file);
    }
    let file = parent.ok_or(io::ErrorKind::InvalidInput)?;
    if !file.metadata()?.is_file() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(file)
}

struct Directory(*mut libc::DIR);
impl Drop for Directory {
    fn drop(&mut self) {
        // SAFETY: fdopendir transferred ownership to this live directory stream.
        unsafe {
            libc::closedir(self.0);
        }
    }
}
fn names(file: &File) -> io::Result<Vec<OsString>> {
    let descriptor = file.try_clone()?.into_raw_fd();
    // SAFETY: descriptor is owned and refers to the directory being enumerated.
    let pointer = unsafe { libc::fdopendir(descriptor) };
    if pointer.is_null() {
        let error = io::Error::last_os_error();
        // SAFETY: fdopendir failed and did not take ownership.
        unsafe {
            libc::close(descriptor);
        }
        return Err(error);
    }
    let directory = Directory(pointer);
    let mut result = Vec::new();
    loop {
        // SAFETY: errno is thread-local; the stream is exclusively owned here.
        let entry = unsafe {
            *libc::__errno_location() = 0;
            libc::readdir(directory.0)
        };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(0) {
                return Err(error);
            }
            break;
        }
        // SAFETY: readdir's name is terminated and copied before the next call.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            result.push(OsString::from_vec(name.to_vec()));
        }
    }
    result.sort();
    Ok(result)
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    roots: Vec<PathBuf>,
    members: BTreeMap<PathBuf, SupportIdentity>,
}
impl Snapshot {
    pub(crate) fn member_count(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn capture(roots: &[PathBuf]) -> Result<Self, Fault> {
        let mut members = BTreeMap::new();
        for path in roots {
            let mut changed = None;
            let file = opened(path, |ancestor, file| {
                let identity = SupportIdentity::from(&file.metadata()?);
                if let Some(previous) = members.insert(ancestor.to_owned(), identity)
                    && previous != identity
                    && changed.is_none()
                {
                    changed = Some(Fault::Changed {
                        path: ancestor.into(),
                        expected: previous,
                        actual: Some(identity),
                    });
                }
                Ok(())
            });
            if let Some(fault) = changed {
                return Err(fault);
            }
            let file = file.map_err(|e| io_fault(path, &e))?;
            Self::descend(path, &file, &mut members)?;
        }
        Ok(Self {
            roots: roots.to_vec(),
            members,
        })
    }
    fn descend(
        path: &Path,
        file: &File,
        members: &mut BTreeMap<PathBuf, SupportIdentity>,
    ) -> Result<(), Fault> {
        let before = file.metadata().map_err(|e| io_fault(path, &e))?;
        let identity = SupportIdentity::from(&before);
        members.insert(path.into(), identity);
        if before.is_dir() {
            for name in names(file).map_err(|e| io_fault(path, &e))? {
                let path = path.join(&name);
                let file = child(file, &name, false).map_err(|e| io_fault(&path, &e))?;
                Self::descend(&path, &file, members)?;
            }
        } else if !before.is_file() {
            return Err(Fault::Io {
                path: path.into(),
                kind: io::ErrorKind::InvalidInput,
            });
        }
        let actual = SupportIdentity::from(&file.metadata().map_err(|e| io_fault(path, &e))?);
        if identity != actual {
            return Err(Fault::Changed {
                path: path.into(),
                expected: identity,
                actual: Some(actual),
            });
        }
        Ok(())
    }
    pub(crate) fn check(&self) -> Result<usize, Fault> {
        let actual = Self::capture(&self.roots).map_err(|fault| match fault {
            Fault::Changed { path, actual, .. } => {
                if let Some(expected) = self.members.get(&path) {
                    Fault::Changed {
                        path,
                        expected: *expected,
                        actual,
                    }
                } else {
                    Fault::Inventory {
                        missing: vec![],
                        unexpected: vec![path],
                    }
                }
            }
            fault => fault,
        })?;
        let missing: Vec<_> = self
            .members
            .keys()
            .filter(|p| !actual.members.contains_key(*p))
            .cloned()
            .collect();
        let unexpected: Vec<_> = actual
            .members
            .keys()
            .filter(|p| !self.members.contains_key(*p))
            .cloned()
            .collect();
        if !missing.is_empty() || !unexpected.is_empty() {
            return Err(Fault::Inventory {
                missing,
                unexpected,
            });
        }
        // A replaced member is the precise cause even when its parent also
        // changed. All ancestor identities still follow in the same pass.
        let directory = |identity: &SupportIdentity| identity.mode & libc::S_IFMT == libc::S_IFDIR;
        for (path, expected) in self
            .members
            .iter()
            .filter(|(_, id)| !directory(id))
            .chain(self.members.iter().filter(|(_, id)| directory(id)))
        {
            let actual = actual.members.get(path).copied();
            if actual != Some(*expected) {
                return Err(Fault::Changed {
                    path: path.clone(),
                    expected: *expected,
                    actual,
                });
            }
        }
        Ok(self.members.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Read, os::unix::fs::symlink};

    #[test]
    fn descriptor_walk_refuses_member_and_ancestor_links_and_rereads_restoration() {
        let root = std::env::temp_dir().join(format!("ftoo-descriptor-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let directory = root.join("members");
        fs::create_dir(&directory).unwrap();
        let file = directory.join("member");
        fs::write(&file, b"locked bytes").unwrap();
        let saved = directory.join("saved");
        fs::rename(&file, &saved).unwrap();
        symlink(&saved, &file).unwrap();
        let parent = open_directory(&directory).unwrap();
        assert_eq!(
            file_beneath(&parent, Path::new("member"))
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ELOOP)
        );
        assert_eq!(
            file_beneath(&parent, Path::new("../members/member"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            open_file(&file).unwrap_err().raw_os_error(),
            Some(libc::ELOOP)
        );
        fs::remove_file(&file).unwrap();
        fs::rename(&saved, &file).unwrap();
        let mut bytes = Vec::new();
        file_beneath(&parent, Path::new("member"))
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, b"locked bytes");
        bytes.clear();
        open_file(&file).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"locked bytes");
        let saved = root.join("saved-directory");
        fs::rename(&directory, &saved).unwrap();
        symlink(&saved, &directory).unwrap();
        assert_eq!(
            open_file(&file).unwrap_err().raw_os_error(),
            Some(libc::ENOTDIR)
        );
        fs::remove_file(&directory).unwrap();
        fs::rename(&saved, &directory).unwrap();
        bytes.clear();
        open_file(&file).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"locked bytes");
        fs::remove_dir_all(root).unwrap();
    }
}
