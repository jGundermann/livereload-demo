use std::{path::PathBuf, sync::Arc};

use axum::{response::{IntoResponse, Html}, http::StatusCode};
use minijinja::{Environment, Source};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

#[derive(Clone)]
pub(crate) struct AppState {
    renderer: Arc<AutoReloader>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        let autoreloader = AutoReloader::new(|notifier| {
            let mut env = Environment::new();
            let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
            notifier.watch_path(&template_path, true);
            env.set_source(Source::from_path(template_path));
            Ok(env)
        });

        Self {
            renderer: Arc::new(autoreloader),
        }
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

        Html(markup).into_response()
    }
}