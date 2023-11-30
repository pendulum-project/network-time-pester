use std::{net::SocketAddr, str::FromStr};

use clap::Parser;
use network_time_pester as pest;
use pest::TestResult;

#[derive(Parser)]
struct Cli {
    #[arg(default_value_t = SocketAddr::from_str("[::1]:123").expect("invalid socket address"))]
    server: SocketAddr,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // TODO: Add a cli for server(s) and timeout
    let mut conn = pest::Connection::new(cli.server)?;

    let mut passed = 0;
    let mut failed = 0;
    for test in pest::all_tests() {
        let name = test.name().trim_start_matches("network_time_pester::");

        match test.run(&mut conn) {
            Ok(TestResult::Pass) => {
                passed += 1;
                println!("✅ {name}");
            }
            Ok(TestResult::Fail(msg, None)) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}")
            }
            Ok(TestResult::Fail(msg, Some(r))) => {
                failed += 1;
                println!("❌ {name}\n ↳ {msg}\n ↳ {r:#?}")
            }
            Err(e) => println!("❓ {name}:\n ↳ {e:#}"),
        }
    }

    println!("\n✅ Passed: {passed}\n❌ Failed: {failed}");

    Ok(())
}
