//! A one-shot pre-exec setup. All allocation and path construction precedes fork.
//! The child uses fixed buffers, libc calls and raw syscalls only. Its status pipe
//! carries a fixed record; neither failure nor missing setup evidence permits exec.
//! Consumers cannot write checked inputs through mounts, procfs aliases or
//! inherited descriptors. External writers remain subject to bracket observations.
//! Same-user terminal output and host-denied outer queue masks are stated limits;
//! a minimal private device directory is their successor. A PID namespace is the
//! successor for other same-user availability changes and surviving descendants.
//! Namespace fixture controllers require `CAP_SYS_ADMIN` in an outer user namespace:
//! run test gates inside `unshare --user --map-root-user --net` before Cargo. The
//! production direct-entry branch is exercised separately inside those gates.
//! Other operating systems or architectures fail to build; no unfiltered ABI is supported.
use std::{
    ffi::CString,
    fs::{self, File},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConfinementOperation {
    CreateNamespace,
    WriteIdentityMaps,
    IsolateNetwork,
    MakePrivate,
    Bind,
    RemountReadOnly,
    RemountWritable,
    InspectMount,
    CloseWritableDescriptors,
    DropPrivileges,
}
use ConfinementOperation as Op;

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("consumer confinement requires the Linux x86-64 syscall ABI");

// Classic BPF over seccomp_data: nr at 0, arch at 4, low word of args[0] at 16.
// Reject foreign audit architectures and nr >= 512 (including x32 and its old
// untagged syscall range). EACCES is fixed for every refusal. io_uring is denied
// because its socket operations otherwise bypass syscall-level socket filters.
const fn statement(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}
const fn equal(k: u32, jf: u8) -> libc::sock_filter {
    libc::sock_filter {
        code: 0x15,
        jt: 0,
        jf,
        k,
    }
}
const DENY: libc::sock_filter = statement(0x06, 0x0005_000d);
const ALLOW: libc::sock_filter = statement(0x06, 0x7fff_0000);
static FILTER: [libc::sock_filter; 59] = [
    statement(0x20, 4),
    libc::sock_filter {
        code: 0x15,
        jt: 1,
        jf: 0,
        k: 0xc000_003e,
    },
    DENY,
    statement(0x20, 0),
    libc::sock_filter {
        code: 0x35,
        jt: 0,
        jf: 1,
        k: 512,
    },
    DENY,
    equal(42, 1),
    DENY, // connect, any arguments
    equal(49, 1),
    DENY, // bind, any arguments
    equal(425, 1),
    DENY, // io_uring_setup
    equal(426, 1),
    DENY, // io_uring_enter
    equal(427, 1),
    DENY, // io_uring_register
    equal(101, 1),
    DENY, // ptrace
    equal(310, 1),
    DENY, // process_vm_readv
    equal(311, 1),
    DENY, // process_vm_writev
    equal(438, 1),
    DENY, // pidfd_getfd
    equal(440, 1),
    DENY,          // process_madvise
    equal(302, 4), // prlimit64: the kernel converts pid_t to a 32-bit int
    statement(0x20, 16),
    equal(0, 1),
    ALLOW,
    DENY,
    equal(16, 6), // ioctl: request is an unsigned int
    statement(0x20, 24),
    equal(0x5412, 1),
    DENY, // TIOCSTI
    equal(0x541c, 1),
    DENY, // TIOCLINUX
    ALLOW,
    equal(41, 4), // socket
    statement(0x20, 16),
    equal(1, 1),
    DENY, // AF_UNIX, irrespective of type/protocol
    ALLOW,
    equal(53, 7), // socketpair: strip only SOCK_CLOEXEC and SOCK_NONBLOCK
    statement(0x20, 24),
    statement(0x54, 0xfff7_f7ff),
    equal(1, 1),
    ALLOW, // SOCK_STREAM
    equal(5, 1),
    ALLOW, // SOCK_SEQPACKET
    DENY,
    equal(44, 6), // sendto: both halves of args[4] must be zero
    statement(0x20, 48),
    equal(0, 3),
    statement(0x20, 52),
    equal(0, 1),
    ALLOW,
    DENY,
    ALLOW,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Fault {
    pub operation: Op,
    pub path: PathBuf,
    pub kind: io::ErrorKind,
}
impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Fault {}

fn setup_io<T>(operation: Op, path: &Path, result: io::Result<T>) -> io::Result<T> {
    result.map_err(|error| {
        io::Error::new(
            error.kind(),
            Fault {
                operation,
                path: path.to_owned(),
                kind: error.kind(),
            },
        )
    })
}

impl From<Fault> for crate::NativeAdapterFault {
    fn from(fault: Fault) -> Self {
        Self::Confinement {
            operation: fault.operation,
            path: fault.path,
            kind: fault.kind,
        }
    }
}
impl From<Fault> for crate::NotationAdapterFault {
    fn from(fault: Fault) -> Self {
        Self::Confinement {
            operation: fault.operation,
            path: fault.path,
            kind: fault.kind,
        }
    }
}

#[cfg(test)]
thread_local! {
    static INJECTION: std::cell::Cell<Option<Op>> = const { std::cell::Cell::new(None) };
    static DIRECT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FILTER_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OPTIONAL_MASK_ERROR: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
    static OPTIONAL_MASK_TARGET: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    static LAST_MASK: std::cell::Cell<MaskOutcome> = const { std::cell::Cell::new(MaskOutcome::Absent) };
    static RELAY_WRITE_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn with_fault<T>(operation: Op, run: impl FnOnce() -> T) -> T {
    struct Reset(Option<Op>);
    impl Drop for Reset {
        fn drop(&mut self) {
            INJECTION.set(self.0);
        }
    }
    let _reset = Reset(INJECTION.replace(Some(operation)));
    run()
}

#[derive(Clone, Copy)]
struct Failure {
    operation: Op,
    path: usize,
    errno: i32,
}
type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaskClass {
    Required,
    HostPermitting,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum MaskOutcome {
    Absent,
    Applied,
    PermissionDenied,
}
fn mask_result(class: MaskClass, result: Result<()>) -> Result<MaskOutcome> {
    match result {
        Ok(()) => Ok(MaskOutcome::Applied),
        Err(error)
            if class == MaskClass::HostPermitting
                && matches!(error.errno, libc::EACCES | libc::EPERM) =>
        {
            Ok(MaskOutcome::PermissionDenied)
        }
        Err(error) => Err(error),
    }
}

pub(crate) struct Scratch(PathBuf);
impl Scratch {
    pub(crate) fn new(parent: &Path) -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = parent.join(format!(
            "consumer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Node writes optional records inside its fresh writable scratch. The controller
// relays complete bytes afterwards; neither preparation nor relay failure changes
// the consumer result. No arbitrary measurement directory becomes a writable bind.
pub(crate) struct Telemetry {
    source: Option<File>,
    destination: Option<PathBuf>,
}
// A telemetry file is optional evidence, never consumer output. Drop an entire
// file larger than 16 MiB; reading one extra byte distinguishes overflow from EOF.
const TELEMETRY_LIMIT: usize = 16 * 1024 * 1024;
impl Telemetry {
    pub(crate) fn new(command: &mut Command, scratch: &Path) -> Self {
        Self::at(
            command,
            scratch,
            std::env::var_os("FTOO_GATE_TELEMETRY").map(PathBuf::from),
        )
    }
    fn at(command: &mut Command, scratch: &Path, destination: Option<PathBuf>) -> Self {
        let source = scratch.join("telemetry");
        let destination = destination
            .filter(|path| path.is_dir())
            .filter(|_| fs::create_dir(&source).is_ok());
        command.env_remove("FTOO_GATE_TELEMETRY");
        if destination.is_some() {
            command.env("FTOO_GATE_TELEMETRY", &source);
        }
        Self {
            source: fs::File::open(&source).ok(),
            destination,
        }
    }
    fn relay(&self) -> io::Result<()> {
        let Some(destination) = &self.destination else {
            return Ok(());
        };
        let Some(source) = &self.source else {
            return Ok(());
        };
        // Enumerate the pinned directory, then open relative to that descriptor.
        // Neither replacement of the directory nor a leaf symlink is followed.
        let entries = fs::read_dir(format!("/proc/self/fd/{}", source.as_raw_fd()))?;
        for entry in entries.flatten() {
            let Ok(name) = CString::new(entry.file_name().as_bytes()) else {
                continue;
            };
            // SAFETY: the directory descriptor and terminated name are valid.
            let fd = unsafe {
                libc::openat(
                    source.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                continue;
            }
            // SAFETY: openat returned a new owned descriptor.
            let input = unsafe { File::from_raw_fd(fd) };
            let mut bytes = Vec::new();
            if input.metadata().is_ok_and(|metadata| metadata.is_file())
                && input
                    .take((TELEMETRY_LIMIT + 1) as u64)
                    .read_to_end(&mut bytes)
                    .is_ok()
                && bytes.len() <= TELEMETRY_LIMIT
            {
                Self::publish(destination, &bytes)?;
            }
        }
        Ok(())
    }
    fn publish(destination: &Path, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "relay-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let temporary = destination.join(format!(".{name}.tmp"));
        let final_path = destination.join(format!("{name}.jsonl"));
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| {
            #[cfg(test)]
            if RELAY_WRITE_FAILURE.get() {
                output.write_all(&bytes[..bytes.len() / 2])?;
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            output.write_all(bytes)?;
            let from = CString::new(temporary.as_os_str().as_bytes())?;
            let to = CString::new(final_path.as_os_str().as_bytes())?;
            // SAFETY: both terminated paths remain live throughout renameat2.
            // NOREPLACE preserves existing records, including after PID reuse.
            if unsafe {
                libc::renameat2(
                    libc::AT_FDCWD,
                    from.as_ptr(),
                    libc::AT_FDCWD,
                    to.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        drop(output);
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}
impl Drop for Telemetry {
    fn drop(&mut self) {
        let _ = self.relay();
    }
}

struct Plan {
    paths: Vec<CString>,
    writable: Vec<usize>,
    proc_views: Vec<usize>,
    masks: Vec<usize>,
    optional_mask: Option<usize>,
    optional_mask_error: Option<i32>,
    mask: usize,
    mask_identity: (u64, u64),
    cwd: Option<usize>,
    direct: bool,
    uid: Vec<u8>,
    gid: Vec<u8>,
    last_cap: u32,
    inject: Option<Op>,
    filter_failure: bool,
}
impl Plan {
    fn new(writable: &[&Path], mask: &Path, inject: Option<Op>) -> io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let mut plan = Self {
            paths: [
                "/",
                "/proc/self/uid_map",
                "/proc/self/gid_map",
                "/proc/self/setgroups",
                "/proc/net/dev",
                "/proc/self/mountinfo",
                "/proc/self/fd",
            ]
            .into_iter()
            .map(|s| CString::new(s).unwrap())
            .collect(),
            writable: vec![],
            proc_views: vec![],
            masks: vec![],
            optional_mask: None,
            #[cfg(test)]
            optional_mask_error: OPTIONAL_MASK_ERROR.get(),
            #[cfg(not(test))]
            optional_mask_error: None,
            mask: 0,
            mask_identity: (0, 0),
            cwd: None,
            direct: setup_io(
                Op::WriteIdentityMaps,
                Path::new("/proc/self/uid_map"),
                fs::read_to_string("/proc/self/uid_map"),
            )?
            .split_whitespace()
            .collect::<Vec<_>>()
                == ["0", "0", "4294967295"],
            // SAFETY: these process identity queries have no preconditions.
            uid: format!("0 {} 1\n", unsafe { libc::geteuid() }).into_bytes(),
            gid: format!("0 {} 1\n", unsafe { libc::getegid() }).into_bytes(),
            last_cap: setup_io(
                Op::DropPrivileges,
                Path::new("/proc/sys/kernel/cap_last_cap"),
                fs::read_to_string("/proc/sys/kernel/cap_last_cap").and_then(|text| {
                    text.trim()
                        .parse()
                        .map_err(|_| io::ErrorKind::InvalidData.into())
                }),
            )?,
            inject,
            filter_failure: false,
        };
        #[cfg(test)]
        if DIRECT.get() {
            plan.direct = true;
        }
        #[cfg(test)]
        {
            plan.filter_failure = FILTER_FAILURE.get();
        }
        plan.mask = plan.path(mask)?;
        let metadata = setup_io(Op::Bind, mask, fs::metadata(mask))?;
        plan.mask_identity = (metadata.dev(), metadata.ino());
        for path in writable {
            let index = plan.path(&setup_io(Op::Bind, path, path.canonicalize())?)?;
            plan.writable.push(index);
        }
        // Mask every procfs alias, not just the conventional /proc spelling.
        let mount_path = Path::new("/proc/self/mountinfo");
        let mounts = setup_io(Op::InspectMount, mount_path, fs::read_to_string(mount_path))?;
        let mut views = vec![];
        for line in mounts.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            let separator = setup_io(
                Op::InspectMount,
                mount_path,
                fields
                    .iter()
                    .position(|s| *s == "-")
                    .ok_or_else(|| io::ErrorKind::InvalidData.into()),
            )?;
            if fields.get(separator + 1) == Some(&"proc") {
                let field = setup_io(
                    Op::InspectMount,
                    mount_path,
                    fields
                        .get(4)
                        .ok_or_else(|| io::ErrorKind::InvalidData.into()),
                )?;
                let path = setup_io(Op::InspectMount, mount_path, decode(field.as_bytes()))?;
                views.push(PathBuf::from(std::ffi::OsStr::from_bytes(&path)));
            }
        }
        views.sort_by_key(|path| path.components().count());
        let mut roots = Vec::<PathBuf>::new();
        for view in views {
            if !roots.iter().any(|root| view.starts_with(root)) {
                let index = plan.path(&view)?;
                plan.proc_views.push(index);
                plan.masks.push(index);
                roots.push(view);
            }
        }
        plan.runtime_masks(&mut roots)?;
        Ok(plan)
    }
    fn runtime_masks(&mut self, roots: &mut Vec<PathBuf>) -> io::Result<()> {
        for path in ["/run/user", "/var/run/user"] {
            let path = Path::new(path);
            let canonical = match path.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return setup_io(Op::Bind, path, Err(error)),
            };
            if !roots.iter().any(|root| canonical.starts_with(root)) {
                let index = self.path(&canonical)?;
                self.masks.push(index);
                roots.push(canonical);
            }
        }
        #[cfg(test)]
        let target = OPTIONAL_MASK_TARGET.with_borrow(Clone::clone);
        #[cfg(not(test))]
        let target: Option<PathBuf> = None;
        let path = target.as_deref().unwrap_or(Path::new("/dev/mqueue"));
        match path.canonicalize() {
            Ok(canonical) => {
                if !roots.iter().any(|root| canonical.starts_with(root)) {
                    self.optional_mask = Some(self.path(&canonical)?);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return setup_io(Op::Bind, path, Err(error)),
        }
        Ok(())
    }
    fn path(&mut self, path: &Path) -> io::Result<usize> {
        let index = self.paths.len();
        self.paths.push(setup_io(
            Op::Bind,
            path,
            CString::new(path.as_os_str().as_bytes())
                .map_err(|_| io::ErrorKind::InvalidInput.into()),
        )?);
        Ok(index)
    }
    fn check(&self, operation: Op, path: usize, status: i64) -> Result<()> {
        if self.inject == Some(operation) {
            return Err(Failure {
                operation,
                path,
                errno: libc::EACCES,
            });
        }
        if status < 0 {
            // SAFETY: errno is thread-local and is read immediately after failure.
            return Err(Failure {
                operation,
                path,
                errno: unsafe { *libc::__errno_location() },
            });
        }
        Ok(())
    }
    fn seam(&self, operation: Op, path: usize) -> Result<()> {
        self.check(operation, path, 0)
    }
    fn failure(operation: Op, path: usize, errno: i32) -> Failure {
        Failure {
            operation,
            path,
            errno,
        }
    }
    fn fault(&self, failure: Failure) -> Fault {
        Fault {
            operation: failure.operation,
            path: PathBuf::from(std::ffi::OsStr::from_bytes(
                self.paths[failure.path].to_bytes(),
            )),
            kind: io::Error::from_raw_os_error(failure.errno).kind(),
        }
    }
    fn write_map(&self, index: usize, bytes: &[u8]) -> Result<()> {
        // SAFETY: strings and slices remain valid throughout each syscall.
        unsafe {
            let fd = libc::open(self.paths[index].as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
            self.check(Op::WriteIdentityMaps, index, i64::from(fd))?;
            let status = libc::write(fd, bytes.as_ptr().cast(), bytes.len());
            let checked = self.check(Op::WriteIdentityMaps, index, status as i64);
            libc::close(fd);
            checked?;
            if status.cast_unsigned() != bytes.len() {
                return Err(Self::failure(Op::WriteIdentityMaps, index, libc::EIO));
            }
        }
        Ok(())
    }
    fn read_file(&self, index: usize, operation: Op, buffer: &mut [u8]) -> Result<usize> {
        // SAFETY: open/read/close use valid storage and an owned descriptor.
        unsafe {
            let fd = libc::open(self.paths[index].as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
            self.check(operation, index, i64::from(fd))?;
            let result =
                read_all(fd, buffer).map_err(|errno| Self::failure(operation, index, errno));
            libc::close(fd);
            result
        }
    }
    #[allow(clippy::too_many_lines)] // One ordered, fail-closed pre-exec contract.
    #[allow(clippy::large_stack_arrays)] // Allocation is forbidden after fork; overflow fails closed.
    fn enter(&self, optional_mask: &mut MaskOutcome) -> Result<()> {
        #[repr(C)]
        struct Header {
            version: u32,
            pid: i32,
        }
        #[repr(C)]
        struct Data {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }
        let mut buffer = [0_u8; 262_144];
        // SAFETY: every pointer refers to preallocated storage; all calls below
        // are libc wrappers or raw Linux syscalls, with no Rust allocation/locks.
        unsafe {
            self.seam(Op::CreateNamespace, 0)?;
            self.check(
                Op::CreateNamespace,
                0,
                i64::from(libc::unshare(
                    libc::CLONE_NEWNS
                        | libc::CLONE_NEWIPC
                        | if self.direct { libc::CLONE_NEWUSER } else { 0 },
                )),
            )?;
            self.seam(Op::WriteIdentityMaps, 1)?;
            if self.direct {
                self.write_map(1, &self.uid)?;
                self.write_map(3, b"deny\n")?;
                self.write_map(2, &self.gid)?;
            }
            self.seam(Op::IsolateNetwork, 4)?;
            if self.direct {
                self.check(
                    Op::IsolateNetwork,
                    4,
                    i64::from(libc::unshare(libc::CLONE_NEWNET)),
                )?;
            }
            let length = self.read_file(4, Op::IsolateNetwork, &mut buffer)?;
            for line in buffer[..length].split(|b| *b == b'\n') {
                if let Some(colon) = line.iter().position(|b| *b == b':')
                    && trim(&line[..colon]) != b"lo"
                {
                    return Err(Self::failure(Op::IsolateNetwork, 4, libc::EPERM));
                }
            }
            self.seam(Op::MakePrivate, 0)?;
            self.check(
                Op::MakePrivate,
                0,
                i64::from(libc::mount(
                    std::ptr::null(),
                    c"/".as_ptr(),
                    std::ptr::null(),
                    libc::MS_REC | libc::MS_PRIVATE,
                    std::ptr::null(),
                )),
            )?;
            self.seam(Op::RemountReadOnly, 0)?;
            let attr = libc::mount_attr {
                attr_set: libc::MOUNT_ATTR_RDONLY,
                attr_clr: 0,
                propagation: 0,
                userns_fd: 0,
            };
            self.check(
                Op::RemountReadOnly,
                0,
                libc::syscall(
                    libc::SYS_mount_setattr,
                    libc::AT_FDCWD,
                    c"/".as_ptr(),
                    libc::AT_RECURSIVE,
                    &raw const attr,
                    std::mem::size_of_val(&attr),
                ),
            )?;
            // Open after unshare, before masking: this descriptor reports this
            // child's mount namespace, including the masks added below.
            self.seam(Op::InspectMount, 5)?;
            let mounts = libc::open(
                c"/proc/self/mountinfo".as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC,
            );
            self.check(Op::InspectMount, 5, i64::from(mounts))?;
            let descriptors = libc::open(
                c"/proc/self/fd".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            );
            self.check(Op::CloseWritableDescriptors, 6, i64::from(descriptors))?;
            let mounted = (|| {
                for &index in &self.masks {
                    self.seam(Op::Bind, index)?;
                    mask_result(
                        MaskClass::Required,
                        self.check(
                            Op::Bind,
                            index,
                            i64::from(libc::mount(
                                self.paths[self.mask].as_ptr(),
                                self.paths[index].as_ptr(),
                                std::ptr::null(),
                                libc::MS_BIND,
                                std::ptr::null(),
                            )),
                        ),
                    )?;
                    let mut observed: libc::stat = std::mem::zeroed();
                    self.check(
                        Op::InspectMount,
                        index,
                        i64::from(libc::stat(self.paths[index].as_ptr(), &raw mut observed)),
                    )?;
                    if (observed.st_dev, observed.st_ino) != self.mask_identity {
                        return Err(Self::failure(Op::InspectMount, index, libc::EPERM));
                    }
                }
                if let Some(index) = self.optional_mask {
                    let result = if let Some(errno) = self.optional_mask_error {
                        Err(Self::failure(Op::Bind, index, errno))
                    } else {
                        self.check(
                            Op::Bind,
                            index,
                            i64::from(libc::mount(
                                self.paths[self.mask].as_ptr(),
                                self.paths[index].as_ptr(),
                                std::ptr::null(),
                                libc::MS_BIND,
                                std::ptr::null(),
                            )),
                        )
                    };
                    *optional_mask = mask_result(MaskClass::HostPermitting, result)?;
                    if *optional_mask == MaskOutcome::Applied {
                        let mut observed: libc::stat = std::mem::zeroed();
                        self.check(
                            Op::InspectMount,
                            index,
                            i64::from(libc::stat(self.paths[index].as_ptr(), &raw mut observed)),
                        )?;
                        if (observed.st_dev, observed.st_ino) != self.mask_identity {
                            return Err(Self::failure(Op::InspectMount, index, libc::EPERM));
                        }
                    }
                }
                for &index in &self.writable {
                    self.seam(Op::Bind, index)?;
                    self.check(
                        Op::Bind,
                        index,
                        i64::from(libc::mount(
                            self.paths[index].as_ptr(),
                            self.paths[index].as_ptr(),
                            std::ptr::null(),
                            libc::MS_BIND,
                            std::ptr::null(),
                        )),
                    )?;
                    self.seam(Op::RemountWritable, index)?;
                    self.check(
                        Op::RemountWritable,
                        index,
                        i64::from(libc::mount(
                            std::ptr::null(),
                            self.paths[index].as_ptr(),
                            std::ptr::null(),
                            libc::MS_BIND | libc::MS_REMOUNT | libc::MS_NOSUID | libc::MS_NODEV,
                            std::ptr::null(),
                        )),
                    )?;
                }
                // Re-enter only an explicitly selected working directory after
                // binds; choosing writable paths never implicitly changes cwd.
                if let Some(index) = self.cwd {
                    self.check(
                        Op::Bind,
                        index,
                        i64::from(libc::chdir(self.paths[index].as_ptr())),
                    )?;
                }
                let length = read_all(mounts, &mut buffer)
                    .map_err(|errno| Self::failure(Op::InspectMount, 5, errno))?;
                self.inspect(&buffer[..length], *optional_mask)
            })();
            libc::close(mounts);
            mounted?;
            self.seam(Op::CloseWritableDescriptors, 6)?;
            let closed = close_inherited(descriptors)
                .map_err(|errno| Self::failure(Op::CloseWritableDescriptors, 6, errno));
            libc::close(descriptors);
            closed?;
            // All inheritable descriptors above stdio are closed now. Also mark
            // every remaining internal descriptor close-on-exec, including the
            // setup report and the standard library's exec-error pipe.
            self.check(
                Op::CloseWritableDescriptors,
                6,
                libc::syscall(
                    libc::SYS_close_range,
                    3_u32,
                    u32::MAX,
                    libc::CLOSE_RANGE_CLOEXEC,
                ),
            )?;
            self.seam(Op::DropPrivileges, 0)?;
            for capability in 0..=self.last_cap {
                self.check(
                    Op::DropPrivileges,
                    0,
                    i64::from(libc::prctl(libc::PR_CAPBSET_DROP, capability, 0, 0, 0)),
                )?;
            }
            self.check(
                Op::DropPrivileges,
                0,
                i64::from(libc::prctl(libc::PR_SET_SECUREBITS, 0x2f, 0, 0, 0)),
            )?;
            let header = Header {
                version: 0x2008_0522,
                pid: 0,
            };
            let data = [
                Data {
                    effective: 0,
                    permitted: 0,
                    inheritable: 0,
                },
                Data {
                    effective: 0,
                    permitted: 0,
                    inheritable: 0,
                },
            ];
            self.check(
                Op::DropPrivileges,
                0,
                libc::syscall(libc::SYS_capset, &raw const header, data.as_ptr()),
            )?;
            self.check(
                Op::DropPrivileges,
                0,
                i64::from(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)),
            )?;
            if self.filter_failure {
                return Err(Self::failure(Op::DropPrivileges, 0, libc::EACCES));
            }
            let filter = libc::sock_fprog {
                len: u16::try_from(FILTER.len()).unwrap(),
                filter: FILTER.as_ptr().cast_mut(),
            };
            self.check(
                Op::DropPrivileges,
                0,
                i64::from(libc::prctl(
                    libc::PR_SET_SECCOMP,
                    libc::SECCOMP_MODE_FILTER,
                    &raw const filter,
                )),
            )?;
        }
        Ok(())
    }
    fn inspect(&self, bytes: &[u8], optional_mask: MaskOutcome) -> Result<()> {
        let mut root_seen = false;
        for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
            let mut fields = line.split(|b| *b == b' ');
            let path = fields
                .nth(4)
                .ok_or(Self::failure(Op::InspectMount, 5, libc::EINVAL))?;
            root_seen |= path == b"/";
            let flags = fields
                .next()
                .ok_or(Self::failure(Op::InspectMount, 5, libc::EINVAL))?;
            let filesystem = fields
                .find(|field| *field == b"-")
                .and_then(|_| fields.next())
                .ok_or(Self::failure(Op::InspectMount, 5, libc::EINVAL))?;
            if filesystem == b"proc"
                && !self.proc_views.iter().any(|&index| {
                    strip_encoded_prefix(path, self.paths[index].to_bytes())
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with(b"/"))
                })
            {
                return Err(Self::failure(Op::InspectMount, 5, libc::EPERM));
            }
            let writable = flags.split(|b| *b == b',').any(|flag| flag == b"rw");
            if writable
                && !self
                    .writable
                    .iter()
                    .any(|&index| encoded_equal(path, self.paths[index].to_bytes()))
            {
                return Err(Self::failure(Op::InspectMount, 5, libc::EPERM));
            }
            if !writable && !flags.split(|b| *b == b',').any(|flag| flag == b"ro") {
                return Err(Self::failure(Op::InspectMount, 5, libc::EINVAL));
            }
        }
        if !root_seen {
            return Err(Self::failure(Op::InspectMount, 5, libc::EINVAL));
        }
        for index in self.masks.iter().copied().chain(
            self.optional_mask
                .filter(|_| optional_mask == MaskOutcome::Applied),
        ) {
            let present = bytes.split(|b| *b == b'\n').any(|line| {
                let mut fields = line.split(|b| *b == b' ');
                let Some(path) = fields.nth(4) else {
                    return false;
                };
                let Some(flags) = fields.next() else {
                    return false;
                };
                let filesystem = fields
                    .find(|field| *field == b"-")
                    .and_then(|_| fields.next());
                encoded_equal(path, self.paths[index].to_bytes())
                    && flags.split(|b| *b == b',').any(|flag| flag == b"ro")
                    && filesystem.is_some_and(|kind| kind != b"proc")
            });
            if !present {
                return Err(Self::failure(Op::InspectMount, 5, libc::EPERM));
            }
        }
        Ok(())
    }
}

fn trim(mut bytes: &[u8]) -> &[u8] {
    while bytes.first() == Some(&b' ') {
        bytes = &bytes[1..];
    }
    while bytes.last() == Some(&b' ') {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}
fn encoded_equal(encoded: &[u8], plain: &[u8]) -> bool {
    strip_encoded_prefix(encoded, plain).is_some_and(<[u8]>::is_empty)
}
fn strip_encoded_prefix<'a>(mut encoded: &'a [u8], plain: &[u8]) -> Option<&'a [u8]> {
    for &byte in plain {
        if encoded.first() == Some(&b'\\') {
            if encoded.len() < 4
                || !(b'0'..=b'3').contains(&encoded[1])
                || !encoded[1..4].iter().all(|b| (b'0'..=b'7').contains(b))
            {
                return None;
            }
            let value = (encoded[1] - b'0') * 64 + (encoded[2] - b'0') * 8 + encoded[3] - b'0';
            if value != byte {
                return None;
            }
            encoded = &encoded[4..];
        } else {
            if encoded.first() != Some(&byte) {
                return None;
            }
            encoded = &encoded[1..];
        }
    }
    Some(encoded)
}
fn decode(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut result = vec![];
    let mut rest = bytes;
    while !rest.is_empty() {
        if rest[0] == b'\\' {
            if rest.len() < 4
                || !(b'0'..=b'3').contains(&rest[1])
                || !rest[1..4].iter().all(|b| (b'0'..=b'7').contains(b))
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            result.push((rest[1] - b'0') * 64 + (rest[2] - b'0') * 8 + rest[3] - b'0');
            rest = &rest[4..];
        } else {
            result.push(rest[0]);
            rest = &rest[1..];
        }
    }
    Ok(result)
}
fn read_all(fd: i32, buffer: &mut [u8]) -> std::result::Result<usize, i32> {
    let mut used = 0;
    loop {
        if used == buffer.len() {
            return Err(libc::EOVERFLOW);
        }
        // SAFETY: the remaining buffer is writable for the supplied length.
        let count =
            unsafe { libc::read(fd, buffer[used..].as_mut_ptr().cast(), buffer.len() - used) };
        if count == 0 {
            return Ok(used);
        }
        if count < 0 {
            let errno = unsafe { *libc::__errno_location() };
            if errno == libc::EINTR {
                continue;
            }
            return Err(errno);
        }
        used += count.cast_unsigned();
    }
}

fn close_inherited(directory: i32) -> std::result::Result<(), i32> {
    let mut buffer = [0_u8; 4096];
    loop {
        // SAFETY: getdents64 writes Linux directory records into the fixed buffer.
        let count = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                directory,
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };
        if count == 0 {
            return Ok(());
        }
        if count < 0 {
            let errno = unsafe { *libc::__errno_location() };
            if errno == libc::EINTR {
                continue;
            }
            return Err(errno);
        }
        let count = usize::try_from(count).map_err(|_| libc::EOVERFLOW)?;
        let mut records = &buffer[..count];
        while !records.is_empty() {
            if records.len() < 20 {
                return Err(libc::EINVAL);
            }
            let length = usize::from(u16::from_ne_bytes([records[16], records[17]]));
            if length < 20 || length > records.len() {
                return Err(libc::EINVAL);
            }
            let name = &records[19..length];
            let end = name.iter().position(|b| *b == 0).ok_or(libc::EINVAL)?;
            let name = &name[..end];
            if !name.is_empty() && name.iter().all(u8::is_ascii_digit) {
                let descriptor = name
                    .iter()
                    .try_fold(0_i32, |value, digit| {
                        value.checked_mul(10)?.checked_add(i32::from(*digit - b'0'))
                    })
                    .ok_or(libc::EINVAL)?;
                if descriptor > 2 && descriptor != directory {
                    // SAFETY: each descriptor is queried before closing; internal
                    // pipes have CLOEXEC and remain available until exec itself.
                    unsafe {
                        let flags = libc::fcntl(descriptor, libc::F_GETFD);
                        if flags < 0 {
                            return Err(*libc::__errno_location());
                        }
                        if flags & libc::FD_CLOEXEC == 0 && libc::close(descriptor) != 0 {
                            return Err(*libc::__errno_location());
                        }
                    }
                }
            }
            records = &records[length..];
        }
    }
}

#[cfg(test)]
pub(crate) fn spawn(command: &mut Command, writable: &[&Path]) -> io::Result<Child> {
    spawn_measured(command, writable, "confinement.setup")
}

pub(crate) fn spawn_measured(
    command: &mut Command,
    writable: &[&Path],
    phase: &'static str,
) -> io::Result<Child> {
    #[cfg(test)]
    let inject = INJECTION.get();
    #[cfg(not(test))]
    let inject = None;
    let launch = crate::telemetry::Phase::new(match phase {
        "native.confinement.setup" => "native.confinement.launch",
        "notation.confinement.setup" => "notation.confinement.launch",
        _ => "confinement.launch",
    });
    let result = spawn_with(command, writable, inject, phase);
    launch.finish(result.is_ok());
    result
}
#[allow(clippy::too_many_lines)] // One child launch and its complete setup-status exchange.
fn spawn_with(
    command: &mut Command,
    writable: &[&Path],
    inject: Option<Op>,
    phase: &'static str,
) -> io::Result<Child> {
    let temporary = std::env::temp_dir();
    let mask = setup_io(Op::Bind, &temporary, Scratch::new(&temporary))?;
    let mut plan = Plan::new(writable, mask.path(), inject)?;
    if let Some(cwd) = command.get_current_dir() {
        let cwd = setup_io(Op::Bind, cwd, cwd.canonicalize())?;
        plan.cwd = Some(plan.path(&cwd)?);
    }
    let plan = Arc::new(plan);
    let mut pipes = [-1; 2];
    // SAFETY: pipe2 initializes two descriptors, subsequently owned by File.
    if unsafe { libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
        return setup_io(
            Op::CloseWritableDescriptors,
            Path::new("/proc/self/fd"),
            Err(io::Error::last_os_error()),
        );
    }
    let mut reader = unsafe { File::from_raw_fd(pipes[0]) };
    let writer = unsafe { File::from_raw_fd(pipes[1]) };
    let child_plan = Arc::clone(&plan);
    // SAFETY: enter and this closure use only preallocated storage and raw
    // syscalls. The owned writer remains alive until Command is dropped.
    unsafe {
        command.pre_exec(move || {
            // The first child operation detaches its controlling terminal. A
            // session is an isolation boundary, so failure is CreateNamespace.
            let session = libc::setsid();
            let session_errno = if session < 0 {
                *libc::__errno_location()
            } else {
                0
            };
            let mut start = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            // Session creation must be first, even before this timing probe.
            // The parent launch measurement includes it; this setup interval
            // measures the namespace, mount, descriptor and privilege sequence.
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut start);
            let mut optional_mask = MaskOutcome::Absent;
            let result = if session < 0 {
                Err(Plan::failure(Op::CreateNamespace, 0, session_errno))
            } else {
                child_plan.enter(&mut optional_mask)
            };
            let mut end = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut end);
            let elapsed = u64::try_from(
                (end.tv_sec - start.tv_sec) * 1_000_000_000 + end.tv_nsec - start.tv_nsec,
            )
            .unwrap_or_default();
            let mut record = [0_u8; 24];
            record[2] = optional_mask as u8;
            record[8..16].copy_from_slice(&elapsed.to_ne_bytes());
            if let Err(failure) = result {
                record[0] = 1;
                record[1] = failure.operation as u8;
                record[4..8].copy_from_slice(&failure.errno.to_ne_bytes());
                record[16..24].copy_from_slice(&(failure.path as u64).to_ne_bytes());
            }
            if libc::write(writer.as_raw_fd(), record.as_ptr().cast(), record.len()) != 24 {
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            result.map_err(|failure| io::Error::from_raw_os_error(failure.errno))
        });
    }
    let mut child = command.spawn();
    let mut record = [0; 24];
    if reader.read_exact(&mut record).is_ok() {
        let outcome = match record[2] {
            0 => MaskOutcome::Absent,
            1 => MaskOutcome::Applied,
            2 => MaskOutcome::PermissionDenied,
            _ => {
                if let Ok(child) = child.as_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let fault = Fault {
                    operation: Op::CreateNamespace,
                    path: "/".into(),
                    kind: io::ErrorKind::InvalidData,
                };
                return Err(io::Error::new(fault.kind, fault));
            }
        };
        #[cfg(test)]
        LAST_MASK.set(outcome);
        crate::telemetry::measurement(
            match outcome {
                MaskOutcome::Absent => "confinement.optional_mask.absent",
                MaskOutcome::Applied => "confinement.optional_mask.applied",
                MaskOutcome::PermissionDenied => "confinement.optional_mask.permission_denied",
            },
            0,
            true,
        );
        crate::telemetry::measurement(
            phase,
            u64::from_ne_bytes(record[8..16].try_into().unwrap()),
            record[0] == 0,
        );
        if record[0] != 0 {
            let operations = [
                Op::CreateNamespace,
                Op::WriteIdentityMaps,
                Op::IsolateNetwork,
                Op::MakePrivate,
                Op::Bind,
                Op::RemountReadOnly,
                Op::RemountWritable,
                Op::InspectMount,
                Op::CloseWritableDescriptors,
                Op::DropPrivileges,
            ];
            let failure = Failure {
                operation: operations[usize::from(record[1])],
                path: usize::try_from(u64::from_ne_bytes(record[16..24].try_into().unwrap()))
                    .unwrap(),
                errno: i32::from_ne_bytes(record[4..8].try_into().unwrap()),
            };
            let fault = plan.fault(failure);
            return Err(io::Error::new(fault.kind, fault));
        }
    } else if let Ok(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
        let fault = Fault {
            operation: Op::CreateNamespace,
            path: PathBuf::from("/"),
            kind: io::ErrorKind::InvalidData,
        };
        return Err(io::Error::new(fault.kind, fault));
    } else {
        return child;
    }
    child
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn confinement_optional_mask_policy_records_denials_and_required_masks_fail_closed() {
        let target = Scratch::new(&std::env::temp_dir()).unwrap();
        fs::write(target.path().join("input"), b"locked").unwrap();
        let command = || {
            let mut command = Command::new("/bin/sh");
            command
                .args([
                    "-c",
                    "if test -e \"$1/input\"; then printf visible; else printf hidden; fi",
                    "fixture",
                ])
                .arg(target.path())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            command
        };
        OPTIONAL_MASK_TARGET.set(Some(target.path().to_owned()));
        for errno in [libc::EACCES, libc::EPERM] {
            OPTIONAL_MASK_ERROR.set(Some(errno));
            let output = spawn(&mut command(), &[])
                .unwrap()
                .wait_with_output()
                .unwrap();
            OPTIONAL_MASK_ERROR.set(None);
            assert_eq!(LAST_MASK.get(), MaskOutcome::PermissionDenied);
            assert_eq!(output.status.code(), Some(0));
            assert_eq!(output.stdout, b"visible");
            assert_eq!(output.stderr, b"");
        }
        OPTIONAL_MASK_ERROR.set(Some(libc::EINVAL));
        let error = spawn(&mut command(), &[]).unwrap_err();
        OPTIONAL_MASK_ERROR.set(None);
        let expected = Fault {
            operation: Op::Bind,
            path: target.path().to_owned(),
            kind: io::ErrorKind::InvalidInput,
        };
        assert_eq!(
            error.get_ref().unwrap().downcast_ref::<Fault>(),
            Some(&expected)
        );
        assert_eq!(error.to_string(), format!("{expected:?}"));
        let error = with_fault(Op::Bind, || spawn(&mut command(), &[])).unwrap_err();
        let expected = Fault {
            operation: Op::Bind,
            path: "/proc".into(),
            kind: io::ErrorKind::PermissionDenied,
        };
        assert_eq!(
            error.get_ref().unwrap().downcast_ref::<Fault>(),
            Some(&expected)
        );
        assert_eq!(
            error.to_string(),
            "Fault { operation: Bind, path: \"/proc\", kind: PermissionDenied }"
        );
        let output = spawn(&mut command(), &[])
            .unwrap()
            .wait_with_output()
            .unwrap();
        assert_eq!(LAST_MASK.get(), MaskOutcome::Applied);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"hidden");
        assert_eq!(output.stderr, b"");
        OPTIONAL_MASK_TARGET.set(Some(target.path().join("absent")));
        assert_eq!(
            spawn(&mut command(), &[]).unwrap().wait().unwrap().code(),
            Some(0)
        );
        assert_eq!(LAST_MASK.get(), MaskOutcome::Absent);
        OPTIONAL_MASK_TARGET.set(None);
        assert_eq!(fs::read(target.path().join("input")).unwrap(), b"locked");
    }

    #[test]
    fn confinement_relay_partial_write_never_publishes_and_preserves_real_result() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let destination = scratch.path().join("destination");
        fs::create_dir(&destination).unwrap();
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "printf complete > \"$FTOO_GATE_TELEMETRY/record\"; printf unchanged; exit 7",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let relay = Telemetry::at(&mut command, scratch.path(), Some(destination.clone()));
        let output = spawn(&mut command, &[scratch.path()])
            .unwrap()
            .wait_with_output()
            .unwrap();
        RELAY_WRITE_FAILURE.set(true);
        let error = relay.relay().unwrap_err();
        RELAY_WRITE_FAILURE.set(false);
        assert_eq!(error.raw_os_error(), Some(libc::EIO));
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"unchanged");
        assert_eq!(output.stderr, b"");
        drop(relay);
        let entries: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].extension().unwrap(), "jsonl");
        assert_eq!(fs::read(&entries[0]).unwrap(), b"complete");
    }

    fn cross_process_refusals(pid: libc::pid_t) {
        // SAFETY: vectors and their one-byte buffers have valid layouts and
        // lifetimes. The filter refuses every access before kernel validation.
        unsafe {
            let mut byte = b'x';
            let vector = libc::iovec {
                iov_base: (&raw mut byte).cast(),
                iov_len: 1,
            };
            let descriptor = libc::syscall(libc::SYS_pidfd_open, pid, 0);
            assert!(descriptor >= 0);
            for number in [
                libc::SYS_ptrace,
                libc::SYS_process_vm_readv,
                libc::SYS_process_vm_writev,
                libc::SYS_pidfd_getfd,
                libc::SYS_process_madvise,
            ] {
                let result = match number {
                    libc::SYS_ptrace => libc::syscall(number, libc::PTRACE_ATTACH, pid, 0, 0),
                    libc::SYS_pidfd_getfd => libc::syscall(number, descriptor, 1, 0),
                    libc::SYS_process_madvise => libc::syscall(
                        number,
                        descriptor,
                        &raw const vector,
                        1,
                        libc::MADV_DONTNEED,
                        0,
                    ),
                    _ => libc::syscall(number, pid, &raw const vector, 1, &raw const vector, 1, 0),
                };
                assert_eq!(result, -1);
                assert_eq!(*libc::__errno_location(), libc::EACCES);
            }
            assert_eq!(libc::close(i32::try_from(descriptor).unwrap()), 0);
        }
    }

    fn process_probe() {
        // SAFETY: scalar queries, ioctl pointer, and message-queue stat storage
        // are valid; failed open supplies no descriptor to close.
        unsafe {
            assert_eq!(libc::getsid(0), libc::getpid());
            assert_eq!(
                libc::open(c"/dev/tty".as_ptr(), libc::O_RDWR | libc::O_NOCTTY),
                -1
            );
            assert_eq!(*libc::__errno_location(), libc::ENXIO);
            if let Some(path) = std::env::var_os("FTOO_SLAVE_PATH") {
                let path = CString::new(path.as_bytes()).unwrap();
                let terminal = libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
                assert!(terminal >= 0);
                let byte = b'x';
                assert_eq!(libc::ioctl(terminal, libc::TIOCSTI, &raw const byte), -1);
                assert_eq!(*libc::__errno_location(), libc::EACCES);
                assert_eq!(libc::close(terminal), 0);
                assert_eq!(
                    libc::open(c"/dev/tty".as_ptr(), libc::O_RDWR | libc::O_NOCTTY),
                    -1
                );
                assert_eq!(*libc::__errno_location(), libc::ENXIO);
            }
            let mut limits: libc::rlimit = std::mem::zeroed();
            assert_eq!(
                libc::syscall(
                    libc::SYS_prlimit64,
                    libc::getppid(),
                    libc::RLIMIT_NOFILE,
                    std::ptr::null::<libc::rlimit>(),
                    &raw mut limits
                ),
                -1
            );
            assert_eq!(*libc::__errno_location(), libc::EACCES);
            assert_eq!(
                libc::syscall(
                    libc::SYS_prlimit64,
                    0,
                    libc::RLIMIT_NOFILE,
                    std::ptr::null::<libc::rlimit>(),
                    &raw mut limits
                ),
                0
            );
            assert_eq!(
                libc::syscall(
                    libc::SYS_prlimit64,
                    0,
                    libc::RLIMIT_NOFILE,
                    &raw const limits,
                    std::ptr::null_mut::<libc::rlimit>()
                ),
                0
            );
            // The kernel truncates pid_t to its low 32 bits, just like the filter.
            assert_eq!(
                libc::syscall(
                    libc::SYS_prlimit64,
                    1_u64 << 32,
                    libc::RLIMIT_NOFILE,
                    std::ptr::null::<libc::rlimit>(),
                    &raw mut limits
                ),
                0
            );
            let byte = b'x';
            for request in [0x5412_u64, 0x541c] {
                assert_eq!(libc::ioctl(0, request, &raw const byte), -1);
                assert_eq!(*libc::__errno_location(), libc::EACCES);
            }
            cross_process_refusals(libc::getppid());
            let queue = std::env::var("FTOO_OUTSIDE_QUEUE")
                .unwrap()
                .parse::<i32>()
                .unwrap();
            let mut stat: libc::msqid_ds = std::mem::zeroed();
            assert_eq!(libc::msgctl(queue, libc::IPC_STAT, &raw mut stat), -1);
            assert_eq!(*libc::__errno_location(), libc::EINVAL);
            if let Some(path) = std::env::var_os("FTOO_QUEUE_PATH") {
                let path = CString::new(path.as_bytes()).unwrap();
                let fd = libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
                let error = if fd < 0 {
                    *libc::__errno_location()
                } else {
                    assert_eq!(libc::close(fd), 0);
                    0
                };
                fs::write(
                    std::env::var_os("FTOO_QUEUE_EVIDENCE").unwrap(),
                    error.to_string(),
                )
                .unwrap();
            }
        }
    }

    fn nested_mount_attack() -> [i32; 5] {
        let mut pipes = [-1; 2];
        // SAFETY: the post-fork child uses only fixed stack storage and raw
        // syscalls, then _exit. Fork gives it one thread, as NEWUSER requires.
        // The parent owns, reads and closes the pipe and reaps the exact child.
        unsafe {
            assert_eq!(libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC), 0);
            let child = libc::fork();
            assert!(child >= 0);
            if child == 0 {
                libc::close(pipes[0]);
                let mut evidence = [0_i32; 5];
                if libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) != 0 {
                    evidence[0] = *libc::__errno_location();
                }
                let header = [0x2008_0522_u32, 0];
                let mut capabilities = [0_u32; 6];
                if libc::syscall(libc::SYS_capget, header.as_ptr(), capabilities.as_mut_ptr()) == 0
                {
                    evidence[1] = i32::from(capabilities[0] & (1 << 21) != 0);
                }
                let attr = libc::mount_attr {
                    attr_set: 0,
                    attr_clr: libc::MOUNT_ATTR_RDONLY,
                    propagation: 0,
                    userns_fd: 0,
                };
                if libc::syscall(
                    libc::SYS_mount_setattr,
                    libc::AT_FDCWD,
                    c"/".as_ptr(),
                    0,
                    &raw const attr,
                    std::mem::size_of_val(&attr),
                ) != 0
                {
                    evidence[2] = *libc::__errno_location();
                }
                if libc::mount(
                    std::ptr::null(),
                    c"/".as_ptr(),
                    std::ptr::null(),
                    libc::MS_REMOUNT | libc::MS_BIND,
                    std::ptr::null(),
                ) != 0
                {
                    evidence[3] = *libc::__errno_location();
                }
                if libc::umount2(c"/proc".as_ptr(), libc::MNT_DETACH) != 0 {
                    evidence[4] = *libc::__errno_location();
                }
                let written = libc::write(
                    pipes[1],
                    evidence.as_ptr().cast(),
                    std::mem::size_of_val(&evidence),
                );
                libc::_exit(if written == 20 { 0 } else { 97 });
            }
            libc::close(pipes[1]);
            let mut reader = File::from_raw_fd(pipes[0]);
            let mut bytes = [0_u8; 20];
            reader.read_exact(&mut bytes).unwrap();
            let mut status = 0;
            assert_eq!(libc::waitpid(child, &raw mut status, 0), child);
            assert_eq!(status, 0);
            std::array::from_fn(|index| {
                i32::from_ne_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap())
            })
        }
    }

    #[allow(clippy::too_many_lines)] // One real-child socket and privilege evidence sequence.
    fn socket_probe(path: &Path) {
        process_probe();
        let mut evidence = Vec::new();
        // SAFETY: socket addresses and transfer buffers have their complete
        // initialized ABI layouts. Every successful descriptor is closed.
        unsafe {
            assert_eq!(libc::prctl(libc::PR_GET_SECUREBITS), 0x2f);
            assert_eq!(libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0), 1);
            assert_eq!(libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0), -1);
            evidence.push(*libc::__errno_location());
            let mut pair = [-1; 2];
            assert_eq!(
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                    0,
                    pair.as_mut_ptr()
                ),
                -1
            );
            evidence.push(*libc::__errno_location());
            assert_eq!(
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
                    0,
                    pair.as_mut_ptr()
                ),
                0
            );
            assert_eq!(libc::write(pair[0], b"x".as_ptr().cast(), 1), 1);
            let mut byte = 0_u8;
            assert_eq!(libc::read(pair[1], (&raw mut byte).cast(), 1), 1);
            assert_eq!(byte, b'x');
            let abstract_name = std::env::var("FTOO_ABSTRACT_NAME").unwrap();
            let abstract_bytes = [b"\0".as_slice(), abstract_name.as_bytes()].concat();
            for name in [path.as_os_str().as_bytes(), abstract_bytes.as_slice()] {
                let mut address: libc::sockaddr_un = std::mem::zeroed();
                address.sun_family = libc::sa_family_t::try_from(libc::AF_UNIX).unwrap();
                assert!(name.len() < address.sun_path.len());
                std::ptr::copy_nonoverlapping(
                    name.as_ptr(),
                    address.sun_path.as_mut_ptr().cast(),
                    name.len(),
                );
                let length =
                    libc::socklen_t::try_from(2 + name.len() + usize::from(name[0] != 0)).unwrap();
                assert_eq!(
                    libc::connect(pair[0], (&raw const address).cast(), length),
                    -1
                );
                evidence.push(*libc::__errno_location());
                assert_eq!(libc::bind(pair[0], (&raw const address).cast(), length), -1);
                evidence.push(*libc::__errno_location());
                assert_eq!(
                    libc::sendto(
                        pair[0],
                        b"x".as_ptr().cast(),
                        1,
                        0,
                        (&raw const address).cast(),
                        length
                    ),
                    -1
                );
                evidence.push(*libc::__errno_location());
            }
            libc::close(pair[0]);
            libc::close(pair[1]);
            let socket = libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0);
            assert!(socket >= 0);
            let address = libc::sockaddr_in {
                sin_family: libc::sa_family_t::try_from(libc::AF_INET).unwrap(),
                sin_port: 80_u16.to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes([198, 51, 100, 1]),
                },
                sin_zero: [0; 8],
            };
            // Addressed sendto is refused even on an admitted Internet socket.
            assert_eq!(
                libc::sendto(
                    socket,
                    b"x".as_ptr().cast(),
                    1,
                    0,
                    (&raw const address).cast(),
                    libc::socklen_t::try_from(std::mem::size_of_val(&address)).unwrap()
                ),
                -1
            );
            evidence.push(*libc::__errno_location());
            let mut byte = b'x';
            let mut vector = libc::iovec {
                iov_base: (&raw mut byte).cast(),
                iov_len: 1,
            };
            let mut message: libc::msghdr = std::mem::zeroed();
            message.msg_name = (&raw const address).cast_mut().cast();
            message.msg_namelen =
                libc::socklen_t::try_from(std::mem::size_of_val(&address)).unwrap();
            message.msg_iov = &raw mut vector;
            message.msg_iovlen = 1;
            // sendmsg remains permitted. On an Internet datagram socket this
            // independently proves that neither entry branch has an external route.
            assert_eq!(libc::sendmsg(socket, &raw const message, 0), -1);
            evidence.push(*libc::__errno_location());
            libc::close(socket);
        }
        fs::write(
            path.with_extension("evidence"),
            serde_json::to_vec(&evidence).unwrap(),
        )
        .unwrap();
    }

    fn socket_controls(direct: bool) {
        use std::os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixListener},
        };
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        // SAFETY: a private queue belongs only to this fixture and is removed
        // after both the confined observation and the controller reread.
        let queue = unsafe { libc::msgget(libc::IPC_PRIVATE, libc::IPC_CREAT | 0o600) };
        assert!(queue >= 0);
        let queue_name =
            CString::new(format!("/confinement-{}-{direct}", std::process::id())).unwrap();
        // SAFETY: a unique terminated name and null attributes select defaults.
        let posix_queue = unsafe {
            libc::mq_open(
                queue_name.as_ptr(),
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o600_u32,
                std::ptr::null::<libc::mq_attr>(),
            )
        };
        assert!(posix_queue >= 0);
        let socket = scratch.path().join("socket");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        // The two tests can run concurrently; isolate each listener's network
        // namespace by using a unique abstract name for the listening control.
        let abstract_name = format!("confinement-{}-{direct}", std::process::id());
        let unique = SocketAddr::from_abstract_name(&abstract_name).unwrap();
        let abstract_listener = UnixListener::bind_addr(&unique).unwrap();
        abstract_listener.set_nonblocking(true).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "support_bracket::confinement::tests::confinement_attempt_child",
                "--ignored",
                "--nocapture",
            ])
            .env("FTOO_SOCKET_PATH", &socket)
            .env_remove("FTOO_TERMINAL_CONTROLLER")
            .env("FTOO_ABSTRACT_NAME", abstract_name)
            .env("FTOO_OUTSIDE_QUEUE", queue.to_string())
            .env(
                "FTOO_QUEUE_PATH",
                format!("/dev/mqueue{}", queue_name.to_str().unwrap()),
            )
            .env("FTOO_QUEUE_EVIDENCE", scratch.path().join("queue-evidence"))
            .current_dir(scratch.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let outer_visible = Path::new(&format!("/dev/mqueue{}", queue_name.to_str().unwrap()))
            .try_exists()
            .unwrap();
        DIRECT.set(direct);
        let output = spawn(&mut command, &[scratch.path()]);
        DIRECT.set(false);
        let output = output.unwrap().wait_with_output().unwrap();
        // SAFETY: these are the controller's queue descriptor and unique name.
        unsafe {
            assert_eq!(libc::mq_close(posix_queue), 0);
            assert_eq!(libc::mq_unlink(queue_name.as_ptr()), 0);
        }
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let expected = if LAST_MASK.get() == MaskOutcome::Applied || !outer_visible {
            libc::ENOENT
        } else {
            0
        };
        assert_eq!(
            fs::read_to_string(scratch.path().join("queue-evidence")).unwrap(),
            expected.to_string()
        );
        // SAFETY: initialized ABI storage and the fixture's own queue id.
        unsafe {
            let mut stat: libc::msqid_ds = std::mem::zeroed();
            assert_eq!(libc::msgctl(queue, libc::IPC_STAT, &raw mut stat), 0);
            assert_eq!(libc::msgctl(queue, libc::IPC_RMID, std::ptr::null_mut()), 0);
        }
        let evidence: Vec<i32> =
            serde_json::from_slice(&fs::read(socket.with_extension("evidence")).unwrap()).unwrap();
        assert_eq!(
            evidence,
            [
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::EACCES,
                libc::ENETUNREACH
            ]
        );
        assert_eq!(
            listener.accept().unwrap_err().raw_os_error(),
            Some(libc::EAGAIN)
        );
        assert_eq!(
            abstract_listener.accept().unwrap_err().raw_os_error(),
            Some(libc::EAGAIN)
        );
    }

    #[test]
    fn confinement_socket_filter_and_securebits_existing_namespace() {
        socket_controls(false);
    }

    #[test]
    fn confinement_siblings_cannot_access_each_other() {
        use std::io::{BufRead, BufReader, Write};
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "printf 'ready\\n'; read answer; printf intact"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut sibling = spawn(&mut command, &[]).unwrap();
        let mut reader = BufReader::new(sibling.stdout.take().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "ready\n");
        let mut attacker = Command::new(std::env::current_exe().unwrap());
        attacker
            .args([
                "--exact",
                "support_bracket::confinement::tests::confinement_attempt_child",
                "--ignored",
            ])
            .env("FTOO_PEER_PID", sibling.id().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = spawn(&mut attacker, &[])
            .unwrap()
            .wait_with_output()
            .unwrap();
        // Release and reap the sibling even if the attack's assertions failed.
        sibling
            .stdin
            .take()
            .unwrap()
            .write_all(b"continue\n")
            .unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        let sibling_output = sibling.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(sibling_output.status.code(), Some(0));
        assert_eq!(sibling_output.stderr, b"");
        assert_eq!(bytes, b"intact");
    }

    #[test]
    fn confinement_terminal_session_detaches_both_branches() {
        let mut master = -1;
        let mut slave = -1;
        // SAFETY: openpty initializes owned descriptors. The controller's
        // pre-exec closure uses only scalar libc calls, then execs a fresh helper.
        let (master, slave) = unsafe {
            assert_eq!(
                libc::openpty(
                    &raw mut master,
                    &raw mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null()
                ),
                0
            );
            (File::from_raw_fd(master), File::from_raw_fd(slave))
        };
        let mut controller = Command::new(std::env::current_exe().unwrap());
        let slave_path = fs::read_link(format!("/proc/self/fd/{}", slave.as_raw_fd())).unwrap();
        controller
            .args([
                "--exact",
                "support_bracket::confinement::tests::confinement_attempt_child",
                "--ignored",
            ])
            .env("FTOO_TERMINAL_CONTROLLER", "1")
            .env("FTOO_SLAVE_PATH", slave_path)
            .stdin(Stdio::from(slave))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // SAFETY: no allocation or locks; fd 0 is the owned PTY slave after stdio setup.
        unsafe {
            controller.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = controller.output().unwrap();
        drop(master);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }

    #[test]
    fn confinement_session_failure_prevents_exec_and_restores() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let marker = scratch.path().join("started");
        let command = || {
            let mut command = Command::new("/bin/sh");
            command
                .args(["-c", "printf started > \"$1\"", "fixture"])
                .arg(&marker)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            command
        };
        let error = spawn(command().process_group(0), &[scratch.path()]).unwrap_err();
        assert_eq!(
            error.get_ref().unwrap().downcast_ref::<Fault>(),
            Some(&Fault {
                operation: Op::CreateNamespace,
                path: "/".into(),
                kind: io::ErrorKind::PermissionDenied,
            })
        );
        assert_eq!(
            error.to_string(),
            "Fault { operation: CreateNamespace, path: \"/\", kind: PermissionDenied }"
        );
        assert_eq!(
            fs::read(&marker).map_err(|error| error.raw_os_error()),
            Err(Some(libc::ENOENT))
        );
        assert_eq!(
            spawn(&mut command(), &[scratch.path()])
                .unwrap()
                .wait()
                .unwrap()
                .code(),
            Some(0)
        );
        assert_eq!(fs::read(&marker).unwrap(), b"started");
    }

    #[test]
    fn confinement_filter_install_failure_prevents_exec_and_restores() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let marker = scratch.path().join("started");
        let command = || {
            let mut command = Command::new("/bin/sh");
            command
                .args(["-c", "printf started > \"$1\"", "fixture"])
                .arg(&marker);
            command
        };
        FILTER_FAILURE.set(true);
        let failed = spawn(&mut command(), &[scratch.path()]);
        FILTER_FAILURE.set(false);
        let error = failed.unwrap_err();
        let expected = Fault {
            operation: Op::DropPrivileges,
            path: "/".into(),
            kind: io::ErrorKind::PermissionDenied,
        };
        assert_eq!(
            error.get_ref().unwrap().downcast_ref::<Fault>(),
            Some(&expected)
        );
        assert_eq!(
            error.to_string(),
            "Fault { operation: DropPrivileges, path: \"/\", kind: PermissionDenied }"
        );
        assert_eq!(
            fs::read(&marker).map_err(|e| e.raw_os_error()),
            Err(Some(libc::ENOENT))
        );
        assert_eq!(
            spawn(&mut command(), &[scratch.path()])
                .unwrap()
                .wait()
                .unwrap()
                .code(),
            Some(0)
        );
        assert_eq!(fs::read(marker).unwrap(), b"started");
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One refusal matrix shares the exact BPF interpreter.
    fn confinement_filter_program_refuses_foreign_abis_and_async_bypass() {
        fn evaluate(arch: u32, nr: u32, family: u32) -> u32 {
            evaluate_args(arch, nr, [family, 1, 0, 0])
        }
        fn evaluate_args(arch: u32, nr: u32, args: [u32; 4]) -> u32 {
            let mut pc = 0;
            let mut accumulator = 0;
            loop {
                let instruction = &FILTER[pc];
                match instruction.code {
                    0x20 => {
                        accumulator = match instruction.k {
                            0 => nr,
                            4 => arch,
                            16 => args[0],
                            24 => args[1],
                            48 => args[2],
                            52 => args[3],
                            _ => panic!("unexpected load"),
                        }
                    }
                    0x15 | 0x35 => {
                        let matched = if instruction.code == 0x15 {
                            accumulator == instruction.k
                        } else {
                            accumulator >= instruction.k
                        };
                        pc += usize::from(if matched {
                            instruction.jt
                        } else {
                            instruction.jf
                        });
                    }
                    0x54 => accumulator &= instruction.k,
                    0x06 => return instruction.k,
                    _ => panic!("unexpected instruction"),
                }
                pc += 1;
            }
        }
        for arch in [0, 0x4000_0003, 0xc000_00b7] {
            for nr in [0, 41, 42, 49, 53] {
                assert_eq!(evaluate(arch, nr, 1), 0x0005_000d);
            }
        }
        for nr in [
            42,
            49,
            101,
            310,
            311,
            425,
            426,
            427,
            438,
            440,
            512,
            547,
            0x4000_0029,
            u32::MAX,
        ] {
            assert_eq!(evaluate(0xc000_003e, nr, 0), 0x0005_000d);
        }
        assert_eq!(evaluate(0xc000_003e, 41, 1), 0x0005_000d);
        for (nr, argument) in [(41, 2), (41, 10), (53, 1), (0, 0), (1, 1), (59, 0)] {
            assert_eq!(evaluate(0xc000_003e, nr, argument), 0x7fff_0000);
        }
        for flags in [0, 0x800, 0x80000, 0x80800] {
            for kind in [1, 5] {
                assert_eq!(
                    evaluate_args(0xc000_003e, 53, [1, kind | flags, 0, 0]),
                    0x7fff_0000
                );
            }
            for kind in [0, 2, 3, 4, 6, 0x100] {
                assert_eq!(
                    evaluate_args(0xc000_003e, 53, [1, kind | flags, 0, 0]),
                    0x0005_000d
                );
            }
        }
        for (low, high) in [(1, 0), (0, 1), (u32::MAX, u32::MAX)] {
            assert_eq!(
                evaluate_args(0xc000_003e, 44, [0, 0, low, high]),
                0x0005_000d
            );
        }
        assert_eq!(evaluate_args(0xc000_003e, 44, [0; 4]), 0x7fff_0000);
        assert_eq!(evaluate(0xc000_003e, 302, 0), 0x7fff_0000);
        for pid in [1, 42, u32::MAX] {
            assert_eq!(evaluate(0xc000_003e, 302, pid), 0x0005_000d);
        }
        for request in [0x5412, 0x541c] {
            assert_eq!(
                evaluate_args(0xc000_003e, 16, [0, request, 0, 0]),
                0x0005_000d
            );
        }
        assert_eq!(
            evaluate_args(0xc000_003e, 16, [0, 0x5413, 0, 0]),
            0x7fff_0000
        );
    }

    #[test]
    fn confinement_telemetry_relay_refuses_links_names_and_unwritable_destination() {
        use std::os::unix::fs::symlink;
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let source = scratch.path().join("source");
        let destination = scratch.path().join("destination");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        let controller = destination.join("controller.jsonl");
        fs::write(&controller, b"controller").unwrap();
        fs::write(source.join("controller.jsonl"), b"consumer").unwrap();
        symlink(&controller, source.join("link")).unwrap();
        let relay = || Telemetry {
            source: Some(File::open(&source).unwrap()),
            destination: Some(destination.clone()),
        };
        drop(relay());
        assert_eq!(fs::read(&controller).unwrap(), b"controller");
        let mut contents: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| fs::read(entry.unwrap().path()).unwrap())
            .collect();
        contents.sort();
        assert_eq!(contents, [b"consumer".to_vec(), b"controller".to_vec()]);
        // /proc is an existing destination on which a regular file cannot be
        // created, even with capabilities in the fixture's outer user namespace.
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "printf record > \"$FTOO_GATE_TELEMETRY/record\"; printf unchanged; exit 7",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let failed_relay = Telemetry::at(&mut command, scratch.path(), Some("/proc".into()));
        let output = spawn(&mut command, &[scratch.path()])
            .unwrap()
            .wait_with_output()
            .unwrap();
        let error = failed_relay.relay().unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        drop(failed_relay);
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"unchanged");
        assert_eq!(output.stderr, b"");
    }

    #[test]
    fn confinement_telemetry_drops_oversized_files_without_truncation() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let source = scratch.path().join("source");
        let destination = scratch.path().join("destination");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        let limit = vec![b'x'; TELEMETRY_LIMIT];
        fs::write(source.join("limit"), &limit).unwrap();
        fs::write(source.join("oversized"), vec![b'y'; TELEMETRY_LIMIT + 1]).unwrap();
        let relay = Telemetry {
            source: Some(File::open(&source).unwrap()),
            destination: Some(destination.clone()),
        };
        assert_eq!(relay.relay().map_err(|error| error.raw_os_error()), Ok(()));
        let entries: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(fs::read(&entries[0]).unwrap(), limit);
        // Restore the oversized record and reread through a fresh relay pass.
        fs::remove_file(&entries[0]).unwrap();
        fs::remove_file(source.join("limit")).unwrap();
        fs::write(source.join("oversized"), b"restored").unwrap();
        drop(relay);
        let entries: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(fs::read(&entries[0]).unwrap(), b"restored");
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Each inventory mutation restores the same complete control.
    fn confinement_mount_inventory_refuses_writable_aliases_and_nested_mounts() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let mask = Scratch::new(&std::env::temp_dir()).unwrap();
        let plan = Plan::new(&[scratch.path()], mask.path(), None).unwrap();
        let mut root = "1 0 0:1 / / ro - ext4 /dev/root rw\n".to_owned();
        for &index in &plan.masks {
            use std::fmt::Write as _;
            writeln!(
                root,
                "2 1 0:1 /mask {} ro - ext4 /dev/root rw",
                plan.paths[index].to_str().unwrap()
            )
            .unwrap();
        }
        assert_eq!(
            plan.inspect(root.as_bytes(), MaskOutcome::Absent)
                .map_err(|error| plan.fault(error)),
            Ok(())
        );
        let valid = format!(
            "{root}2 1 0:1 / {} rw - ext4 /dev/root rw\n",
            scratch.path().display()
        );
        assert_eq!(
            plan.inspect(valid.as_bytes(), MaskOutcome::Absent)
                .map_err(|error| plan.fault(error)),
            Ok(())
        );
        if let Some(index) = plan.optional_mask {
            let fault = plan
                .inspect(valid.as_bytes(), MaskOutcome::Applied)
                .map_err(|error| plan.fault(error));
            let expected = Fault {
                operation: Op::InspectMount,
                path: "/proc/self/mountinfo".into(),
                kind: io::ErrorKind::PermissionDenied,
            };
            assert_eq!(fault, Err(expected));
            assert_eq!(
                fault.unwrap_err().to_string(),
                "Fault { operation: InspectMount, path: \"/proc/self/mountinfo\", kind: PermissionDenied }"
            );
            let applied = format!(
                "{valid}4 1 0:1 /mask {} ro - ext4 /dev/root rw\n",
                plan.paths[index].to_str().unwrap()
            );
            assert_eq!(
                plan.inspect(applied.as_bytes(), MaskOutcome::Applied)
                    .map_err(|error| plan.fault(error)),
                Ok(())
            );
        }
        for line in [
            "3 1 0:1 / /alias rw - ext4 /dev/root rw\n".to_owned(),
            format!(
                "3 2 0:1 / {}/nested rw - ext4 /dev/root rw\n",
                scratch.path().display()
            ),
            "3 1 0:2 / /unexpected-proc ro - proc proc rw\n".to_owned(),
        ] {
            let changed = format!("{valid}{line}");
            let fault = plan
                .inspect(changed.as_bytes(), MaskOutcome::Absent)
                .map_err(|error| plan.fault(error));
            let expected = Fault {
                operation: Op::InspectMount,
                path: "/proc/self/mountinfo".into(),
                kind: io::ErrorKind::PermissionDenied,
            };
            assert_eq!(fault, Err(expected));
            assert_eq!(
                fault.unwrap_err().to_string(),
                "Fault { operation: InspectMount, path: \"/proc/self/mountinfo\", kind: PermissionDenied }"
            );
            assert_eq!(
                plan.inspect(valid.as_bytes(), MaskOutcome::Absent)
                    .map_err(|error| plan.fault(error)),
                Ok(())
            );
        }
        for &index in &plan.masks {
            let mask = format!(
                "2 1 0:1 /mask {} ro - ext4 /dev/root rw\n",
                plan.paths[index].to_str().unwrap()
            );
            let missing = valid.replace(&mask, "");
            let expected = Fault {
                operation: Op::InspectMount,
                path: "/proc/self/mountinfo".into(),
                kind: io::ErrorKind::PermissionDenied,
            };
            let fault = plan
                .inspect(missing.as_bytes(), MaskOutcome::Absent)
                .map_err(|error| plan.fault(error));
            assert_eq!(fault, Err(expected));
            assert_eq!(
                fault.unwrap_err().to_string(),
                "Fault { operation: InspectMount, path: \"/proc/self/mountinfo\", kind: PermissionDenied }"
            );
            assert_eq!(
                plan.inspect(valid.as_bytes(), MaskOutcome::Absent)
                    .map_err(|error| plan.fault(error)),
                Ok(())
            );
        }
        assert!(encoded_equal(
            b"/scratch\\040with\\134escape",
            b"/scratch with\\escape"
        ));
        assert!(!encoded_equal(b"/scratch\\777", b"/scratch"));
        let invalid = setup_io(
            Op::InspectMount,
            Path::new("/proc/self/mountinfo"),
            decode(b"/scratch\\777"),
        )
        .unwrap_err();
        assert_eq!(
            invalid.get_ref().unwrap().downcast_ref::<Fault>().unwrap(),
            &Fault {
                operation: Op::InspectMount,
                path: "/proc/self/mountinfo".into(),
                kind: io::ErrorKind::InvalidData
            }
        );
    }

    // Invoked only as the confined executable of the following fixture. Its
    // complete errno vector is stored in writable scratch for the controller.
    #[test]
    #[ignore = "confined subprocess helper, explicitly invoked by its parent controls"]
    fn confinement_attempt_child() {
        if std::env::var_os("FTOO_TERMINAL_CONTROLLER").is_some() {
            // SAFETY: this helper owns a controlling PTY, established before exec.
            unsafe {
                let terminal = libc::open(c"/dev/tty".as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
                assert!(terminal >= 0);
                assert_eq!(libc::close(terminal), 0);
            }
            socket_controls(false);
            socket_controls(true);
            return;
        }
        if let Ok(pid) = std::env::var("FTOO_PEER_PID") {
            cross_process_refusals(pid.parse().unwrap());
            return;
        }
        if let Some(path) = std::env::var_os("FTOO_SOCKET_PATH") {
            socket_probe(Path::new(&path));
            return;
        }
        let root =
            std::env::var_os("FTOO_CONFINEMENT_ATTEMPTS").expect("parent supplies attack fixture");
        let root = PathBuf::from(root);
        let scratch = PathBuf::from(std::env::var_os("FTOO_CONFINEMENT_OUTPUT").unwrap());
        let descriptor: i32 = std::env::var("FTOO_CONFINEMENT_DESCRIPTOR")
            .unwrap()
            .parse()
            .unwrap();
        let directory: i32 = std::env::var("FTOO_CONFINEMENT_DIRECTORY")
            .unwrap()
            .parse()
            .unwrap();
        let parent = std::env::var("FTOO_CONFINEMENT_PARENT").unwrap();
        let mut evidence = vec![];
        for path in [
            root.join("input"),
            root.join("alias"),
            root.join("nested/input"),
        ] {
            let error = fs::OpenOptions::new().write(true).open(path).unwrap_err();
            evidence.push(error.raw_os_error().unwrap());
        }
        // SAFETY: all paths are terminated and the buffer is valid; failure is
        // the independently asserted outcome, and successful opens are closed.
        unsafe {
            assert_eq!(libc::write(descriptor, b"changed".as_ptr().cast(), 7), -1);
            evidence.push(*libc::__errno_location());
            let fd = libc::openat(directory, c"input".as_ptr(), libc::O_WRONLY);
            assert_eq!(fd, -1);
            evidence.push(*libc::__errno_location());
            for path in [
                PathBuf::from(format!("/proc/{parent}/root"))
                    .join(root.strip_prefix("/").unwrap())
                    .join("input"),
                root.join(format!("proc-alias/{parent}/root"))
                    .join(root.strip_prefix("/").unwrap())
                    .join("input"),
            ] {
                let path = CString::new(path.as_os_str().as_bytes()).unwrap();
                let fd = libc::open(path.as_ptr(), libc::O_WRONLY);
                assert_eq!(fd, -1);
                evidence.push(*libc::__errno_location());
            }
            let path = CString::new(root.as_os_str().as_bytes()).unwrap();
            assert_eq!(
                libc::mount(
                    std::ptr::null(),
                    path.as_ptr(),
                    std::ptr::null(),
                    libc::MS_REMOUNT | libc::MS_BIND,
                    std::ptr::null()
                ),
                -1
            );
            evidence.push(*libc::__errno_location());
            let socket = libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0);
            assert!(socket >= 0);
            let address = libc::sockaddr_in {
                sin_family: libc::sa_family_t::try_from(libc::AF_INET).unwrap(),
                sin_port: 80_u16.to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes([198, 51, 100, 1]),
                },
                sin_zero: [0; 8],
            };
            assert_eq!(
                libc::connect(
                    socket,
                    (&raw const address).cast(),
                    libc::socklen_t::try_from(std::mem::size_of_val(&address)).unwrap()
                ),
                -1
            );
            evidence.push(*libc::__errno_location());
            libc::close(socket);
        }
        evidence.extend(nested_mount_attack());
        fs::write(
            scratch.join("evidence"),
            serde_json::to_vec(&evidence).unwrap(),
        )
        .unwrap();
    }

    #[test]
    #[allow(clippy::too_many_lines)] // The namespace owns all aliases and restoration evidence.
    fn confinement_nested_mount_alias_descriptors_and_remount_are_refused() {
        struct Borrowed(PathBuf);
        impl Borrowed {
            fn path(&self) -> &Path {
                &self.0
            }
        }
        const TEST: &str = "support_bracket::confinement::tests::confinement_nested_mount_alias_descriptors_and_remount_are_refused";
        if std::env::var_os("FTOO_CONFINEMENT_CONTROLLER").is_none() {
            let checked = Scratch::new(&std::env::temp_dir()).unwrap();
            let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
            let mut controller = Command::new(std::env::current_exe().unwrap());
            controller
                .args(["--exact", TEST, "--nocapture"])
                .env("FTOO_CONFINEMENT_CONTROLLER", "1")
                .env("FTOO_CONFINEMENT_CHECKED", checked.path())
                .env("FTOO_CONFINEMENT_SCRATCH", scratch.path());
            // SAFETY: a fixture controller gets its own mount namespace before
            // exec; it never changes the test runner's mounts.
            unsafe {
                controller.pre_exec(|| {
                    if libc::unshare(libc::CLONE_NEWNS) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                    if libc::mount(
                        std::ptr::null(),
                        c"/".as_ptr(),
                        std::ptr::null(),
                        libc::MS_REC | libc::MS_PRIVATE,
                        std::ptr::null(),
                    ) != 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let output = controller.output().unwrap();
            assert_eq!(output.status.code(), Some(0), "{output:?}");
            return;
        }
        // The parent owns directory cleanup. Exiting this controller releases
        // its private mount namespace before the parent removes the files.
        let checked = Borrowed(std::env::var_os("FTOO_CONFINEMENT_CHECKED").unwrap().into());
        let scratch = Borrowed(std::env::var_os("FTOO_CONFINEMENT_SCRATCH").unwrap().into());
        let input = checked.path().join("input");
        fs::write(&input, b"locked").unwrap();
        fs::hard_link(&input, checked.path().join("alias")).unwrap();
        for name in ["nested", "proc-alias"] {
            fs::create_dir(checked.path().join(name)).unwrap();
        }
        let root = CString::new(checked.path().as_os_str().as_bytes()).unwrap();
        let nested = CString::new(checked.path().join("nested").as_os_str().as_bytes()).unwrap();
        let proc_alias =
            CString::new(checked.path().join("proc-alias").as_os_str().as_bytes()).unwrap();
        // SAFETY: this fixture controller owns its isolated namespace. The bind
        // is writable before confinement, and procfs has an alternate spelling.
        unsafe {
            assert_eq!(
                libc::mount(
                    root.as_ptr(),
                    nested.as_ptr(),
                    std::ptr::null(),
                    libc::MS_BIND,
                    std::ptr::null()
                ),
                0
            );
            assert_eq!(
                libc::mount(
                    c"/proc".as_ptr(),
                    proc_alias.as_ptr(),
                    std::ptr::null(),
                    libc::MS_BIND | libc::MS_REC,
                    std::ptr::null()
                ),
                0,
                "{}",
                io::Error::last_os_error()
            );
        }
        let writable = fs::OpenOptions::new().write(true).open(&input).unwrap();
        let directory = File::open(checked.path()).unwrap();
        // F_DUPFD deliberately makes inheritable descriptors, above the normal
        // runtime range so a closed number cannot be reused by test startup.
        let (writable, directory) = unsafe {
            let writable = libc::fcntl(writable.as_raw_fd(), libc::F_DUPFD, 200);
            let directory = libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD, 201);
            assert!(writable >= 200 && directory >= 201);
            (File::from_raw_fd(writable), File::from_raw_fd(directory))
        };
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "support_bracket::confinement::tests::confinement_attempt_child",
                "--ignored",
                "--nocapture",
            ])
            .env("FTOO_CONFINEMENT_ATTEMPTS", checked.path())
            .env("FTOO_CONFINEMENT_OUTPUT", scratch.path())
            .env(
                "FTOO_CONFINEMENT_DESCRIPTOR",
                writable.as_raw_fd().to_string(),
            )
            .env(
                "FTOO_CONFINEMENT_DIRECTORY",
                directory.as_raw_fd().to_string(),
            )
            .env("FTOO_CONFINEMENT_PARENT", std::process::id().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = spawn(&mut command, &[scratch.path()])
            .unwrap()
            .wait_with_output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let evidence: Vec<i32> =
            serde_json::from_slice(&fs::read(scratch.path().join("evidence")).unwrap()).unwrap();
        assert_eq!(
            evidence,
            [
                libc::EROFS,
                libc::EROFS,
                libc::EROFS,
                libc::EBADF,
                libc::EBADF,
                libc::ENOENT,
                libc::ENOENT,
                libc::EPERM,
                libc::EACCES,
                0, // nested namespace creation succeeded
                1, // CAP_SYS_ADMIN was restored in that namespace
                libc::EPERM,
                libc::EPERM,
                libc::EINVAL // a locked mount cannot be detached
            ]
        );
        assert_eq!(fs::read(&input).unwrap(), b"locked");
        // Controller writes still work, including the previously writable alias.
        fs::write(checked.path().join("nested/input"), b"restored").unwrap();
        assert_eq!(fs::read(&input).unwrap(), b"restored");
        fs::write(&input, b"locked").unwrap();
        assert_eq!(fs::read(checked.path().join("alias")).unwrap(), b"locked");
        let mut reread = Command::new("/bin/cat");
        reread
            .arg(&input)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = spawn(&mut reread, &[]).unwrap().wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"locked");
        assert_eq!(output.stderr, b"");
    }
    #[test]
    fn confinement_each_operation_fault_prevents_exec_and_restores() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let marker = scratch.path().join("started");
        let cases = [
            (Op::CreateNamespace, Path::new("/")),
            (Op::WriteIdentityMaps, Path::new("/proc/self/uid_map")),
            (Op::IsolateNetwork, Path::new("/proc/net/dev")),
            (Op::MakePrivate, Path::new("/")),
            (Op::Bind, Path::new("/proc")),
            (Op::RemountReadOnly, Path::new("/")),
            (Op::RemountWritable, scratch.path()),
            (Op::InspectMount, Path::new("/proc/self/mountinfo")),
            (Op::CloseWritableDescriptors, Path::new("/proc/self/fd")),
            (Op::DropPrivileges, Path::new("/")),
        ];
        for (operation, path) in cases {
            let command = || {
                let mut command = Command::new("/bin/sh");
                command
                    .arg("-c")
                    .arg("printf started > \"$1\"")
                    .arg("fixture")
                    .arg(&marker);
                command
            };
            let error =
                with_fault(operation, || spawn(&mut command(), &[scratch.path()])).unwrap_err();
            let expected = Fault {
                operation,
                path: path.to_owned(),
                kind: io::ErrorKind::PermissionDenied,
            };
            let fault = error.get_ref().unwrap().downcast_ref::<Fault>().unwrap();
            assert_eq!(fault, &expected);
            assert_eq!(
                error.to_string(),
                format!(
                    "Fault {{ operation: {operation:?}, path: {path:?}, kind: PermissionDenied }}"
                )
            );
            let native = crate::NativeAdapterFault::from(fault.clone());
            let notation = crate::NotationAdapterFault::from(fault.clone());
            assert_eq!(
                native,
                crate::NativeAdapterFault::Confinement {
                    operation,
                    path: path.to_owned(),
                    kind: io::ErrorKind::PermissionDenied
                }
            );
            assert_eq!(
                notation,
                crate::NotationAdapterFault::Confinement {
                    operation,
                    path: path.to_owned(),
                    kind: io::ErrorKind::PermissionDenied
                }
            );
            let display = format!(
                "Confinement {{ operation: {operation:?}, path: {path:?}, kind: PermissionDenied }}"
            );
            assert_eq!(
                crate::DocumentError::Native(native).to_string(),
                format!("Native({display})")
            );
            assert_eq!(notation.to_string(), display);
            assert_eq!(
                fs::read(&marker).map_err(|e| e.raw_os_error()),
                Err(Some(libc::ENOENT))
            );
            let mut child = spawn(&mut command(), &[scratch.path()]).unwrap();
            assert_eq!(child.wait().unwrap().code(), Some(0));
            assert_eq!(fs::read(&marker).unwrap(), b"started");
            fs::remove_file(&marker).unwrap();
        }
    }

    #[test]
    fn confinement_direct_entry_writes_maps_and_isolates_network() {
        socket_controls(true);
    }
    #[test]
    fn confinement_readonly_view_and_scratch_control() {
        let scratch = Scratch::new(&std::env::temp_dir()).unwrap();
        let checked = Scratch::new(&std::env::temp_dir()).unwrap();
        let input = checked.path().join("input");
        fs::write(&input, b"locked").unwrap();
        let mut command = Command::new("/bin/sh");
        command.current_dir(scratch.path()).arg("-c").arg("test ! -e /proc/self/root && ! (printf changed > \"$1\") 2>/dev/null && printf output > \"$2/output\" && printf relative > relative").arg("fixture").arg(&input).arg(scratch.path());
        let mut child = spawn(&mut command, &[scratch.path()]).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(0));
        assert_eq!(fs::read(input).unwrap(), b"locked");
        assert_eq!(fs::read(scratch.path().join("output")).unwrap(), b"output");
        assert_eq!(
            fs::read(scratch.path().join("relative")).unwrap(),
            b"relative"
        );
    }
}
