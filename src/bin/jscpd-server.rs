use std::ffi::OsString;

use anyhow::{Result, bail};
use clap::Parser;
use jscpd_rs::cli::{Cli, Options};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{}", server_error_message(&error.to_string()));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let server_args = ServerArgs::from_env()?;
    if server_args.help {
        print_server_help();
        return Ok(());
    }
    if let Some(option) = server_args.unknown_option {
        eprintln!("error: unknown option '{option}'");
        std::process::exit(1);
    }
    let cli = Cli::parse_from(server_args.jscpd_args);
    if cli.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
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
    unknown_option: Option<String>,
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
        let mut unknown_option = None;
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
            } else if is_supported_jscpd_server_option(&arg) || !is_option_like(&arg) {
                jscpd_args.push(arg);
            } else {
                unknown_option = arg.to_str().map(str::to_string);
                break;
            }
        }

        Ok(Self {
            host,
            port,
            jscpd_args,
            help,
            unknown_option,
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

fn is_option_like(arg: &OsString) -> bool {
    arg.to_str().is_some_and(|value| value.starts_with('-'))
}

fn is_supported_jscpd_server_option(arg: &OsString) -> bool {
    let Some(value) = arg.to_str() else {
        return false;
    };
    let option = value
        .split_once('=')
        .map_or(value, |(option, _value)| option);
    matches!(
        option,
        "-V" | "--version"
            | "-c"
            | "--config"
            | "-f"
            | "--format"
            | "-i"
            | "--ignore"
            | "--ignore-pattern"
            | "-l"
            | "--min-lines"
            | "-k"
            | "--min-tokens"
            | "-x"
            | "--max-lines"
            | "-z"
            | "--max-size"
            | "-m"
            | "--mode"
            | "--store"
            | "--store-path"
            | "-a"
            | "--absolute"
            | "-n"
            | "--noSymlinks"
            | "--ignoreCase"
            | "-g"
            | "--gitignore"
            | "--skipLocal"
    )
}

fn server_help() -> &'static str {
    r#"Usage: jscpd-server [options] <path>

Start jscpd as a server

Options:
  -V, --version              output the version number
  -p, --port [number]        port to run the server on (Default is 3000)
  -H, --host [string]        host to bind the server to (Default is 0.0.0.0)
  -c, --config [string]      path to config file (Default is .jscpd.json in
                             <path>)
  -f, --format [string]      format or formats separated by comma
  -i, --ignore [string]      glob pattern for files to exclude
  --ignore-pattern [string]  ignore code blocks matching regexp patterns
  -l, --min-lines [number]   min size of duplication in code lines (Default is
                             5)
  -k, --min-tokens [number]  min size of duplication in code tokens (Default is
                             50)
  -x, --max-lines [number]   max size of source in lines (Default is 1000)
  -z, --max-size [string]    max size of source in bytes, examples: 1kb, 1mb,
                             120kb (Default is 100kb)
  -m, --mode [string]        mode of quality of search, can be "strict", "mild" and "weak" (Default is "function mild(token) {
    return strict(token) && token.type !== "empty" && token.type !== "new_line";
  }")
  --store [string]           use for define custom store (e.g. --store leveldb
                             used for big codebase)
  --store-path [string]      directory to use for store cache (e.g.
                             --store-path /tmp/jscpd-cache, useful when running
                             multiple instances in parallel)
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

fn server_error_message(message: &str) -> String {
    match message {
        "TypeError: mode is not a function" => {
            format!("Failed to start server: {message}")
        }
        message
            if message.starts_with("TypeError [ERR_INVALID_ARG_TYPE]")
                || message.starts_with("TypeError:")
                || message.starts_with("SyntaxError:") =>
        {
            message.to_string()
        }
        message => format!("Failed to start server: Error: {message}"),
    }
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
        assert_eq!(args.unknown_option, None);
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
        assert_eq!(args.unknown_option, None);
    }

    #[test]
    fn detects_server_help_without_forwarding_to_jscpd_cli() {
        let args = parse(&["jscpd-server", "--help"]);

        assert!(args.help);
        assert_eq!(args.unknown_option, None);
        let help = server_help();
        assert!(help.contains("Usage: jscpd-server [options] <path>"));
        assert!(help.contains("Start jscpd as a server"));
        assert!(help.contains("-p, --port [number]"));
        assert!(help.contains("-H, --host [string]"));
        assert!(
            help.contains("function mild(token)"),
            "server help should preserve upstream default mode text"
        );
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

    #[test]
    fn formats_server_start_errors_like_upstream() {
        assert_eq!(
            server_error_message("Invalid port number: true"),
            "Failed to start server: Error: Invalid port number: true"
        );
        assert_eq!(
            server_error_message("TypeError: cli.format.split is not a function"),
            "TypeError: cli.format.split is not a function"
        );
        assert_eq!(
            server_error_message("TypeError: mode is not a function"),
            "Failed to start server: TypeError: mode is not a function"
        );
    }

    #[test]
    fn rejects_options_not_supported_by_upstream_server() {
        for option in [
            "--list",
            "-h",
            "--reporters",
            "--output",
            "--debug",
            "--verbose",
            "--exitCode",
            "--noTips",
            "--skipComments",
            "--formats-exts",
            "--formats-names",
            "--pattern",
            "--blame",
            "--silent",
            "--threshold",
            "--no-gitignore",
        ] {
            let args = parse(&["jscpd-server", option]);

            assert_eq!(args.unknown_option, Some(option.to_string()));
        }
    }

    #[test]
    fn forwards_only_upstream_server_common_options() {
        let input = [
            "jscpd-server",
            "src",
            "-V",
            "--version",
            "-c",
            ".jscpd.json",
            "--config=custom.json",
            "-f",
            "javascript",
            "--format=typescript",
            "-i",
            "**/*.min.js",
            "--ignore=dist/**",
            "--ignore-pattern",
            "generated",
            "-l",
            "5",
            "--min-lines=6",
            "-k",
            "50",
            "--min-tokens=60",
            "-x",
            "1000",
            "--max-lines=2000",
            "-z",
            "1mb",
            "--max-size=2mb",
            "-m",
            "strict",
            "--mode=weak",
            "--store",
            "memory",
            "--store-path",
            ".cache",
            "-a",
            "--absolute",
            "-n",
            "--noSymlinks",
            "--ignoreCase",
            "-g",
            "--gitignore",
            "--skipLocal",
        ];
        let args = parse(&input);

        assert_eq!(args.unknown_option, None);
        assert_eq!(
            args.jscpd_args,
            input.iter().map(OsString::from).collect::<Vec<_>>()
        );
    }
}
