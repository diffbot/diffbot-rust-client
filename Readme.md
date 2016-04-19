# Diffbot API client for Rust

[![Build Status](https://travis-ci.org/diffbot/diffbot-rust-client.svg?branch=master)](https://travis-ci.org/diffbot/diffbot-rust-client)
[![crates.io](http://meritbadge.herokuapp.com/diffbot)](https://crates.io/crates/diffbot)

This library allows you to access the [Diffbot API](https://www.diffbot.com)
from your rust application.
You still need a diffbot token (check their [trial](https://www.diffbot.com/plans/trial)).

It returns a Json object from [rustc_serialize](https://doc.rust-lang.org/rustc-serialize/rustc_serialize/json/index.html),
which is basically a [BTreeMap](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html).

## [Documentation](http://diffbot.github.io/diffbot-rust-client/diffbot/index.html)

## Installation

Add to your `Cargo.toml` dependencies:

```
[dependencies]
diffbot = "0.2"
```

And to your main source file:

```rust
extern crate diffbot;
```

## Usage

```rust
extern crate diffbot;
use diffbot::*;

fn main() {
    let client = Diffbot::v3("insert_your_token_here");
    match client.call(API::Analyze, "http://www.diffbot.com") {
        Ok(result) =>
            println!("{:?}", result),
        Err(Error::Api(code, msg)) =>
            println!("API returned error {}: {}", code, msg),
        Err(err) =>
            println!("Other error: {:?}", err),
    };
}
```


```rust
extern crate diffbot;
use diffbot::*;

fn main() {
	let client = Diffbot::v3("insert_your_token_here");
	match client.search("GLOBAL-INDEX", "type:article diffbot") {
        Ok(result) =>
            println!("{:?}", result),
        Err(Error::Api(code, msg)) =>
            println!("API returned error {}: {}", code, msg),
        Err(err) =>
            println!("Other error: {:?}", err),
	};
}

```

## License

This library is under the MIT license. You can probably use it in your commercial application without complication.
