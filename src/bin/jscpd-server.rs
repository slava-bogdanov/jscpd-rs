use std::ffi::OsString;

use anyhow::{Context, Result, bail};
use clap::Parser;
use jscpd_rs::cli::{Cli, Options};
use jscpd_rs::formats;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let server_args = ServerArgs::from_env()?;
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

struct ServerArgs {
    host: String,
    port: u16,
    jscpd_args: Vec<OsString>,
}

impl ServerArgs {
    fn from_env() -> Result<Self> {
        Self::parse(std::env::args_os())
    }

    fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let program = args
            .next()
            .unwrap_or_else(|| OsString::from("jscpd-server"));
        let mut host = "0.0.0.0".to_string();
        let mut port = 3000u16;
        let mut jscpd_args = vec![program];

        while let Some(arg) = args.next() {
            if arg == "--host" || arg == "-H" {
                host = next_value(&mut args, "--host")?;
            } else if arg == "--port" || arg == "-p" {
                port = parse_port(&next_value(&mut args, "--port")?)?;
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
        })
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .with_context(|| format!("missing value for {flag}"))?
        .into_string()
        .map_err(|_| anyhow::anyhow!("{flag} value must be valid UTF-8"))
}

fn prefixed_value(arg: &OsString, prefix: &str) -> Option<String> {
    arg.to_str()
        .and_then(|value| value.strip_prefix(prefix))
        .map(str::to_string)
}

fn parse_port(value: &str) -> Result<u16> {
    let port = value
        .parse::<u16>()
        .with_context(|| format!("invalid --port value `{value}`"))?;
    if port == 0 {
        bail!("--port must be greater than 0");
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
}
