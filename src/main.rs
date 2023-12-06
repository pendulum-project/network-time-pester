use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;
use network_time_pester as pest;
use network_time_pester::root_ca;
use pest::{TestConfig, TestError};

#[derive(Parser)]
struct Cli {
    server: Option<SocketAddr>,

    #[arg(long)]
    nts_ke_server: Option<String>,

    #[arg(long, default_value_t = 4460)]
    nts_ke_port: u16,

    #[arg(long)]
    ca_file: Option<PathBuf>,

    #[arg(long, short, default_value = "100ms")]
    timeout: humantime::Duration,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = TestConfig {
        udp: cli.server,
        ke: cli
            .nts_ke_server
            .map(|hostname| (hostname, cli.nts_ke_port)),
        root_cert_store: root_ca(cli.ca_file)?,
        timeout: cli.timeout.into(),
    };

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    for test in pest::all_tests() {
        let name = test.name().trim_start_matches("network_time_pester::");

        match test.run(&config) {
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
            Err(TestError::Error(e)) => println!("❓ {name}:\n ↳ {e:#}"),
        }
    }

    println!("\n✅ Passed: {passed}\n❌ Failed: {failed}\n⏩ Skipped: {skipped}");

    Ok(())
}
