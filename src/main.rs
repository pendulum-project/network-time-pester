use std::{net::IpAddr, str::FromStr};

use clap::Parser;
use network_time_pester as pest;
use pest::TestResult;

const DEFAULT_SERVER: &str = "::1:123";
const DEFAULT_PORT: u16 = 123;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    server: Option<IpAddr>,
    #[arg(short, long)]
    port: Option<u16>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // TODO: Add a cli for server(s) and timeout
    let server = cli.server.unwrap_or(IpAddr::from_str(DEFAULT_SERVER)?);
    let port = cli.port.unwrap_or(DEFAULT_PORT);
    
    let mut conn = pest::Connection::new((server, port))?;

    let mut passed = 0;
    let mut failed = 0;
    for test in pest::all_tests() {
        let name = test.name().trim_start_matches("network_time_pester::");

        match test.run(&mut conn) {
            Ok(TestResult::Pass) => {
                passed += 1;
                println!("✅ {name}");
            },
            Ok(TestResult::Fail(msg, None)) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}")
            },
            Ok(TestResult::Fail(msg, Some(r))) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}\n ↳ {r:#?}")
            },
            Err(e) => println!("❓ {name}:\n ↳ {e:#}"),
        }
    }

    println!("\n✅ Passed: {passed}\n❌ Failed: {failed}");

    Ok(())
}
