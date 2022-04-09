use std::env;
use actix_web::{App, HttpServer, web};

pub mod modules;
use dotenv;

use sqlx::mysql::MySqlPoolOptions;

// services cfg aliases
use modules::client::client_models as clients;
use modules::auth::auth as auth;

const PROJECT_NAME: &str = "WEBAPI";
const PARAM_DB_URL_NAME: &str = "PROJECT_DB_DNS";
const PARAM_SERVER_ADDR: &str = "MIKASA_SERVER_ADDR";
const PARAM_SERVER_ADDR_DEFAULT: &str = "127.0.0.1:8080";

use crate::modules::client::client_models::*;
use sqlx::{Pool, MySqlPool, MySql};
use crate::modules::api_lib::api_response::ContextGuard;

pub struct AppContext {
    db: Pool<MySql>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let url = match env::var(PARAM_DB_URL_NAME) {
        Ok(db_url) => db_url,
        Err(e) => panic!("Can't get env var {}. Error: {}", PARAM_DB_URL_NAME, e),
    };
    let server_addr = match env::var(PARAM_SERVER_ADDR) {
        Ok(db_a) => db_a,
        _ => String::from(PARAM_SERVER_ADDR_DEFAULT),
    };

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(url.as_str()).await.expect("Cant connect to db");

    let context = web::Data::new( AppContext {
        db: pool.clone(),
    });

    let mut i: usize = 12;


    HttpServer::new(move || {
        App::new()
            .app_data(context.clone())
            .configure(clients::config)
            .configure(auth::config)
           //.wrap to register middleware - auth, headers modify, etc
    })
        .bind(server_addr)?
        .workers(1)
        .run()
        .await
}
