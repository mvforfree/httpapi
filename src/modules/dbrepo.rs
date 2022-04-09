use sqlx::*;

pub type dbId = u64;

pub trait dbRepo<T> {
    fn create(pool: Pool<MySql>) -> Self;

    fn get_by_id(&self, identifier: dbId) -> Result<T>;

    fn query_one(&self, query: &str) -> Result<T>;

    fn query_all(&self, query: &str) -> Result<Vec<T>>;
}
