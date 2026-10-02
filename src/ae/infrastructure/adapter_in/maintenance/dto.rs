use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct EnqueueResponse {
    pub enqueued: usize,
}
