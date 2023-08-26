use crate::Result;
use mio::{event::Source, net::TcpStream};
use std::io::{Read, Write};

pub(crate) trait HandshakeStream: Source + Send + 'static {
    type Stream: IoStream;

    fn progress_handshake(&mut self) -> Result<Option<Self::Stream>>;
}

/// Combination trait for readable, writable streams that can be polled by mio.
pub trait IoStream: Read + Write + Source + Send + 'static {}

impl IoStream for TcpStream {}

#[cfg(feature = "native-tls")]
mod native_tls;

#[cfg(feature = "native-tls")]
pub use self::native_tls::TlsConnector;
