use network_time_pester as pest;
use network_time_pester::TestResult;

fn main() -> anyhow::Result<()> {
    let mut conn = pest::Connection::new("[::1]:123")?;

    for test in network_time_pester::tests() {
        let name = test.name().trim_start_matches("network_time_pester::");

        match test.run(&mut conn) {
            Ok(TestResult::Pass) => println!("✓ {name}"),
            Ok(TestResult::Fail) => println!("❌ {name}"),
            Err(e) => println!("❓ {name}:\n ↳ {e:#}"),
        }
    }

    Ok(())
}
