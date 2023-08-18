# ngt-rs

[![crate]][crate-ngt] [![doc]][doc-ngt]

[crate]: https://img.shields.io/crates/v/ngt.svg
[crate-ngt]: https://crates.io/crates/ngt
[doc]: https://docs.rs/ngt/badge.svg
[doc-ngt]: https://docs.rs/ngt

Rust wrappers for [NGT][], which provides high-speed approximate nearest neighbor
searches against a large volume of data in high dimensional vector data space (several
ten to several thousand dimensions). The vector data can be `f32`, `u8`, or [f16][].

This crate provides the following indexes:
* [`NgtIndex`][index-ngt]: Graph and tree based index[^1]
* [`QgIndex`][index-qg]: Quantized graph based index[^2]
* [`QbgIndex`][index-qbg]: Quantized blob graph based index

Both quantized indexes are available through the `quantized` Cargo feature. Note that
they rely on `BLAS` and `LAPACK` which thus have to be installed locally. Furthermore,
`QgIndex` performances can be [improved][qg-optim] by using the `qg_optim` Cargo
feature.

The `NgtIndex` default implementation is an ANNG. It can be optimized[^3] or converted
to an ONNG through the [`optim`][ngt-optim] module.

By default `ngt-rs` will be built dynamically, which requires `CMake` to build NGT. This
means that you'll have to make the build artifact `libngt.so` available to your final
binary (see an example in the [CI][ngt-ci]). However the `static` feature will build and
link NGT statically. Note that `OpenMP` will also be linked statically. If the
`quantized` feature is used, then `BLAS` and `LAPACK` libraries will also be linked
statically.

NGT's [shared memory][ngt-sharedmem] and [large dataset][ngt-largedata] features are
available through the Cargo features `shared_mem` and `large_data` respectively.

[^1]: [Graph and tree based method explanation][ngt-desc]

[^2]: [Quantized graph based method explanation][qg-desc]

[^3]: [NGT index optimizations in Python][ngt-optim-py]

[ngt]: https://github.com/yahoojapan/NGT
[ngt-desc]: https://opensource.com/article/19/10/ngt-open-source-library
[ngt-sharedmem]: https://github.com/yahoojapan/NGT#shared-memory-use
[ngt-largedata]: https://github.com/yahoojapan/NGT#large-scale-data-use
[ngt-ci]: https://github.com/lerouxrgd/ngt-rs/blob/master/.github/workflows/ci.yaml
[ngt-optim]: https://docs.rs/ngt/latest/ngt/optim/index.html
[ngt-optim-py]: https://github.com/yahoojapan/NGT/wiki/Optimization-Examples-Using-Python
[qg-desc]: https://medium.com/@masajiro.iwasaki/fusion-of-graph-based-indexing-and-product-quantization-for-ann-search-7d1f0336d0d0
[qg-optim]: https://github.com/yahoojapan/NGT#build-parameters-1
[f16]: https://docs.rs/half/latest/half/struct.f16.html
[index-ngt]: https://docs.rs/ngt/latest/ngt/#usage
[index-qg]: https://docs.rs/ngt/latest/ngt/qg/
[index-qbg]: https://docs.rs/ngt/latest/ngt/qgb/
