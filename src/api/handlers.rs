use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use crate::api::dto::*;
use crate::api::errors::ApiResult;
use crate::api::relationship_dtos::*;
use crate::api::scene_dtos::*;

pub async fn list_characters(
    State(state): State<crate::api::routes::AppState>,
) -> ApiResult<Vec<CharacterDto>> {
    let characters = state.service.list_characters().await?;
    let dtos = characters.into_iter().map(CharacterDto::from).collect();
    Ok(Json(dtos))
}

pub async fn create_character(
    State(state): State<crate::api::routes::AppState>,
    Json(body): Json<CreateCharacterRequest>,
) -> ApiResult<CharacterDto> {
    if body.name.trim().is_empty() {
        return Err(
            crate::api::errors::ApiError::new("validation_error", "name is required")
                .with_field("name", "must not be blank"),
        );
    }
    let id = crate::models::CharacterId(Uuid::new_v4());
    state
        .service
        .db()
        .characters()
        .create(id, &body.name, &body.personality, &body.skills)
        .await?;
    let character = state.service.db().characters().get(id).await?;
    let character = character
        .ok_or_else(|| crate::api::errors::ApiError::new("not_found", "character not created"))?;
    Ok(Json(character.into()))
}

pub async fn list_scenes(
    State(state): State<crate::api::routes::AppState>,
) -> ApiResult<Vec<SceneDto>> {
    let scenes = state.service.list_scenes().await?;
    let dtos = scenes.into_iter().map(SceneDto::from).collect();
    Ok(Json(dtos))
}

pub async fn create_scene(
    State(state): State<crate::api::routes::AppState>,
    Json(body): Json<CreateSceneRequest>,
) -> ApiResult<SceneDto> {
    if body.objective_event.trim().is_empty() {
        return Err(crate::api::errors::ApiError::new(
            "validation_error",
            "objective_event is required",
        )
        .with_field("objective_event", "must not be blank"));
    }
    let participant_ids = body
        .participant_ids
        .iter()
        .map(|s| crate::models::CharacterId(Uuid::parse_str(s).unwrap_or(Uuid::nil())))
        .collect();
    let occurred_at = chrono::DateTime::parse_from_rfc3339(&body.occurred_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let scene_id = state
        .service
        .create_scene(crate::models::CreateScene {
            objective_event: body.objective_event,
            participant_ids,
            occurred_at,
        })
        .await?;
    let scene = state.service.get_scene(scene_id).await?;
    let scene =
        scene.ok_or_else(|| crate::api::errors::ApiError::new("not_found", "scene not created"))?;
    Ok(Json(scene.into()))
}

pub async fn get_scene(
    State(state): State<crate::api::routes::AppState>,
    Path(scene_id): Path<String>,
) -> ApiResult<SceneDto> {
    let scene_id = crate::models::SceneId(Uuid::parse_str(&scene_id).unwrap_or(Uuid::nil()));
    let scene = state.service.get_scene(scene_id).await?;
    let scene = scene.ok_or_else(|| {
        crate::api::errors::ApiError::new("not_found", format!("scene {:?} not found", scene_id))
    })?;
    Ok(Json(scene.into()))
}

pub async fn derive_scene(
    State(state): State<crate::api::routes::AppState>,
    Path(scene_id): Path<String>,
) -> ApiResult<Vec<BatchDerivationResultDto>> {
    let scene_id = crate::models::SceneId(Uuid::parse_str(&scene_id).unwrap_or(Uuid::nil()));
    let results = state.service.derive_scene(scene_id).await;
    let mut dtos = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(derivation) => {
                let dto = DerivationDto::from(derivation);
                dtos.push(BatchDerivationResultDto {
                    derivation: Some(dto),
                    error: None,
                });
            }
            Err(e) => {
                dtos.push(BatchDerivationResultDto {
                    derivation: None,
                    error: Some(crate::api::errors::ApiError::new(
                        "derivation_error",
                        e.to_string(),
                    )),
                });
            }
        }
    }
    Ok(Json(dtos))
}

pub async fn scene_derivations(
    State(state): State<crate::api::routes::AppState>,
    Path(scene_id): Path<String>,
) -> ApiResult<Vec<SceneDerivationDto>> {
    let scene_id = crate::models::SceneId(Uuid::parse_str(&scene_id).unwrap_or(Uuid::nil()));
    let details = state.service.scene_derivations(scene_id).await?;
    let dtos = details
        .into_iter()
        .map(|detail| SceneDerivationDto {
            character: detail.character.into(),
            memory: detail.memory.into(),
            sensation: detail.sensation.into(),
            plot_developments: detail
                .plot_developments
                .into_iter()
                .map(PlotDto::from)
                .collect(),
            relationship_candidates: detail
                .relationship_candidates
                .into_iter()
                .map(CandidateDto::from)
                .collect(),
        })
        .collect();
    Ok(Json(dtos))
}

pub async fn story_graph(
    State(state): State<crate::api::routes::AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<GraphSnapshotDto> {
    let scene_id = params
        .get("scene_id")
        .or_else(|| params.get("through"))
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(crate::models::SceneId)
        .unwrap_or(crate::models::SceneId(Uuid::nil()));
    let snapshot = state.service.story_graph_through(scene_id).await?;
    Ok(Json(snapshot.into()))
}

pub async fn resolve_relationship_candidate(
    State(state): State<crate::api::routes::AppState>,
    Path(candidate_id): Path<String>,
    Json(body): Json<ResolveCandidateRequest>,
) -> ApiResult<()> {
    let candidate_id = crate::models::RelationshipCandidateId(
        Uuid::parse_str(&candidate_id).unwrap_or(Uuid::nil()),
    );
    let action = body.decision.action.to_lowercase();
    let resolution = if action == "accept" {
        crate::db::CandidateResolution::accept(
            body.decision.summary.unwrap_or_default(),
            body.decision.scores.map(|s| crate::db::RelationshipScores {
                tension: s.tension,
                trust: s.trust,
                affection: s.affection,
                power: s.power,
            }),
        )
    } else {
        crate::db::CandidateResolution::reject()
    };
    state
        .service
        .resolve_relationship_candidate(candidate_id, resolution)
        .await?;
    Ok(Json(()))
}

pub async fn relationship_history(
    State(state): State<crate::api::routes::AppState>,
    Path(fact_id): Path<String>,
) -> ApiResult<Vec<RelationshipRevisionDto>> {
    let fact_id =
        crate::models::RelationshipFactId(Uuid::parse_str(&fact_id).unwrap_or(Uuid::nil()));
    let history = state.service.relationship_history(fact_id).await?;
    let dtos = history
        .into_iter()
        .map(RelationshipRevisionDto::from)
        .collect();
    Ok(Json(dtos))
}
