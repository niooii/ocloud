use std::path::PathBuf;

use clap::Subcommand;
use url::Url;

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Upload files to oCloud.
    Upload { 
        path: PathBuf,
        /// Preserve the directory structure relative to the cwd when uploading.
        /// Ex: ocloud upload -p ./books/fiction/AM.pdf will be uploaded to
        /// endpoint.com/media/root/books/fiction/AM.pdf
        /// rather than endpoint.com/media/root/AM.pdf without the preserve flag.
        #[arg(short = 'p', long = "preserve")]
        preserve: bool,
        /// The target directory to upload the file to.
        #[arg(short = 'd', long = "dir", default_value = "")]
        dir: String,
    },
    /// Set the base url of the server to use.
    SetUrl { url: Url },
    /// Manage or run the oCloud server.
    Server {
        #[command(subcommand)]
        command: ServerCommand
    },
    /// Print the paths that oCloud uses.
    Paths
}

#[derive(Subcommand, Debug)]
pub enum ServerCommand {
    /// Runs the server.
    Run {
        /// The host to use when starting the server.
        #[arg(short = 'H', long = "host", default_value="0.0.0.0")]
        host: String,
        /// The port to use when starting the server.
        #[arg(short = 'p', long = "port", default_value="443")]
        port: u16,
        /// Run with an embedded postgres database instead of connecting to an external one.
        #[arg(short = 'd', long = "database")]
        embedded_db: bool
    },
    /// Clears all data in the server, including uploaded files, etc.
    Wipe
}