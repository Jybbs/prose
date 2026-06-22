//! The `server` subcommand args and its transport value-enum.

#[derive(Debug, Default, clap::Args)]
pub(crate) struct ServerArgs {
    /// Transport the server speaks over. Only stdio is supported.
    #[arg(long, value_enum, default_value_t)]
    pub(crate) transport: Transport,
}

/// Transport the language server speaks over.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub(crate) enum Transport {
    /// Communicate over stdin and stdout.
    #[default]
    Stdio,
}
