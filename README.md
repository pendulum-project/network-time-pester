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
```

Or IPv4:
```bash
$ cargo run -- 0.0.0.0:123
```
