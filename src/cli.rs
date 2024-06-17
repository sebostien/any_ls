use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[arg(long = "lsp")]
    pub lsp: bool,
    /// Increase logging verbosity.
    /// Use multiple times to increase it further.
    #[arg(short = 'v', long="verbose", action=clap::ArgAction::Count, global=true)]
    pub verbosity: u8,
}
