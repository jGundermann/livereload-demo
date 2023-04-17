use std::{path::PathBuf, sync::Arc, convert::Infallible};

use async_stream::try_stream;
use axum::{response::{IntoResponse, Html, Sse, sse::{Event, KeepAlive}}, http::StatusCode, Router, extract::State, routing::get};
use futures_core::Stream;
use minijinja::{Environment, Source};
use minijinja_autoreload::{AutoReloader};
use notify::{INotifyWatcher, Watcher, RecursiveMode};
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};

#[derive(Clone)]
pub(crate) struct AppState {
    renderer: Arc<AutoReloader>,
    notifier: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    _watcher: Arc<INotifyWatcher>
}

impl AppState {
    pub(crate) fn new() -> (Self, Router<AppState>) {
        let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
        let watcher_path = template_path.clone();

        let autoreloader = AutoReloader::new(|notifier| {
            let mut env = Environment::new();
            let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
            notifier.watch_path(&template_path, true);
            env.set_source(Source::from_path(template_path));
            Ok(env)
        });

        let (tx, rx) = mpsc::unbounded_channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
               Ok(event) => {
                if !event.kind.is_access() {
                    tx.send("updated".to_string()).unwrap();
                }
               },
               Err(e) => println!("watch error: {:?}", e),
            }
        }).unwrap();

        watcher.watch(&watcher_path, RecursiveMode::Recursive).unwrap();

        (Self {
            renderer: Arc::new(autoreloader),
            notifier: Arc::new(Mutex::new(rx)),
            _watcher: Arc::new(watcher)
        },
        add_livereload_router())
    }

    pub(crate) fn render<S>(&self, template_name: &str, ctx: S) -> impl IntoResponse where S: Serialize {
        let env = match self.renderer.acquire_env() {
            Ok(env) => env,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "no env").into_response(),
        };

        let template = match env.get_template(template_name) {
            Ok(template) => template,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "no template").into_response(),
        };

        let markup = match template.render(ctx) {
            Ok(markup) => markup,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Could not render").into_response(),
        };

        let markup = markup + r#"<script>
            let es = null;
            function initES() {
                if (es == null || es.readyState == 2) {
                    es = new EventSource('/dev/reload');
                    es.onerror = (e) => {
                        if (es.readyState == 2) {
                            setTimeout(initES, 5000);
                        }
                    };

                    es.onmessage = (e) => {
                        location.reload()
                    }
                }
            }
            initES();
        </script>"#;

        Html(markup).into_response()
    }

    async fn get_message(&self) -> Option<String> {
        self.notifier.lock().await.recv().await
    }
}

fn add_livereload_router() -> Router<AppState> {
    Router::new().route("/reload", get(event_handler))
}

#[axum_macros::debug_handler]
async fn event_handler(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(try_stream! {
        loop {
            if let Some(path) = state.get_message().await {
                yield Event::default().data(path);
            }
        }
    }).keep_alive(KeepAlive::default())
}