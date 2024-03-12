use std::{
    collections::VecDeque,
    io::{ErrorKind, Read, Result, Write},
    sync::{atomic::AtomicBool, Arc},
    task::Poll,
};

use futures::{lock::Mutex, task::AtomicWaker, FutureExt};

#[derive(Clone)]
pub struct Channel {
    inner: Arc<ChannelInner>,
}

impl Channel {
    pub fn new() -> Self {
        let inner = ChannelInner {
            waker: AtomicWaker::new(),
            buf: Mutex::new(VecDeque::new()),
            closed: AtomicBool::new(false),
        };
        let inner = Arc::new(inner);
        Self { inner }
    }
}

struct ChannelInner {
    waker: AtomicWaker,
    buf: Mutex<VecDeque<u8>>,
    closed: AtomicBool,
}

impl futures::AsyncRead for Channel {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        match self.inner.buf.lock().poll_unpin(cx) {
            Poll::Ready(mut inner_buf) => {
                self.inner.waker.register(cx.waker());
                if inner_buf.is_empty() {
                    if self.inner.closed.load(std::sync::atomic::Ordering::SeqCst) {
                        return Poll::Ready(Err(ErrorKind::BrokenPipe.into()));
                    }
                    return Poll::Pending;
                }
                let new_len = inner_buf.len().saturating_sub(buf.len());
                let n = inner_buf.read(buf)?;
                buf = &mut buf[n..];
                let m = inner_buf.read(buf)?;
                inner_buf.resize(new_len, 0);
                Poll::Ready(Ok(n + m))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for ChannelInner {
    fn drop(&mut self) {
        self.waker.wake();
    }
}

impl futures::AsyncWrite for Channel {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.inner.waker.register(cx.waker());
        match self.inner.buf.lock().poll_unpin(cx) {
            Poll::Ready(mut inner_buf) => {
                let n = inner_buf.write(buf)?;

                if n == 0 {
                    return Poll::Pending;
                }

                self.inner.waker.wake();

                Poll::Ready(Ok(n))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<()>> {
        self.inner.waker.register(cx.waker());
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<()>> {
        self.inner
            .closed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.inner.waker.register(cx.waker());
        Poll::Ready(Ok(()))
    }
}
