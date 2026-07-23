use crate::models::StoryError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub field_errors: Vec<FieldError>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field_errors: Vec::new(),
        }
    }

    pub fn with_field(mut self, field: impl Into<String>, message: impl Into<String>) -> Self {
        self.field_errors.push(FieldError {
            field: field.into(),
            message: message.into(),
        });
        self
    }
}

impl From<StoryError> for ApiError {
    fn from(value: StoryError) -> Self {
        Self::new("story_error", value.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "validation_error" => StatusCode::UNPROCESSABLE_ENTITY,
            "derivation_error" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, axum::Json(self)).into_response()
    }
}

pub type ApiResult<T> = Result<axum::Json<T>, ApiError>;
