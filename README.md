# Network Time Pester

A work-in-progress test tool for Network Time Protocol implementations. Use at your own risk and feel free to contribute your own tests or suggest ideas!

## Usage

To run the tests against a server, run:
```bash
$ cargo run -- [SERVER_ADDRESS]
```

For example:
```bash
$ cargo run -- [::1]:123
# or with IPv4
$ cargo run -- 127.0.0.1:123
```

### Options
| Short | Long      | Description                                                                                                                                                                                                 |
|-------|-----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| -p    | --port    | The port to use for NTP, default `123`, can not be used with NTS, where the port is detected from the NTS-KE server.                                                                                        |
| -t    | --timeout | The maximum time to wait for a response before concluding there never will be one. Default 100ms.                                                                                                           |
| -s    | --nts     | Use Network Time Security (NTS). This will enable more tests that assume the server is NTS capable.                                                                                                         |
|       | --ke-port | Port that should be used for the NTS key establishment protocol. Default `4460`.                                                                                                                            |
| -c    | --ca-file | Path to a `.pem` file that contains the public key of the trusted CA. For an example of how to generate a CA for testing see the [ntpd-rs docs](https://docs.ntpd-rs.pendulum-project.org/development/ca/). |
| -h    | --help    | Display a brief description of the available options                                                                                                                                                        |

For example:
```bash
# Run against a local NTP server listening on port 1123, and only wait 10ms for a reply
$ cargo run -- --port 1123 --timeout 10ms localhost
```

Or with NTS:
```bash
$ cargo run -- --nts --ca-file ca-data/ca.pem ntpd-rs.test
```

Since NTS uses TLS it requires a hostname instead of an IP. This can be done by adding a line to `/etc/hosts` or 
similar. See [ntpd-rs docs](https://docs.ntpd-rs.pendulum-project.org/development/ca/) for an example.

## Output
The test report is printed as the tests are executed. The first part lists one test result and test case name per line.
Followed by statistics on how many results happend.

For example:
```text
$ cargo run -- localhost
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/network-time-pester localhost`
✅ tests::basic::test_responds_to_version_4
❌ tests::basic::test_ignores_version_5
 ↳ After test: Server did no longer reply to normal poll
✅ tests::extensions::test_unknown_extensions_are_ignored
❓ tests::extensions::test_unique_id_is_returned:
 ↳ Can not connect to 127.0.0.1:123 from 0.0.0.0:0: Network is unreachable (os error 101)
⏩ tests::nts::happy
⏩ tests::nts_ke::happy
⏩ tests::nts_ke::error_on_unknown_next_protocol
⏩ tests::nts_ke::ignore_unknown_extra_protocols
⏩ tests::nts_ke::error_on_unknown_aead
⏩ tests::nts_ke::ignore_unknown_extra_aead
⏩ tests::nts_ke::empty_message_resolves_in_error

✅ Passed: 2
❌ Failed: 1
❓ Errored: 1
⏩ Skipped: 7
```

### Results
| Symbol | Name    | Description                                                                                                                                                      |
|--------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| ✅      | Passed  | The test was executed and all checks passed.                                                                                                                     |
| ❌      | Failed  | During execution one of the checks failed. A description of the failure is added on a new line.                                                                  |
| ❓      | Error   | During execution an error occurred, this could mean that the server did not behave as expected or another issue occured. (e.g. the network connection was lost). |
| ⏩      | Skipped | The test was not executed because it needed a different connection. (e.g. when running without the `--nts` flag.                                                 |

*Note:* Symbols might appear different depending on the terminal font.

### Tests names
The tests are named after their Rust module paths. For example `tests::basic::test_responds_to_version_4` can be found 
in [`src/tests/basic.rs`](src/tests/basic.rs) in the function `test_responds_to_version_4`.

# Contributing
This tool is a work in progress that was developed to test edge cases of NTP implementations. If you have any
**thoughts**, **ideas for tests**, or **test implementations** feel free to 
[open an issue](https://github.com/pendulum-project/ntpd-rs/issues/new) on this repository.

For an example of a small test case see [`tests::extensions::test_unknown_extensions_are_ignored`](src/tests/extensions.rs).
The existing test cases use the [`ntp-proto`](https://crates.io/crates/ntp-proto) crate, but packets can also just be constructed byte-by-byte.