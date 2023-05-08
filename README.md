# ngt-rs &emsp; [![Latest Version]][crates.io] [![Latest Doc]][docs.rs]

[Latest Version]: https://img.shields.io/crates/v/ngt.svg
[crates.io]: https://crates.io/crates/ngt
[Latest Doc]: https://docs.rs/ngt/badge.svg
[docs.rs]: https://docs.rs/ngt

Rust wrappers for [NGT][], which provides high-speed approximate nearest neighbor
searches against a large volume of data in high dimensional vector data space (several
ten to several thousand dimensions).

This crate provides the following indexes:
* `NgtIndex`: Graph and tree-based index[^1]
* `QqIndex`: Quantized graph-based index[^2]
* `QbgIndex`: Quantized blob graph-based index

The quantized indexes are available through the `quantized` Cargo feature. Note that
they rely on `BLAS` and `LAPACK` which thus have to be installed locally. The CPU
running the code must also support `AVX2` instructions.

The `NgtIndex` default implementation is an ANNG, it can be optimized[^3] or converted
to an ONNG through the [`optim`][ngt-optim] module.

By default `ngt-rs` will be built dynamically, which requires `CMake` to build NGT. This
means that you'll have to make the build artifact `libngt.so` available to your final
binary (see an example in the [CI][ngt-ci]).

However the `static` feature will build and link NGT statically. Note that `OpenMP` will
also be linked statically. If the `quantized` feature is used, then `BLAS` and `LAPACK`
libraries will also be linked statically.

Finally, NGT's [shared memory][ngt-sharedmem] and [large dataset][ngt-largedata]
features are available through the features `shared_mem` and `large_data` respectively.

## Usage

Defining the properties of a new index:

```rust,ignore
use ngt::{NgtProperties, NgtDistance, NgtObject};

// Defaut properties with vectors of dimension 3
let prop = NgtProperties::dimension(3)?;

// Or customize values (here are the defaults)
let prop = NgtProperties::dimension(3)?
    .creation_edge_size(10)?
    .search_edge_size(40)?
    .object_type(NgtObject::Float)?
    .distance_type(NgtDistance::L2)?;
```

Creating/Opening an index and using it:

```rust,ignore
use ngt::{NgtIndex, NgtProperties, EPSILON};

// Create a new index
let prop = NgtProperties::dimension(3)?;
let index = NgtIndex::create("target/path/to/index/dir", prop)?;

// Open an existing index
let mut index = NgtIndex::open("target/path/to/index/dir")?;

// Insert two vectors and get their id
let vec1 = vec![1.0, 2.0, 3.0];
let vec2 = vec![4.0, 5.0, 6.0];
let id1 = index.insert(vec1)?;
let id2 = index.insert(vec2)?;

// Actually build the index (not yet persisted on disk)
// This is required in order to be able to search vectors
index.build(2)?;

// Perform a vector search (with 1 result)
let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
assert_eq!(res[0].id, id1);
assert_eq!(index.get_vec(id1)?, vec![1.0, 2.0, 3.0]);

// Remove a vector and check that it is not present anymore
index.remove(id1)?;
let res = index.get_vec(id1);
assert!(matches!(res, Result::Err(_)));

// Verify that now our search result is different
let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
assert_eq!(res[0].id, id2);
assert_eq!(index.get_vec(id2)?, vec![4.0, 5.0, 6.0]);

// Persist index on disk
index.persist()?;
```

[ngt]: https://github.com/yahoojapan/NGT
[ngt-sharedmem]: https://github.com/yahoojapan/NGT#shared-memory-use
[ngt-largedata]: https://github.com/yahoojapan/NGT#large-scale-data-use
[ngt-ci]: https://github.com/lerouxrgd/ngt-rs/blob/master/.github/workflows/ci.yaml
[ngt-optim]: https://docs.rs/ngt/latest/ngt/optim/index.html

[^1]: https://opensource.com/article/19/10/ngt-open-source-library
[^2]: https://medium.com/@masajiro.iwasaki/fusion-of-graph-based-indexing-and-product-quantization-for-ann-search-7d1f0336d0d0
[^3]: https://github.com/yahoojapan/NGT/wiki/Optimization-Examples-Using-Python
