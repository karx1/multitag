# multitag

[Documentation](https://docs.rs/multitag) | [GitHub](https://github.com/karx1/multitag) | [Crates.io](https://crates.io/crates/multitag)

`multitag` is a Rust crate for reading and writing music metadata in a variety of formats. It aims to fix some of the issues present in `audiotag`, such as adding `wav` file support.

### Supported Formats

| Format                    | Backend                                         |
| ------------------------- | ----------------------------------------------- |
| `mp3/wav/aiff`            | [`id3`](https://crates.io/crates/id3)           |
| `flac`                    | [`metaflac`](https://crates.io/crates/metaflac) |
| `mp4/m4a/m4p/m4b/m4r/m4a` | [`mp4ameta`](https://crates.io/crates/mp4ameta) |
| `opus`                    | [`opusmeta`](https://crates.io/crates/opusmeta) |
| `ogg`                     | [`oggmeta`](https://crates.io/crates/oggmeta)   |

PRs that add support for more formats are appreciated.

### Contributors

Thank you to everyone who has contributed to this repository!

<a href="https://github.com/karx1/multitag/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=karx1/multitag" />
</a>
