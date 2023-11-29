use network_time_pester as pest;
use pest::TestResult;

fn main() -> anyhow::Result<()> {
    let server = "[::1]:123";
    // let server = "time.windows.com:123";

    let mut conn = pest::Connection::new(server)?;

    for test in pest::tests::all_tests() {
        let name = test.name().trim_start_matches("network_time_pester::");

        match test.run(&mut conn) {
            Ok(TestResult::Pass) => println!("✓ {name}"),
            Ok(TestResult::Fail(msg, None)) => println!("❌ {name}\n ↳ {msg}"),
            Ok(TestResult::Fail(msg, Some(r))) => println!("❌ {name}\n ↳ {msg}\n ↳ {r:#?}"),
            Err(e) => println!("❓ {name}:\n ↳ {e:#}"),
        }
    }

    Ok(())
}
