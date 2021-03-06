# Vyper Compiler Version Manager in Rust

### Install
```
$ cargo install --git https://github.com/storming0x/vvm-rs --locked vvm-rs
```

### Install from source
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

### Credit and Acknowledgments

* [SVM-RS](https://github.com/roynalnaruto/svm-rs)
* [VVM](https://github.com/vyperlang/vvm)

## Contributing

Help is always appreciated! Feel free to open an issue if you find a problem, or a pull request if you've solved an issue.

TODO: Contribution guide
