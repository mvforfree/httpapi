pub mod auth {
    use argon2::{self, Config};

    use actix_web::{web, HttpResponse, Responder, HttpRequest, HttpMessage};

    use crate::modules::dbrepo::*;
    use crate::modules::api_lib::*;
    use crate::AppContext;
    use validator::{Validate, ValidationError};
    use serde::{Deserialize,Serialize};
    use sqlx::{MySql, Row, MySqlPool, Pool, Error};
    use mysql::chrono::{DateTime, Utc};
    use crate::modules::staff_session::staff_session::{SessionRepo, Session};
    use std::fmt;
    use crate::modules::api_lib::api_response::{ApiResponse, ApiResponseStatus};

    const PASSWD_MIN_LEN: i32 = 12;
    const PASSWD_MAX_LEN: i32 = 64;
    const SALT_LEN: usize = 32;

    #[derive(Deserialize, Validate)]
    struct AddStaffRequest {
        project_id: i32,
        #[validate(email)]
        email: String,
        #[validate(length(min = "PASSWD_MIN_LEN", max = "PASSWD_MAX_LEN"))]
        password: String,
    }

    #[derive(Deserialize)]
    struct AuthStaffRequest {
        login: String,
        password: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct SimpleStaffRequest {
        project_id: i32,
    }

    #[derive(Deserialize, Serialize, Validate, sqlx::FromRow, Debug)]
    pub struct StaffUser {
        id: dbId,
        projectId:i32,
        #[validate(email)]
        email: String,
        createdAt: DateTime<Utc>,
        updatedAt: DateTime<Utc>,
        #[validate(email)]
        login: String,
        active: i8,
        deleted: i8,
        #[validate(length(min = 12, max = 64))]
        password: String,
        salt: String,
    }

    pub struct AuthError;

    impl fmt::Display for AuthError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Authentication error")
        }
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct ApiAuthSuccessResponse {
        code: i32,
        status: String,
        token: String,
    }

    impl ApiAuthSuccessResponse {
        fn new(code: i32, status: ApiResponseStatus, token: String) ->HttpResponse {
            let status_text = match status {
                ApiResponseStatus::Ok => "ok",
                ApiResponseStatus::Error => "error",
            };

            HttpResponse::Ok().content_type("text/json").json(
                ApiAuthSuccessResponse {
                    code,
                    status: String::from(status_text),
                    token
                }
            )
        }
    }

    // api routes
    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(web::scope("/staff")
            .route("auth",web::post().to(authorization))
            .route("add",web::post().to(add))
        );
    }

    async fn authorization(data: web::Data<AppContext>, request: web::Json<AuthStaffRequest>) -> impl Responder {
        let auth_result = auth_user(data.db.clone(),&request.login,&request.password).await;

        let auth_response = match auth_result {
            Ok(res) => ApiAuthSuccessResponse::new(
                0,
                ApiResponseStatus::Ok,
                String::from(res.get_token())),
            Err(e) => ApiResponse::new(
                1,
                ApiResponseStatus::Error,
                String::from(format!("Authentication error: {:?}", e))),
        };

        auth_response
    }

    async fn add(data: web::Data<AppContext>, request: web::Json<AddStaffRequest>) -> impl Responder {
        use std::str::*;
        let mut pool = data.db.clone();

        let exist_users = sqlx::query_as::<_,StaffUser>("SELECT * FROM staff_users WHERE login = ?")
            .bind(&request.email)
            .fetch_one(&pool).await;

        let user_exists = match exist_users {
            Ok(user) => true,
            Err(e) => false,
        };

        if user_exists {
            return HttpResponse::Ok().body(format!("User already exist: {}", request.email));
        }

        match request.validate() {
            Ok(_) => (),
            Err(e) => return ApiResponse::new(1, ApiResponseStatus::Error, format!("Validation error: {:?}", e))
        };

        //add user to db
        let login = request.email.as_str();
        let password = request.password.as_str();

        let salt = generate_random_salt();

        let hashedPassword = hash_password(password, &salt);

        let ir = sqlx::query("\
        INSERT INTO staff_users \
        (`id`,`projectId`,`email`,`createdAt`, `updatedAt`, `login`, `active`, `deleted`, `password`, `salt`)\
        VALUES \
        (NULL,?,?,NOW(),NOW(),?,?,?,?,?)")
            .bind(1)
            .bind(&request.email)
            .bind(login)
            .bind(1)
            .bind(0)
            .bind(hashedPassword)
            .bind(salt.iter().map(|b| *b as char).collect::<Vec<_>>().iter().collect::<String>())
            .execute(&pool).await;

        let inserted = match ir {
            Ok(user) => true,
            Err(e) => false,
        };

        if !inserted {
            return ApiResponse::new(1, ApiResponseStatus::Error, format!("Cant insert user."));
        }

        let e_users = sqlx::query_as::<_,StaffUser>("SELECT * FROM staff_users WHERE login = ?")
            .bind(&request.email)
            .fetch_one(&pool).await;

        let response = match e_users {
            Ok(user) => format!("User added: {} created {}", user.login, user.createdAt),
            Err(error) => format!("Everything bad: {:?}", error),
        };

        ApiResponse::new(0,ApiResponseStatus::Ok, response)
    }

    async fn auth_user(db: Pool<MySql>, user_login: &str, proposed_password: &str) -> Result<Session, Error> {
        let pool = db.clone();
        let sessions = SessionRepo::new(pool.clone());
        let lifetime = 3_600 * 30;

        let user = sqlx::query_as::<_, StaffUser>(
            "select *\
            from staff_users \
            where login = ? and active = 1 and deleted = 0"
        ).bind(user_login).fetch_one(&pool).await?;

        if verify_password(user.password.as_str(), proposed_password) {
            let res: Result<Session, Error>= sessions.add(user.id, lifetime).await;
            Ok(res.unwrap())
        } else {
            Err(Error::RowNotFound)
        }
    }

    fn generate_random_salt() -> [u8; SALT_LEN] {
        use openssl::rand::rand_bytes;
        let mut salt = [0; SALT_LEN];
        rand_bytes(&mut salt).unwrap();
        salt
    }

    fn hash_password(password: &str, salt: &[u8; SALT_LEN]) -> String {
        use std::str::*;
        use argon2::{self, Config, ThreadMode, Variant, Version};

        let config = Config {
            variant: Variant::Argon2i,
            version: Version::Version13,
            mem_cost: 4096,
            time_cost: 10,
            lanes: 4,
            thread_mode: ThreadMode::Parallel,
            secret: &[],
            ad: &[],
            hash_length: 32
        };

        argon2::hash_encoded(password.as_bytes(), salt, &config).expect("Unexpected error: can't generate argon2 hash.")
    }

    fn verify_password(encoded_password: &str, password: &str ) -> bool {
        use std::str::*;
        use argon2::{self};

        argon2::verify_encoded(encoded_password, password.as_bytes()).unwrap_or(false)
    }
}