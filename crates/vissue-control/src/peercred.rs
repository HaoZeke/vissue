//! Same-uid check for an accepted Unix socket. No `unsafe` in this crate.

use std::io;
use std::os::fd::AsFd;

use nix::unistd::Uid;

/// Accept the peer when its uid matches the current process uid.
#[must_use]
pub fn accept_peer(uid: u32) -> bool {
    uid == current_uid()
}

/// Real uid of this process (`getuid`), matching `Uid::current`.
#[must_use]
pub fn current_uid() -> u32 {
    Uid::current().as_raw()
}

/// Peer uid from `SO_PEERCRED` on Linux, or `getpeereid` on BSD/macOS.
pub fn peer_uid<F: AsFd>(sock: &F) -> io::Result<u32> {
    peer_uid_impl(sock)
}

/// True when [`peer_uid`] succeeds and [`accept_peer`] accepts it.
#[must_use]
pub fn accept_socket<F: AsFd>(sock: &F) -> bool {
    match peer_uid(sock) {
        Ok(uid) => accept_peer(uid),
        Err(_) => false,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn peer_uid_impl<F: AsFd>(sock: &F) -> io::Result<u32> {
    use nix::sys::socket::{getsockopt, sockopt};
    let creds = getsockopt(sock, sockopt::PeerCredentials)?;
    Ok(creds.uid())
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn peer_uid_impl<F: AsFd>(sock: &F) -> io::Result<u32> {
    let (uid, _) = nix::unistd::getpeereid(sock)?;
    Ok(uid.as_raw())
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn peer_uid_impl<F: AsFd>(_sock: &F) -> io::Result<u32> {
    // Dir 0700 / sock 0600 is the check on this OS.
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "peer credentials are unavailable",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_uid_is_accepted() {
        assert!(accept_peer(current_uid()));
    }

    #[test]
    fn other_uid_is_rejected() {
        let other = current_uid().wrapping_add(1);
        assert!(!accept_peer(other));
    }

    #[test]
    fn same_process_peer_is_accepted() {
        use std::os::unix::net::{UnixListener, UnixStream};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peer.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let client = UnixStream::connect(&path).unwrap();
        let (server, _) = listener.accept().unwrap();
        let uid = peer_uid(&server).unwrap();
        assert_eq!(uid, current_uid());
        assert!(accept_peer(uid));
        assert!(accept_socket(&server));
        assert!(accept_socket(&client));
    }
}
