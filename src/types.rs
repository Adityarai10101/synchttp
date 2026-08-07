use std::time::Duration;

use http::StatusCode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    BadRequest(&'static str),
    HeaderTooLarge,
    PayloadTooLarge,
    NotImplemented(&'static str),
}

impl ParseError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ParseError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ParseError::HeaderTooLarge => StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            ParseError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ParseError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
        }
    }
}

/// The operation that produced a non-fatal error passed to
/// [`Server::on_error`](crate::Server::on_error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorSource {
    /// Accepting a new connection from the listener failed. The listener stays
    /// open.
    Accept,
    /// Registering or re-registering a connection with the poll registry
    /// failed. The connection is closed.
    Register,
    /// Reading from a connection failed. The connection is closed.
    Read,
    /// Writing to a connection failed. The connection is closed, discarding any
    /// response bytes still queued for it.
    Write,
    /// A handler panicked. The client receives a `500`, the connection is
    /// closed, and the server keeps running. The reported error carries the
    /// panic message; the panic hook still runs, so the location and any
    /// backtrace go to stderr as usual.
    Handler,
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub(crate) max_request_line_bytes: usize,
    pub(crate) max_header_bytes: usize,
    pub(crate) max_headers: usize,
    pub(crate) max_body_bytes: usize,
    pub(crate) max_pending_write_bytes: usize,
    pub(crate) read_buffer_capacity: usize,
    pub(crate) poll_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_request_line_bytes: 8 * 1024,
            max_header_bytes: 64 * 1024,
            max_headers: 128,
            max_body_bytes: 1024 * 1024,
            max_pending_write_bytes: 1024 * 1024,
            read_buffer_capacity: 8 * 1024,
            poll_timeout: Duration::from_millis(50),
        }
    }
}

impl ServerConfig {
    pub fn max_request_line_bytes(mut self, bytes: usize) -> Self {
        self.max_request_line_bytes = bytes;
        self
    }

    pub fn max_header_bytes(mut self, bytes: usize) -> Self {
        self.max_header_bytes = bytes;
        self
    }

    pub fn max_headers(mut self, count: usize) -> Self {
        self.max_headers = count;
        self
    }

    pub fn max_body_bytes(mut self, bytes: usize) -> Self {
        self.max_body_bytes = bytes;
        self
    }

    pub fn max_pending_write_bytes(mut self, bytes: usize) -> Self {
        self.max_pending_write_bytes = bytes;
        self
    }

    pub fn read_buffer_capacity(mut self, bytes: usize) -> Self {
        self.read_buffer_capacity = bytes;
        self
    }

    pub fn poll_timeout(mut self, timeout: Duration) -> Self {
        self.poll_timeout = timeout;
        self
    }
}
