cargo new livereload
cd livereload
cargo add axum -F tokio
cargo add axum-macros
cargo add futures-core
cargo add minijinja -F source
cargo add minijinja-autoreload
cargo add tokio -F full
cargo add async_stream
cargo add tracing
cargo add tracing-subscriber
cargo add serde -F derive
cargo add serde_json
cargo add notify