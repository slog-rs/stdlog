## slog-scope-stdlog - Standard Rust log crate adapter for [slog-rs] - integrated with [`slog-scope`][slog-scope]

<p align="center">
  <a href="https://travis-ci.org/slog-rs/scope-stdlog">
      <img src="https://img.shields.io/travis/slog-rs/scope-stdlog/master.svg" alt="Travis CI Build Status">
  </a>

  <a href="https://crates.io/crates/slog-scope-stdlog">
      <img src="https://img.shields.io/crates/d/slog-scope-stdlog.svg" alt="slog-scope-stdlog on crates.io">
  </a>

  <a href="https://gitter.im/slog-rs/slog">
      <img src="https://img.shields.io/gitter/room/slog-rs/slog.svg" alt="slog-rs Gitter Chat">
  </a>
</p>

This is a preferred (over original `slog-stdlog`) method of backward
compatibility with legacy `log` crate.

The difference is: this library does not define own logging scopes
functionality, and instead relies on `slog_scope::scope`.

[slog-rs]: //github.com/slog-rs/slog
[slog-scope]: //github.com/slog-rs/scope
