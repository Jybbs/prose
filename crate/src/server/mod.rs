//! `prose server`: a Language Server Protocol server over stdio.
//!
//! Editors reach prose through a long-lived process rather than a
//! per-save shellout. The server tracks each open buffer, formats it on
//! request, and republishes diagnostics on open and change, running the
//! same `Pipeline` the CLI runs so an editor session and `prose check`
//! agree on the active rule set.
//!
//! Layout: `dispatch` owns the handshake and the message loop,
//! `capabilities` advertises the server's surface and negotiates
//! position encoding, `documents` holds the live buffers, `config_cache`
//! resolves and memoizes each document's config, `analysis` runs the
//! pipeline over a buffer, and `conversion` maps between prose's byte
//! offsets and the protocol's line/character positions. This module holds
//! only the stdio glue, the one piece that resists unit testing.

use anyhow::Context;
use lsp_server::Connection;

use crate::cli::{
    args::{ServerArgs, Transport},
    exit_status::ExitStatus,
};

mod analysis;
mod capabilities;
mod config_cache;
mod conversion;
mod dispatch;
mod documents;

/// Serves the protocol over the requested transport until the client
/// disconnects or completes the shutdown handshake.
pub(crate) fn run(args: ServerArgs) -> anyhow::Result<ExitStatus> {
    match args.transport {
        Transport::Stdio => serve_stdio(),
    }
}

/// Runs the message loop over stdin and stdout, joining the reader and
/// writer threads once the loop returns.
fn serve_stdio() -> anyhow::Result<ExitStatus> {
    let (connection, io_threads) = Connection::stdio();
    dispatch::serve(connection)?;
    io_threads.join().context("joining server I/O threads")?;
    Ok(ExitStatus::Clean)
}
