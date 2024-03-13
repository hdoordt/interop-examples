use std::{
    io::{Error, ErrorKind, Result},
    pin::{pin, Pin},
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    task::AtomicWaker,
    AsyncRead, AsyncWrite, Future, StreamExt,
};

pub struct ChannelReader {
    closed: bool,
    inner: UnboundedReceiver<u8>,
    waker: Arc<AtomicWaker>,
}

impl AsyncRead for ChannelReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.waker.register(cx.waker());

        if self.closed {
            return Poll::Ready(Err(Error::from(ErrorKind::BrokenPipe)));
        }

        let mut n = 0;
        while let Poll::Ready(next) = pin!(self.inner.next()).poll(cx) {
            let Some(next) = next else {
                self.closed = true;
                break;
            };
            buf[n] = next;
            n += 1;
        }

        match n {
            0 => Poll::Pending,
            n => Poll::Ready(Ok(n)),
        }
    }
}

pub struct ChannelWriter {
    inner: UnboundedSender<u8>,
    waker: Arc<AtomicWaker>,
}

impl AsyncWrite for ChannelWriter {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        for b in buf.iter().copied() {
            if let Err(_e) = self.inner.unbounded_send(b) {
                return Poll::Ready(Err(Error::from(ErrorKind::BrokenPipe)));
            }
        }

        // self.waker.wake();

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.waker.wake();
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.waker.wake();
        self.inner.close_channel();
        Poll::Ready(Ok(()))
    }
}

pub fn channel() -> (ChannelReader, ChannelWriter) {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let waker = Arc::new(AtomicWaker::new());

    let r = ChannelReader {
        closed: false,
        inner: rx,
        waker: waker.clone(),
    };

    let w = ChannelWriter { inner: tx, waker };

    (r, w)
}
