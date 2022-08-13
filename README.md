# Vyper Compiler Version Manager in Rust

### Install
```
$ cargo install --git https://github.com/storming0x/vvm-rs --locked vvm-rs
```

### Install from source `vvm` and `vyper` (runner)
```
$ git clone https://github.com/storming0x/vvm-rs 
cd vvm-rs
cargo install --path ./ --bins --locked --force
```

### Manual Download

You can manually download release for your platform [here](https://github.com/storming0x/vvm-rs/releases)

### Usage
* List available versions
```
$ vvm list
```
* Install a version
```
$ vvm install <version>
```
* Use an installed version
```
$ vvm use <version>
```
* Remove an installed version
```
$ vvm remove <version>
```

### Note and Issues
VVM tries to use an environment variable called `GITHUB_TOKEN` to fetch and install vyper releases. In case its not found the installation may failed because of github rate limits

### Vyper Runner Usage

Vyper runner included in this repository proxies all commands to vyper compiler with an added layer of caching for all your vyper projects.

```
$ vyper <file-path-to-vyper-file>
```

Note: in case of issues with caching just delete the folder under `$HOME/.vvm/cache/`

Caching only supports one file as input on commands.


### Credit and Acknowledgments

* [SVM-RS](https://github.com/roynalnaruto/svm-rs)
* [VVM](https://github.com/vyperlang/vvm)

## Contributing

Help is always appreciated! Feel free to open an issue if you find a problem, or a pull request if you've solved an issue.

TODO: Contribution guide
