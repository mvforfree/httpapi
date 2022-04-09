pub mod staff_session {
    use crate::modules::dbrepo::dbId;
    use sqlx::{MySql, Pool, Row, Error};
    use crate::modules::api_lib::api_response::{ApiResponse};

    #[derive(sqlx::FromRow, Debug)]
    pub struct Session {
        id: dbId,
        staffId: dbId,
        token: String,
        createdAt: u64,
        updatedAt: u64,
        expireAt: u64,
        locked: i16,
    }

    impl Session {
        pub fn get_token(&self) -> &str {
            &self.token
        }
    }

    #[derive(sqlx::FromRow, Debug)]
    pub struct SessionRepo {
        db: Pool<MySql>
    }

    impl SessionRepo {
        pub fn new(pool: Pool<MySql>) -> SessionRepo {
            SessionRepo {db: pool}
        }

        pub async fn get_by_token(&self, token: &str) -> Result<Session, Error> {
            let pool = self.db.clone();
            sqlx::query_as::<_, Session>("\
                select * from staff_session where token = ?
            ").bind(token).fetch_one(&pool).await
        }

        pub async fn add(&self, staff_id:dbId, lifetime: u64) -> Result<Session, Error> {
            let pool = self.db.clone();

            let session = sqlx::query("\
            INSERT INTO staff_session \
            (`id`, `staffId`, `token`, `createdAt`, `updatedAt`, `expireAt`, `locked`)\
            VALUES \
            (NULL,?,UUID(),unix_timestamp(),unix_timestamp(),unix_timestamp() + ?,?)")
                .bind(staff_id)
                .bind(lifetime)
                .bind(0)
                .execute(&pool).await?;

            let last_id = session.last_insert_id();

            self.get_by_id(last_id).await
        }

        pub async fn get_by_id(&self, id: dbId) -> Result<Session, Error> {
            let pool = self.db.clone();
            sqlx::query_as::<_, Session>("\
                select * from staff_session where id = ?
            ").bind(id).fetch_one(&pool).await
        }

        pub async fn is_valid(&self, token: &str) -> Result<bool, Error> {
            let pool = self.db.clone();

            // TODO: more sophisticated validation, ip check, cookies, etc
            let result = sqlx::query("\
                select \
                    count(*) as cnt \
                from staff_session \
                where \
                    token = ? \
                    and expireAt > unix_timestamp() \
                    and locked = 0")
                .bind(token)
                .fetch_one(&pool).await;

            let r: u64 = match result {
                Ok(row) => row.try_get("cnt")?,
                Err(e) => 0
            };

            if r > 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        // TODO: implement expiration
        pub async fn expire(&self, session: &Session) -> Result<bool, Error> {
            Ok(false)
        }

        // TODO: implement session lock
        pub async fn lock(&self, session: &Session) -> Result<bool, Error> {
            Ok(false)
        }
    }
}