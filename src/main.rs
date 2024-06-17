use std::process::ExitCode;

use any_ls::Backend;
use flexi_logger::FileSpec;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() -> ExitCode {
    let _ = flexi_logger::Logger::try_with_env_or_str("debug")
        .expect("Could not create logger")
        .log_to_file(
            FileSpec::default()
                .suppress_timestamp()
                // TODO: Maybe change dir
                .directory("/home/sn/.config/any_ls/"),
        )
        .append()
        .start();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
    ExitCode::SUCCESS
}
