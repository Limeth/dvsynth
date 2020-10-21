# DVSynth — Digital Video Synthesizer
DVSynth is a real-time graph-based video compositor for broadcasting and creative coding.

## Development
When working on the projects, it helps to be able to change the source code for some of the dependencies. This can be done by cloning the source code of the dependency and then adding the path to [`~/.cargo/config.toml`](https://doc.rust-lang.org/cargo/reference/config.html):

```toml
paths = [ "workspace/rust/iced" ]
```

This way, `Cargo.toml` can be left unchanged and the local source code for those dependencies will be used.
