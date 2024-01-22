## AFLRunner (WIP)

`AFL_Runner` is a simple CLI tool to make running efficient multi-core [AFLPlusPlus](https://github.com/AFLplusplus/AFLplusplus)
campaigns easier. The default configuration is based on the section [_Using multiple cores_](https://aflplus.plus/docs/fuzzing_in_depth/#c-using-multiple-cores)
of the official documentation.

The current implementation only accepts a small subset of `AFLPlusPlus` flags for some custom configuration.

## Usage

You can compile it yourself via:

```bash
git clone https://github.com/0xricksanchez/AFL_Runner.git alfrunner
cd aflrunner
cargo build --release
./target/release/afl_runner --help
```

Alternatively you can install via `crates.io`:

```bash
cargo install afl_runner
```

## TODO

- [ ] Add Tmux option to automatically create an appropriate layout for all runners
- [ ] Add more sensible defaults for other options
- [ ] Add more configuration options
