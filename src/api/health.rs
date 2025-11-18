use poem_openapi::{ApiResponse, OpenApi};

pub struct HealthApi;

#[derive(ApiResponse)]
pub enum HealthResponse {
    #[oai(status = 200)]
    Ok,
}

#[OpenApi]
impl HealthApi {
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> HealthResponse {
        HealthResponse::Ok
    }
}

