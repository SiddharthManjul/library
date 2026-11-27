use std::io::{self, Write};

use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

use crate::redact::{scrub, strip_ansi};

/// Wraps any writer so each log line is scrubbed before it leaves the process.
#[derive(Clone)]
pub struct RedactingMakeWriter<M>(pub M);

pub struct RedactingWriter<W: Write> {
    inner: W,
    buf: Vec<u8>,
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W: Write> Drop for RedactingWriter<W> {
    /// One event is written and then the writer is dropped; the whole event is scrubbed as one piece,
    /// so a secret split across several `write` calls is still found.
    fn drop(&mut self) {
        let text = String::from_utf8_lossy(&self.buf);
        let _ = self.inner.write_all(scrub(&strip_ansi(&text)).as_bytes());
        let _ = self.inner.flush();
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.0.make_writer(),
            buf: Vec::new(),
        }
    }
}

/// Installs the global subscriber: filtered by `RUST_LOG`, one line per event, scrubbed.
/// Returns false when a subscriber was already installed.
pub fn init_tracing<M>(writer: M, json: bool) -> bool
where
    M: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let w = RedactingMakeWriter(writer);
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(w)
        .with_target(true);
    if json {
        builder.json().try_init().is_ok()
    } else {
        builder.try_init().is_ok()
    }
}
