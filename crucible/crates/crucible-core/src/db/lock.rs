//! One crucible per database directory, enforced by the kernel.
//!
//! Two sweepers against one database is not a merge conflict, it is two
//! schedulers each believing they own the queue: both dequeue the same board,
//! both spawn a planner, and the box that was carefully limited to two jobs is
//! suddenly running four. The timings from that hour are worthless and nothing
//! in the output says so.
//!
//! The lock is `flock(LOCK_EX|LOCK_NB)` on a file in the database directory,
//! held open for the life of the process, and that choice is the whole point:
//! **the kernel drops it when the holder dies, however it dies.** A PID file
//! does not. After a `kill -9`, a stale PID file is the difference between
//! "restart works" and "delete this file by hand before the sweep will start
//! again" -- and the second one gets discovered at 3am by somebody who then
//! deletes the wrong file.
//!
//! Two details that look cosmetic and are not:
//!
//! * `O_CLOEXEC`. Every planner this process spawns inherits its open file
//!   descriptors. An inherited lock fd means a wedged child keeps the lock
//!   alive after the crucible that took it has exited, which reproduces the
//!   stale-PID-file failure by another route.
//! * The lock file is never unlinked, not even on a clean exit. Unlinking races
//!   a second process that has already opened the same inode: it would hold a
//!   lock on a file nobody can find, and the next opener would create a fresh
//!   inode and lock that instead -- two holders, no error. An empty file is a
//!   cheap price for that not being possible.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Why the lock could not be taken. `Busy` is a normal operational state (the
/// answer is "the other one is still running"), everything else is a real
/// fault, and a caller must be able to tell them apart without matching on a
/// string.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("{path}: another crucible is already running against this database")]
    Busy { path: String },
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// An exclusive advisory lock on a database directory, released on drop -- or
/// by the kernel, if this process never gets to drop anything.
#[derive(Debug)]
pub struct DirLock {
    fd: libc::c_int,
    path: PathBuf,
}

impl DirLock {
    /// The lock file's name inside the database directory.
    pub const FILE_NAME: &'static str = ".crucible.lock";

    /// Take the lock, or say who has it.
    ///
    /// The directory is created if it does not exist: the first run of a fresh
    /// checkout should not fail on a missing parent.
    pub fn acquire(dir: &Path) -> Result<Self, LockError> {
        let path = dir.join(Self::FILE_NAME);
        let named = |source: std::io::Error| LockError::Io {
            path: path.display().to_string(),
            source,
        };
        std::fs::create_dir_all(dir).map_err(named)?;

        let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
            named(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "database path contains a NUL byte",
            ))
        })?;

        // SAFETY: c_path is a live NUL-terminated string for the duration of
        // the call, and the variadic mode argument is required by O_CREAT.
        let fd = unsafe {
            libc::open(
                c_path.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC,
                0o644 as libc::c_int,
            )
        };
        if fd < 0 {
            return Err(named(std::io::Error::last_os_error()));
        }
        let held = DirLock {
            fd,
            path: path.clone(),
        };

        // SAFETY: fd is a descriptor this function just opened and owns.
        if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let e = std::io::Error::last_os_error();
            // EWOULDBLOCK and EAGAIN are the same number on Darwin and Linux,
            // so this is a guard rather than two match arms -- as patterns the
            // second would be dead code and `-D warnings` would say so, while
            // dropping it would break a platform where they differ.
            let busy =
                matches!(e.raw_os_error(), Some(c) if c == libc::EWOULDBLOCK || c == libc::EAGAIN);
            return if busy {
                Err(LockError::Busy {
                    path: path.display().to_string(),
                })
            } else {
                Err(named(e))
            };
            // `held` drops here and closes the fd. It never took the lock, so
            // closing it cannot release somebody else's.
        }

        held.stamp_pid();
        Ok(held)
    }

    /// The path of the lock file, for messages a human has to act on.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write our pid into the file. Purely so `cat .crucible.lock` answers the
    /// question a human actually has. Nothing reads it back and nothing may
    /// ever start to: the lock is the kernel's, and a second source of truth
    /// about who holds it is how the stale-PID-file failure gets reinvented.
    fn stamp_pid(&self) {
        // SAFETY: fd is owned by self; both calls are plain syscalls on it.
        unsafe {
            libc::ftruncate(self.fd, 0);
            let text = format!("{}\n", std::process::id());
            libc::write(self.fd, text.as_ptr() as *const libc::c_void, text.len());
        }
    }
}

impl Drop for DirLock {
    fn drop(&mut self) {
        // Closing the descriptor releases the flock. Explicitly unlocking first
        // would be redundant, and the file is deliberately left in place.
        //
        // SAFETY: fd is owned by self and is closed exactly once.
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "crucible-lock-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The property the whole module exists for: a second opener is refused,
    /// and refused with `Busy` rather than a generic io error a caller would
    /// have to parse a string to recognise.
    ///
    /// flock is held against the open file DESCRIPTION, not the process, so two
    /// opens in one process conflict exactly as two processes would -- which is
    /// what makes this testable at all.
    #[test]
    fn second_opener_is_refused() {
        let dir = scratch("second");
        let first = DirLock::acquire(&dir).expect("first lock");
        match DirLock::acquire(&dir) {
            Err(LockError::Busy { .. }) => {}
            other => panic!("expected Busy, got {other:?}"),
        }
        drop(first);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Releasing must actually release. If the fd leaked -- the obvious way to
    /// get this wrong -- a restart after a clean shutdown would be refused, and
    /// the only symptom would be a sweep that never starts.
    #[test]
    fn releasing_lets_the_next_one_in() {
        let dir = scratch("release");
        drop(DirLock::acquire(&dir).expect("first lock"));
        let _second = DirLock::acquire(&dir).expect("lock after release");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The lock file survives the lock. Unlinking it would let a later opener
    /// create a different inode and lock that instead -- two holders, no error.
    #[test]
    fn lock_file_is_not_unlinked() {
        let dir = scratch("keep");
        let path = {
            let l = DirLock::acquire(&dir).expect("lock");
            l.path().to_path_buf()
        };
        assert!(path.exists(), "lock file was removed on drop");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
