use sqlx::PgPool;

#[derive(Clone)]
pub struct AdminState {
    pub db: PgPool,
    pub jwt_secret: String,
}
