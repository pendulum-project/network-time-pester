use anyhow::Context;
use std::net::ToSocketAddrs;
use std::path::PathBuf;

use clap::Parser;
use network_time_pester as pest;
use network_time_pester::{NtsServer, Server};
use pest::{TestConfig, TestError};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(default_value = "localhost")]
    host: String,

    #[arg(long, short, default_value_t = 123)]
    port: u16,

    #[arg(long, short = 's')]
    nts: bool,

    #[arg(long, default_value_t = 4460)]
    ke_port: u16,

    #[arg(long, short, requires = "nts")]
    ca_file: Option<PathBuf>,

    #[arg(long, short, default_value = "100ms")]
    timeout: humantime::Duration,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = if cli.nts {
        let server = NtsServer::new(cli.host, cli.ke_port, cli.ca_file, cli.timeout.into())
            .context("Could not connect to NTS server to gather cookies and information")?;
        TestConfig {
            server: Server::Nts(server),
            timeout: cli.timeout.into(),
        }
    } else {
        let server = format!("{}:{}", cli.host, cli.port)
            .to_socket_addrs()
            .with_context(|| format!("Failed to lookup host: {:?}", cli.host))?
            .next()
            .with_context(|| format!("Host {:?} did not resolve into an IPs", cli.host))?;
        TestConfig {
            server: Server::Ntp(server),
            timeout: cli.timeout.into(),
        }
    };

    let mut passed = 0;
    let mut failed = 0;
    let mut errored = 0;
    let mut skipped = 0;
    for test in pest::all_tests() {
        let name = test.name().trim_start_matches("network_time_pester::");
        let config_ref = &config;

        match pest::util::catch_unwind(move || test.run(config_ref)) {
            Ok(()) => {
                passed += 1;
                println!("✅ {name}");
            }
            Err(TestError::Fail(msg, None)) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}")
            }
            Err(TestError::Fail(msg, Some(r))) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}\n ↳ {r:#?}")
            }
            Err(TestError::Skipped) => {
                skipped += 1;
                println!("⏩ {name}")
            }
            Err(TestError::Error(e)) => {
                errored += 1;
                println!("❓ {name}:\n ↳ {e:#}")
            }
        }
    }

    println!(
        "\n✅ Passed: {passed}\n❌ Failed: {failed}\n❓ Errored: {errored}\n⏩ Skipped: {skipped}"
    );

    Ok(())
}
