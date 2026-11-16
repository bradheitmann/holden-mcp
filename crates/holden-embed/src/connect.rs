//! `holden connect --role <role> --socket <path>`: the stdio bridge a
//! harness runs (the command in the host's `.mcp.json`) to reach the holden
//! server's Unix socket. It only moves bytes: the server reads the caller's
//! uid from the kernel and its role from the socket's sealed role map, so a
//! bridge cannot change what its caller may do.

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Copy a live byte stream one read at a time, flushing each response chunk.
/// `io::copy` may use Linux's kernel-copy operations for pipes and sockets;
/// this bridge instead uses the same userspace streaming path on every OS.
fn pump<R: Read + ?Sized, W: Write + ?Sized>(from: &mut R, to: &mut W) -> io::Result<()> {
    let mut buf = [0_u8; 8192];
    loop {
        let n = from.read(&mut buf)?;
        if n == 0 {
            return to.flush();
        }
        to.write_all(&buf[..n])?;
        to.flush()?;
    }
}

/// Bridge this process's stdin and stdout to the socket at `socket` until
/// the server closes the connection.
///
/// # Errors
/// The I/O error when the socket cannot be reached or the copy fails.
pub fn bridge(socket: &Path) -> io::Result<()> {
    let stream = UnixStream::connect(socket)?;
    let mut to_server = stream.try_clone()?;
    let mut from_server = stream;
    // The harness closing stdin ends the request side; the server then
    // closes the connection, which ends the bridge.
    std::thread::spawn(move || {
        let _ = pump(&mut io::stdin().lock(), &mut to_server);
        let _ = to_server.shutdown(Shutdown::Write);
    });
    pump(&mut from_server, &mut io::stdout().lock())
}
