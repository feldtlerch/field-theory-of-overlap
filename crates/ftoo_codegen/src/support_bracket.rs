//! Descriptor-relative observations anchored to complete support verification.
#![allow(clippy::result_large_err)] // Preserve both complete identity observations.
pub(crate) mod confinement;
pub use confinement::ConfinementOperation;
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

// Read the epoch and in-flight gauge atomically with respect to each other.
static CONSUMERS: std::sync::Mutex<(u64, usize)> = std::sync::Mutex::new((0, 0));
#[cfg(test)]
pub(crate) fn consumer_epoch() -> u64 {
    CONSUMERS.lock().unwrap().0
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Observation {
    consumer: u64,
    #[cfg(test)]
    mutation: u64,
}
pub(crate) fn idle_observation(root: &Path) -> Option<Observation> {
    #[cfg(not(test))]
    let _ = root;
    let state = CONSUMERS.lock().unwrap();
    if state.1 != 0 {
        return None;
    }
    #[cfg(test)]
    let mutation = mutation::idle_epoch(root)?;
    Some(Observation {
        consumer: state.0,
        #[cfg(test)]
        mutation,
    })
}
pub(crate) struct Consumer;
impl Drop for Consumer {
    fn drop(&mut self) {
        CONSUMERS.lock().unwrap().1 -= 1;
    }
}
pub(crate) fn consumer_started() -> Consumer {
    let mut state = CONSUMERS.lock().unwrap();
    state.0 += 1;
    state.1 += 1;
    Consumer
}

#[cfg(test)]
pub(crate) mod mutation {
    use std::{
        collections::BTreeMap,
        fs, io,
        os::fd::AsRawFd,
        path::{Path, PathBuf},
        sync::{LazyLock, Mutex},
    };
    static ROOTS: LazyLock<Mutex<BTreeMap<PathBuf, (u64, usize)>>> =
        LazyLock::new(|| Mutex::new(BTreeMap::new()));
    fn normalized(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_owned())
    }
    pub fn register(root: &Path) {
        ROOTS.lock().unwrap().entry(normalized(root)).or_default();
    }
    pub(super) fn idle_epoch(root: &Path) -> Option<u64> {
        let roots = ROOTS.lock().unwrap();
        let state = roots.get(&normalized(root)).copied().unwrap_or_default();
        (state.1 == 0).then_some(state.0)
    }
    // The most specific registered fixture owns its writes. Rename and link
    // operations name directory entries; ordinary writes resolve the target.
    // Unknown roots conservatively invalidate the global observation.
    struct Mutation {
        roots: Vec<PathBuf>,
        _global: Option<super::Consumer>,
    }
    impl Drop for Mutation {
        fn drop(&mut self) {
            let mut roots = ROOTS.lock().unwrap();
            for root in &self.roots {
                roots.get_mut(root).unwrap().1 -= 1;
            }
        }
    }
    fn begin(paths: &[&Path], follow: bool) -> Mutation {
        let mut states = ROOTS.lock().unwrap();
        let mut selected = vec![];
        let mut unknown = false;
        for path in paths {
            let path = if follow {
                normalized(path)
            } else {
                path.parent().zip(path.file_name()).map_or_else(
                    || normalized(path),
                    |(parent, name)| normalized(parent).join(name),
                )
            };
            if let Some(root) = states
                .keys()
                .filter(|root| path.starts_with(root))
                .max_by_key(|root| root.components().count())
            {
                selected.push(root.clone());
            } else {
                unknown = true;
            }
            selected.extend(
                states
                    .keys()
                    .filter(|root| root.starts_with(&path))
                    .cloned(),
            );
        }
        selected.sort();
        selected.dedup();
        for root in &selected {
            let state = states.get_mut(root).unwrap();
            state.0 += 1;
            state.1 += 1;
        }
        drop(states);
        Mutation {
            roots: selected,
            _global: unknown.then(super::consumer_started),
        }
    }
    pub trait FileMutation {
        fn set_support_times(&self, times: fs::FileTimes) -> io::Result<()>;
    }
    impl FileMutation for fs::File {
        fn set_support_times(&self, times: fs::FileTimes) -> io::Result<()> {
            let path = PathBuf::from(format!("/proc/self/fd/{}", self.as_raw_fd()));
            let _mutation = begin(&[&path], true);
            self.set_times(times)
        }
    }
    pub fn set_permissions(path: impl AsRef<Path>, permissions: fs::Permissions) -> io::Result<()> {
        let _mutation = begin(&[path.as_ref()], true);
        fs::set_permissions(path, permissions)
    }
    pub fn symlink(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        let _mutation = begin(&[to.as_ref()], false);
        std::os::unix::fs::symlink(from, to)
    }
    pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
        let _mutation = begin(&[path.as_ref()], true);
        fs::write(path, bytes)
    }
    pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
        let _mutation = begin(&[path.as_ref()], false);
        fs::remove_file(path)
    }
    pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        let _mutation = begin(&[from.as_ref(), to.as_ref()], false);
        fs::rename(from, to)
    }
}

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
impl Directory {
    fn open(file: &File) -> io::Result<Self> {
        // A new open description gives this enumeration its own offset.
        let descriptor = child(file, std::ffi::OsStr::new("."), true)?.into_raw_fd();
        // SAFETY: descriptor is an owned directory descriptor.
        let pointer = unsafe { libc::fdopendir(descriptor) };
        if pointer.is_null() {
            let error = io::Error::last_os_error();
            // SAFETY: failed fdopendir did not take ownership.
            unsafe {
                libc::close(descriptor);
            }
            return Err(error);
        }
        Ok(Self(pointer))
    }
    fn next(&mut self) -> io::Result<Option<&CStr>> {
        loop {
            // SAFETY: the stream is exclusively borrowed; errno is thread-local.
            let entry = unsafe {
                *libc::__errno_location() = 0;
                libc::readdir(self.0)
            };
            if entry.is_null() {
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(0) {
                    Ok(None)
                } else {
                    Err(error)
                };
            }
            // SAFETY: the name is terminated and remains valid until next(),
            // which requires another exclusive borrow of this stream.
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() != b"." && name.to_bytes() != b".." {
                return Ok(Some(name));
            }
        }
    }
}
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
struct Baseline {
    roots: Vec<PathBuf>,
    members: BTreeMap<PathBuf, SupportIdentity>,
}
impl Baseline {
    fn capture(roots: &[PathBuf]) -> Result<Self, Fault> {
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

#[derive(Debug)]
struct Entry {
    name: CString,
    identity: SupportIdentity,
}
#[derive(Debug)]
struct HeldDirectory {
    path: PathBuf,
    file: File,
    identity: SupportIdentity,
    entries: Vec<Entry>,
    complete: bool,
}
enum Mismatch {
    Observation(Fault),
    Order,
}
impl From<Fault> for Mismatch {
    fn from(fault: Fault) -> Self {
        Self::Observation(fault)
    }
}

fn entry_identity(file: &File, name: &CStr) -> io::Result<SupportIdentity> {
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: file and terminated name remain live; value is writable storage.
    let status = unsafe {
        libc::fstatat(
            file.as_raw_fd(),
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful fstatat initialized the stat structure.
    let value = unsafe { value.assume_init() };
    Ok(SupportIdentity {
        device: value.st_dev,
        inode: value.st_ino,
        size: u64::try_from(value.st_size).map_err(|_| io::ErrorKind::InvalidData)?,
        mode: value.st_mode,
        mtime_seconds: value.st_mtime,
        mtime_nanoseconds: value.st_mtime_nsec,
        ctime_seconds: value.st_ctime,
        ctime_nanoseconds: value.st_ctime_nsec,
    })
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    roots: Vec<PathBuf>,
    directories: Vec<HeldDirectory>,
    members: usize,
}
impl Snapshot {
    pub(crate) fn member_count(&self) -> usize {
        self.members
    }

    pub(crate) fn capture(roots: &[PathBuf]) -> Result<Self, Fault> {
        let baseline = Baseline::capture(roots)?;
        let mut directories = Vec::new();
        let is_directory =
            |identity: &SupportIdentity| identity.mode & libc::S_IFMT == libc::S_IFDIR;
        for (path, identity) in baseline.members.iter().filter(|(_, id)| is_directory(id)) {
            let file = open_directory(path).map_err(|e| io_fault(path, &e))?;
            let actual = SupportIdentity::from(&file.metadata().map_err(|e| io_fault(path, &e))?);
            if actual != *identity {
                return Err(Fault::Changed {
                    path: path.clone(),
                    expected: *identity,
                    actual: Some(actual),
                });
            }
            let complete = roots.iter().any(|root| path.starts_with(root));
            let mut children: BTreeMap<_, _> = baseline
                .members
                .iter()
                .filter(|(member, _)| member.parent() == Some(path.as_path()))
                .map(|(member, id)| (member.file_name().expect("child name").to_owned(), *id))
                .collect();
            let mut entries = Vec::new();
            let mut unexpected = Vec::new();
            if complete {
                let mut stream = Directory::open(&file).map_err(|e| io_fault(path, &e))?;
                while let Some(name) = stream.next().map_err(|e| io_fault(path, &e))? {
                    let spelling = std::ffi::OsStr::from_bytes(name.to_bytes());
                    if let Some(identity) = children.remove(spelling) {
                        entries.push(Entry {
                            name: name.to_owned(),
                            identity,
                        });
                    } else {
                        unexpected.push(path.join(spelling));
                    }
                }
                if !children.is_empty() || !unexpected.is_empty() {
                    unexpected.sort();
                    return Err(Fault::Inventory {
                        missing: children.keys().map(|name| path.join(name)).collect(),
                        unexpected,
                    });
                }
            } else {
                for (name, identity) in children {
                    entries.push(Entry {
                        name: CString::new(name.as_bytes())
                            .map_err(|e| io_fault(path, &e.into()))?,
                        identity,
                    });
                }
            }
            directories.push(HeldDirectory {
                path: path.clone(),
                file,
                identity: *identity,
                entries,
                complete,
            });
        }
        Ok(Self {
            roots: roots.to_vec(),
            directories,
            members: baseline.members.len(),
        })
    }

    fn compare_entry(directory: &HeldDirectory, entry: &Entry, name: &CStr) -> Result<(), Fault> {
        let actual = entry_identity(&directory.file, name).map_err(|e| {
            io_fault(
                &directory
                    .path
                    .join(std::ffi::OsStr::from_bytes(name.to_bytes())),
                &e,
            )
        })?;
        if actual == entry.identity {
            return Ok(());
        }
        Err(Fault::Changed {
            path: directory
                .path
                .join(std::ffi::OsStr::from_bytes(name.to_bytes())),
            expected: entry.identity,
            actual: Some(actual),
        })
    }

    fn streamed_check(&self) -> Result<(), Mismatch> {
        for directory in &self.directories {
            let actual = SupportIdentity::from(
                &directory
                    .file
                    .metadata()
                    .map_err(|e| io_fault(&directory.path, &e))?,
            );
            if actual != directory.identity {
                return Err(Fault::Changed {
                    path: directory.path.clone(),
                    expected: directory.identity,
                    actual: Some(actual),
                }
                .into());
            }
            if directory.complete {
                let mut stream =
                    Directory::open(&directory.file).map_err(|e| io_fault(&directory.path, &e))?;
                let mut expected = directory.entries.iter();
                while let Some(name) = stream.next().map_err(|e| io_fault(&directory.path, &e))? {
                    let Some(entry) = expected.next() else {
                        return Err(Mismatch::Order);
                    };
                    if entry.name.as_c_str() != name {
                        return Err(Mismatch::Order);
                    }
                    Self::compare_entry(directory, entry, name)?;
                }
                if expected.next().is_some() {
                    return Err(Mismatch::Order);
                }
            } else {
                for entry in &directory.entries {
                    Self::compare_entry(directory, entry, &entry.name)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn check(&self) -> Result<usize, Fault> {
        let Err(mismatch) = self.streamed_check() else {
            return Ok(self.members);
        };
        // Reconstruct only on refusal, preserving the established complete
        // inventory and member-before-ancestor diagnostic ordering. Directory
        // enumeration order itself is not an integrity property.
        let mut members = BTreeMap::new();
        for directory in &self.directories {
            members.insert(directory.path.clone(), directory.identity);
            for entry in &directory.entries {
                members.insert(
                    directory
                        .path
                        .join(std::ffi::OsStr::from_bytes(entry.name.to_bytes())),
                    entry.identity,
                );
            }
        }
        Baseline {
            roots: self.roots.clone(),
            members,
        }
        .check()?;
        match mismatch {
            Mismatch::Observation(fault) => Err(fault),
            Mismatch::Order => Ok(self.members),
        }
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
