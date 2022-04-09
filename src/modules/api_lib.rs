pub mod api_response {
    use actix_web::HttpResponse;
    use serde::{Deserialize,Serialize};

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use actix_service::{Service, Transform};
    use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
    use futures::future::{ok, Ready};
    use futures::{Future, TryFutureExt, StreamExt};
    use crate::AppContext;
    use crate::modules::dbrepo::dbId;
    use crate::modules::auth::auth::StaffUser;

    pub enum ApiResponseStatus {
        Ok,
        Error
    }

    #[derive(Deserialize, Serialize, sqlx::FromRow, Debug)]
    pub struct ApiResponse {
        code: i32,
        status: String,
        response: String,
    }

    impl ApiResponse {
        pub fn new(code: i32, status: ApiResponseStatus, response: String) ->HttpResponse {
            let status_text = match status {
                ApiResponseStatus::Ok => "ok",
                ApiResponseStatus::Error => "error",
            };

            HttpResponse::Ok().content_type("text/json").json(
                ApiResponse {
                    code,
                    status: String::from(status_text),
                    response
                }
            )
        }
    }

    pub struct ApiUser {
        user: StaffUser,
        roles: Vec<ApiRole>,
    }

    pub struct ApiRole {
        id: dbId,
        name: String,
        permissions: Vec<ApiPermission>
    }

    pub struct ApiPermission {
        id: dbId,
        path: String,
    }

    pub struct ContextGuard;

    // Middleware factory is `Transform` trait from actix-service crate
    // `S` - type of the next service
    // `B` - type of response's body
    impl<S, B> Transform<S> for ContextGuard
        where
            S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
            S::Future: 'static,
            B: 'static,
    {
        type Request = ServiceRequest;
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Transform = ContextGuardMiddleware<S>;
        type InitError = ();
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        fn new_transform(&self, service: S) -> Self::Future {
            ok(ContextGuardMiddleware { service })
        }
    }

    pub struct ContextGuardMiddleware<S> {
        service: S,
    }

    impl<S, B> Service for ContextGuardMiddleware<S>
        where
            S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
            S::Future: 'static,
            B: 'static,
    {
        type Request = ServiceRequest;
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.service.poll_ready(cx)
        }

        fn call(&mut self, req: ServiceRequest) -> Self::Future {
            println!("Hi from start. You requested: {}", req.path());
            let app_data = req.app_data::<AppContext>().unwrap();
            let db = app_data.db.clone();

            //req.headers().
            //req.headers().contains_key();

            let fut = self.service.call(req);

            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        }
    }
}
