# Deltoid

![Rust](https://github.com/jjpe/deltoid/workflows/Rust/badge.svg)

## Synopsis

**Deltoid** is a rust library that can be used to calculate a [delta] `Δ`
between 2 values `a` and `b` of the same type.  Once calculated, `Δ` can
then be applied to the first value `a` to obtain a new value `c` that is
equivalent to the second value `b`.

A primary use case for calculating delta's is to keep track of a sequence of
related deeply-nested data trees while making sure to keep consumption of
resources (e.g. RAM, network bandwidth) reasonable. Since such a sequence may
be exported for further processing, delta's are by definition de/serializable.
This allows you to collect the data in once place as a sequence of delta's,
export it (perhaps over a network connection), and then reconstruct the series
on the receiving side by successively applying the delta's in the sequence.

[delta]: https://en.wikipedia.org/wiki/Delta_encoding

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
deltoid = "0.5"
deltoid-derive = "0.5"
```

Computing a delta, then applying it:

``` rust
use deltoid::Deltoid;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Deltoid)]
struct Point {
    x: usize,
    y: usize,
}

fn main() {
    // Define 2 instances of the same type
    let point0 = Point { x:  0, y: 0 };
    let point1 = Point { x: 42, y: 8 };

    // Calculate the delta between them
    let delta = point0.delta(&point1).unwrap();

    // Apply  the delta to `point0`
    let point2 = point0.apply(delta).unwrap();
    assert_eq!(point1, point2);
}
```

## Limitations

There are some limitations to this library:

1. Unions are not supported. Only `struct`s and `enum`s are currently supported.

2. The derive macro tries to accommodate generic types, but for types making
   use of advanced generics a manual implementation is generally recommended
   because it allows for finer control.
