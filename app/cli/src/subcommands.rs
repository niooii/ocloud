use std::path::PathBuf;

use clap::Subcommand;
use url::Url;

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Upload files to the cloud.
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
    /// Set the cloud's base url.
    SetUrl { url: Url },
    /// Host the cloud with the given parameters.
    Server {
        #[command(subcommand)]
        command: ServerCommand
    }
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
    },
    /// Utilities for hosting a postgres instance.
    Db {
        #[command(subcommand)]
        command: DbCommand
    },
    /// Clears all data in the server, including uploaded files, etc.
    Wipe
}

/// TODO! what if user wants to use an external db, or not use docker?
/// What if they want to run this entire program in docker?
/// you fucking bum
#[derive(Subcommand, Debug)]
pub enum DbCommand {
    Start,
    Stop,
}