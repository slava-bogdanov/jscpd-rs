use std::ffi::OsString;

use anyhow::{Result, bail};
use clap::Parser;
use jscpd_rs::cli::{Cli, Options};
use jscpd_rs::formats;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to start server: Error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let server_args = ServerArgs::from_env()?;
    if server_args.help {
        print_server_help();
        return Ok(());
    }
    let cli = Cli::parse_from(server_args.jscpd_args);
    if cli.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if cli.list {
        println!(
            "Supported formats: \n{}",
            formats::supported_formats().join(", ")
        );
        return Ok(());
    }
    let options = Options::from_cli(cli)?;
    jscpd_rs::server::serve(options, &server_args.host, server_args.port).await
}

#[derive(Debug)]
struct ServerArgs {
    host: String,
    port: u16,
    jscpd_args: Vec<OsString>,
    help: bool,
}

impl ServerArgs {
    fn from_env() -> Result<Self> {
        Self::parse(std::env::args_os())
    }

    fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter().collect::<Vec<_>>();
        let program = args
            .first()
            .cloned()
            .unwrap_or_else(|| OsString::from("jscpd-server"));
        if !args.is_empty() {
            args.remove(0);
        }
        let mut args = args.into_iter().peekable();
        let mut host = "0.0.0.0".to_string();
        let mut port = 3000u16;
        let mut help = false;
        let mut jscpd_args = vec![program];

        while let Some(arg) = args.next() {
            if arg == "--help" {
                help = true;
            } else if arg == "--host" || arg == "-H" {
                host = next_optional_value(&mut args).unwrap_or_else(|| "true".to_string());
            } else if arg == "--port" || arg == "-p" {
                let value = next_optional_value(&mut args).unwrap_or_else(|| "true".to_string());
                port = parse_port(&value)?;
            } else if let Some(value) = prefixed_value(&arg, "--host=") {
                host = value;
            } else if let Some(value) = prefixed_value(&arg, "--port=") {
                port = parse_port(&value)?;
            } else {
                jscpd_args.push(arg);
            }
        }

        Ok(Self {
            host,
            port,
            jscpd_args,
            help,
        })
    }
}

fn next_optional_value<I>(args: &mut std::iter::Peekable<I>) -> Option<String>
where
    I: Iterator<Item = OsString>,
{
    let next = args.peek()?;
    if next.to_str().is_some_and(|value| value.starts_with('-')) {
        return None;
    }
    args.next().and_then(|value| value.into_string().ok())
}

fn print_server_help() {
    println!("{}", server_help());
}

fn server_help() -> &'static str {
    r#"Usage: jscpd-server [options] <path>

Start jscpd as a server

Options:
  -V, --version              output the version number
  -p, --port [number]        port to run the server on (Default is 3000)
  -H, --host [string]        host to bind the server to (Default is 0.0.0.0)
  -c, --config [string]      path to config file (Default is .jscpd.json in <path>)
  -f, --format [string]      format or formats separated by comma
  -i, --ignore [string]      glob pattern for files to exclude
  --ignore-pattern [string]  ignore code blocks matching regexp patterns
  -l, --min-lines [number]   min size of duplication in code lines (Default is 5)
  -k, --min-tokens [number]  min size of duplication in code tokens (Default is 50)
  -x, --max-lines [number]   max size of source in lines (Default is 1000)
  -z, --max-size [string]    max size of source in bytes, examples: 1kb, 1mb, 120kb (Default is 100kb)
  -m, --mode [string]        mode of quality of search, can be "strict", "mild" and "weak"
  --store [string]           use for define custom store (e.g. --store leveldb used for big codebase)
  --store-path [string]      directory to use for store cache (e.g. --store-path /tmp/jscpd-cache, useful when running multiple instances in parallel)
  -a, --absolute             use absolute path in reports
  -n, --noSymlinks           dont use symlinks for detection
  --ignoreCase               ignore case of symbols in code (experimental)
  -g, --gitignore            ignore all files from .gitignore file
  --skipLocal                skip duplicates in local folders
  --help                     display help for command"#
}

fn prefixed_value(arg: &OsString, prefix: &str) -> Option<String> {
    arg.to_str()
        .and_then(|value| value.strip_prefix(prefix))
        .map(str::to_string)
}

fn parse_port(value: &str) -> Result<u16> {
    let Ok(port) = value.parse::<u16>() else {
        bail!("Invalid port number: {value}");
    };
    if port == 0 {
        bail!("Invalid port number: {value}");
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> ServerArgs {
        ServerArgs::parse(args.iter().map(OsString::from)).expect("parse server args")
    }

    #[test]
    fn extracts_server_host_and_port() {
        let args = parse(&[
            "jscpd-server",
            ".",
            "--host",
            "127.0.0.1",
            "--port",
            "4567",
            "--format",
            "javascript",
        ]);

        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 4567);
        assert_eq!(
            args.jscpd_args,
            vec![
                OsString::from("jscpd-server"),
                OsString::from("."),
                OsString::from("--format"),
                OsString::from("javascript"),
            ]
        );
    }

    #[test]
    fn supports_equals_server_flags() {
        let args = parse(&["jscpd-server", "--host=localhost", "--port=3001", "src"]);

        assert_eq!(args.host, "localhost");
        assert_eq!(args.port, 3001);
        assert_eq!(
            args.jscpd_args,
            vec![OsString::from("jscpd-server"), OsString::from("src")]
        );
    }

    #[test]
    fn detects_server_help_without_forwarding_to_jscpd_cli() {
        let args = parse(&["jscpd-server", "--help"]);

        assert!(args.help);
        let help = server_help();
        assert!(help.contains("Usage: jscpd-server [options] <path>"));
        assert!(help.contains("Start jscpd as a server"));
        assert!(help.contains("-p, --port [number]"));
        assert!(help.contains("-H, --host [string]"));
        assert!(!help.contains("detector of copy/paste in files"));
    }

    #[test]
    fn bare_or_invalid_server_port_matches_upstream_error() {
        let error = ServerArgs::parse(["jscpd-server", "--port"].into_iter().map(OsString::from))
            .expect_err("bare port should fail");
        assert_eq!(error.to_string(), "Invalid port number: true");

        let error = ServerArgs::parse(
            ["jscpd-server", "--port", "abc"]
                .into_iter()
                .map(OsString::from),
        )
        .expect_err("invalid port should fail");
        assert_eq!(error.to_string(), "Invalid port number: abc");
    }
}
