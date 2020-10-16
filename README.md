# delayTimer
Time-manager of delayed tasks.

[![Build](https://github.com/BinChengZhao/delay_timer/workflows/Build%20and%20test/badge.svg)](
https://github.com/BinChengZhao/delay_timer/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/BinChengZhao/delay_timer)
[![Cargo](https://img.shields.io/crates/v/delay_timer.svg)](
https://crates.io/BinChengZhao/delay_timer)
[![Documentation](https://docs.rs/delay_timer/badge.svg)](
https://docs.rs/delay_timer)

## Examples

Do what?:

```

fn main() -> io::Result<()> {
 //TODO
}
```

There's a lot more in the [examples] directory.


## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## To Do List
- [x]Disable unwrap related methods that will panic.
- [x] Thread and running task quit when delayTimer drop.
- [x] error handle need supplement.
- [x] neaten todo in code, replenish tests and benchmark.
- [x] when append that generate async-wrapper macro, ready for release.
- [x] append that generate bloking-wrapper macro, ready for release.
- [x] batch-opration.
- [x] report-for-server.
- [x] TASK-TAG.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.



#### The author comments:

#### Maybe can use that abstract a delivery server for MQ message  idempotent.

#### Make an upgrade plan for smooth updates in the future, Such as stop serve  back-up ` unfinished task`  then up new version serve load task.bak, Runing.
