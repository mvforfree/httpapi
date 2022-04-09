pub mod client_models {
    use actix_web::{web, HttpResponse, Responder};
    use sqlx::{Pool, MySql};

    use crate::modules::dbrepo::*;
    use crate::AppContext;

    use serde::{Serialize, Deserialize};
    use serde_json::*;

    use crate::modules::api_lib::api_response::{ApiResponse, ApiResponseStatus, ContextGuard};

    #[derive(Serialize, Deserialize, sqlx::FromRow)]
    pub struct CClient {
        id: dbId,
        name: String,
    }

    #[derive(Serialize, Deserialize, sqlx::FromRow)]
    pub struct ApiResponseClientsList {
        code: i32,
        status: String,
        clients: Vec<CClient>,
    }

    impl ApiResponseClientsList {
        pub fn new(code: i32, status: ApiResponseStatus, response: Vec<CClient>) ->HttpResponse {
            let status_text = match status {
                ApiResponseStatus::Ok => "ok",
                ApiResponseStatus::Error => "error",
            };

            HttpResponse::Ok().content_type("text/json").json(
                ApiResponseClientsList {
                    code,
                    status: String::from(status_text),
                    clients: response
                }
            )
        }
    }

    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(
                web::scope("/clients").wrap(ContextGuard)
                .route("list",web::get().to(list))
            //.route("get/{id}", web::get().to(get_by_id))
        );
    }

    async fn list(data: web::Data<AppContext>) -> impl Responder {
        let pool = data.db.clone();

        let clients_list = sqlx::query_as::<_,CClient>(
            "SELECT \
            id, \
            name \
            FROM clients")
            .fetch_all(&pool).await;

        let response = match clients_list {
            Ok(list) => ApiResponseClientsList::new(0, ApiResponseStatus::Ok, list),
            Err(e) => ApiResponse::new(1, ApiResponseStatus::Error, format!("Error receiving clients list: {:?}", e)),
        };

        response
    }
}
