use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue},
    routing::{get, patch, post},
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::api::handlers;
use crate::scene::StoryService;

#[derive(Clone)]
pub struct AppState {
    pub service: StoryService,
}

pub fn app(service: StoryService) -> Router {
    let state = AppState { service };

    let serve_dir = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));

    Router::new()
        .route(
            "/api/characters",
            get(handlers::list_characters).post(handlers::create_character),
        )
        .route(
            "/api/scenes",
            get(handlers::list_scenes).post(handlers::create_scene),
        )
        .route("/api/scenes/{scene_id}", get(handlers::get_scene))
        .route(
            "/api/scenes/{scene_id}/derive",
            post(handlers::derive_scene),
        )
        .route(
            "/api/scenes/{scene_id}/derivations",
            get(handlers::scene_derivations),
        )
        .route("/api/story-graph", get(handlers::story_graph))
        .route(
            "/api/relationship-candidates/{candidate_id}",
            patch(handlers::resolve_relationship_candidate),
        )
        .route(
            "/api/relationships/{fact_id}/history",
            get(handlers::relationship_history),
        )
        .fallback_service(serve_dir)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .with_state(state)
}
