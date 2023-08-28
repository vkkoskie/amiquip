use super::{HandshakeStream, IoStream};
use crate::errors::*;
use mio::{event::Source, Interest, Registry, Token};
use native_tls::{HandshakeError, MidHandshakeTlsStream};
use snafu::ResultExt;
use std::io::{self, Read, Write};

/// Newtype wrapper around a `native_tls::TlsConnector` to make it usable by amiquip's I/O loop.
pub struct TlsConnector(native_tls::TlsConnector);

impl TlsConnector {
    pub(crate) fn connect<S>(&self, domain: &str, stream: S) -> Result<TlsHandshakeStream<S>>
    where
        S: Read + Write,
    {
        let inner = Some(match self.0.connect(domain, stream) {
            Ok(s) => InnerHandshake::Done(s),
            Err(HandshakeError::WouldBlock(s)) => InnerHandshake::MidHandshake(s),
            Err(HandshakeError::Failure(err)) => Err(err).context(TlsHandshakeSnafu)?,
        });
        Ok(TlsHandshakeStream { inner })
    }
}

impl From<native_tls::TlsConnector> for TlsConnector {
    fn from(inner: native_tls::TlsConnector) -> TlsConnector {
        TlsConnector(inner)
    }
}

pub(crate) struct TlsHandshakeStream<S> {
    inner: Option<InnerHandshake<S>>,
}

enum InnerHandshake<S> {
    MidHandshake(MidHandshakeTlsStream<S>),
    Done(native_tls::TlsStream<S>),
}

impl<S: Read + Write> InnerHandshake<S> {
    fn get_ref(&self) -> &S {
        match self {
            InnerHandshake::MidHandshake(s) => s.get_ref(),
            InnerHandshake::Done(s) => s.get_ref(),
        }
    }
}

impl<S: Source + Read + Write + Send + 'static> HandshakeStream for TlsHandshakeStream<S> {
    type Stream = TlsStream<S>;

    fn progress_handshake(&mut self) -> Result<Option<Self::Stream>> {
        let mid_hs = match self.inner.take().unwrap() {
            InnerHandshake::MidHandshake(mid_hs) => mid_hs,
            InnerHandshake::Done(s) => return Ok(Some(TlsStream(s))),
        };

        match mid_hs.handshake() {
            Ok(s) => Ok(Some(TlsStream(s))),
            Err(HandshakeError::WouldBlock(s)) => {
                self.inner = Some(InnerHandshake::MidHandshake(s));
                Ok(None)
            }
            Err(HandshakeError::Failure(err)) => Err(err).context(TlsHandshakeSnafu)?,
        }
    }
}

impl<S: Source + Read + Write> Source for TlsHandshakeStream<S> {
    #[inline]
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner
            .as_ref()
            .unwrap()
            .get_ref()
            .register(registry, token, interests)
    }

    #[inline]
    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner
            .as_ref()
            .unwrap()
            .get_ref()
            .reregister(registry, token, interests)
    }

    #[inline]
    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        self.inner.as_ref().unwrap().get_ref().deregister(registry)
    }
}

pub(crate) struct TlsStream<S>(native_tls::TlsStream<S>);

impl<S: Source + Read + Write + Send + 'static> IoStream for TlsStream<S> {}

impl<S: Read + Write> Read for TlsStream<S> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<S: Read + Write> Write for TlsStream<S> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<S: Source + Read + Write> Source for TlsStream<S> {
    #[inline]
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.0.get_ref().register(registry, token, interests)
    }

    #[inline]
    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.0.get_ref().reregister(registry, token, interests)
    }

    #[inline]
    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        self.0.get_ref().deregister(registry)
    }
}
