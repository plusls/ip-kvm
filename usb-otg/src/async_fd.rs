use std::convert::TryFrom;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;
use nix;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, unix};

pub struct AsyncFd(unix::AsyncFd<RawFd>);

impl TryFrom<RawFd> for AsyncFd {
    type Error = std::io::Error;

    fn try_from(fd: RawFd) -> io::Result<Self> {
        set_nonblock(fd)?;
        Ok(Self(unix::AsyncFd::new(fd)?))
    }
}

impl AsRawFd for AsyncFd {
    fn as_raw_fd(&self) -> RawFd {
        *self.0.get_ref()
    }
}

impl Drop for AsyncFd {
    fn drop(&mut self) {
        if let Err(err) = nix::unistd::close(self.as_raw_fd()) {
            log::error!("Close failed: {err}");
        }
    }
}

impl AsyncRead for AsyncFd {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.0.poll_read_ready(cx))?;

            match nix::unistd::read(self.as_raw_fd(), buf.initialize_unfilled()) {
                Ok(len) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Err(nix::Error::EWOULDBLOCK) => {
                    guard.clear_ready();
                    continue;
                }
                Err(err) => {
                    return Poll::Ready(Err(err.into()));
                }
            }
        }
    }
}

impl AsyncWrite for AsyncFd {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.0.poll_write_ready(cx))?;

            match nix::unistd::write(self.as_raw_fd(), buf) {
                Ok(len) => {
                    return Poll::Ready(Ok(len));
                }
                Err(nix::Error::EWOULDBLOCK) => {
                    guard.clear_ready();
                    continue;
                }
                Err(err) => {
                    return Poll::Ready(Err(err.into()));
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn set_nonblock(fd: RawFd) -> io::Result<()> {
    // fcntl 成功返回的情况下 OFlag 一定是合法的
    let mut flags = nix::fcntl::OFlag::from_bits(nix::fcntl::fcntl(fd, nix::fcntl::F_GETFL)?).unwrap();
    flags.set(nix::fcntl::OFlag::O_NONBLOCK, true);
    nix::fcntl::fcntl(fd, nix::fcntl::F_SETFL(flags))?;
    Ok(())
}