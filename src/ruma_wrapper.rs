/// From https://git.koesters.xyz/timo/conduit/src/commit/88d091fca1c1db9f238c7a42467f63a619201f8c/src/ruma_wrapper.rs
use crate::utils;
use log::warn;
use rocket::{
    data::{Data, FromData, FromDataFuture, Transform, TransformFuture, Transformed},
    http::Status,
    response::{self, Responder},
    Outcome::*,
    Request, State,
};
use ruma::{api::Endpoint, identifiers::UserId};
use std::{convert::TryInto, io::Cursor, ops::Deref};
use tokio::io::AsyncReadExt;

const MESSAGE_LIMIT: u64 = 20 * 1024 * 1024; // 20 MB

/// This struct converts rocket requests into ruma structs by converting them into http requests
/// first.
pub struct Ruma<T> {
    pub body: T,
    pub user_id: Option<UserId>,
    pub device_id: Option<String>,
    pub json_body: Option<Box<serde_json::value::RawValue>>, // This is None when body is not a valid string
}

impl<'a, T: Endpoint> FromData<'a> for Ruma<T> {
    type Error = (); // TODO: Better error handling
    type Owned = Data;
    type Borrowed = Self::Owned;

    fn transform<'r>(
        _req: &'r Request<'_>,
        data: Data,
    ) -> TransformFuture<'r, Self::Owned, Self::Error> {
        Box::pin(async move { Transform::Owned(Success(data)) })
    }

    fn from_data(
        request: &'a Request<'_>,
        outcome: Transformed<'a, Self>,
    ) -> FromDataFuture<'a, Self, Self::Error> {
        Box::pin(async move {
            let data = rocket::try_outcome!(outcome.owned());

            let (user_id, device_id) = if T::METADATA.requires_authentication {
                let db = request
                    .guard::<State<'_, crate::data::Data>>()
                    .await
                    .unwrap();

                // Get token from header or query value
                let token = match request
                    .headers()
                    .get_one("Authorization")
                    .map(|s| s[7..].to_owned()) // Split off "Bearer "
                    .or_else(|| request.get_query_value("access_token").and_then(|r| r.ok()))
                {
                    // TODO: M_MISSING_TOKEN
                    None => return Failure((Status::Unauthorized, ())),
                    Some(token) => token,
                };

                // Check if token is valid
                match db.users.find_from_token(&token).unwrap() {
                    // TODO: M_UNKNOWN_TOKEN
                    None => return Failure((Status::Unauthorized, ())),
                    Some((user_id, device_id)) => (Some(user_id), Some(device_id)),
                }
            } else {
                (None, None)
            };

            let mut http_request = http::Request::builder()
                .uri(request.uri().to_string())
                .method(&*request.method().to_string());
            for header in request.headers().iter() {
                http_request = http_request.header(header.name.as_str(), &*header.value);
            }

            let mut handle = data.open().take(MESSAGE_LIMIT);
            let mut body = Vec::new();
            handle.read_to_end(&mut body).await.unwrap();

            let http_request = http_request.body(body.clone()).unwrap();
            log::info!("{:?}", http_request);

            match T::try_from(http_request) {
                Ok(t) => Success(Ruma {
                    body: t,
                    user_id,
                    device_id,
                    // TODO: Can we avoid parsing it again? (We only need this for append_pdu)
                    json_body: utils::string_from_bytes(&body)
                        .ok()
                        .and_then(|s| serde_json::value::RawValue::from_string(s).ok()),
                }),
                Err(e) => {
                    warn!("{:?}", e);
                    Failure((Status::BadRequest, ()))
                }
            }
        })
    }
}

impl<T> Deref for Ruma<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// This struct converts ruma responses into rocket http responses.
pub struct MatrixResult<T, E = ruma::api::client::Error>(pub std::result::Result<T, E>);

impl<T, E> TryInto<http::Response<Vec<u8>>> for MatrixResult<T, E>
where
    T: TryInto<http::Response<Vec<u8>>>,
    E: Into<http::Response<Vec<u8>>>,
{
    type Error = T::Error;

    fn try_into(self) -> Result<http::Response<Vec<u8>>, T::Error> {
        match self.0 {
            Ok(t) => t.try_into(),
            Err(e) => Ok(e.into()),
        }
    }
}

#[rocket::async_trait]
impl<'r, T, E> Responder<'r> for MatrixResult<T, E>
where
    T: Send + TryInto<http::Response<Vec<u8>>>,
    T::Error: Send,
    E: Into<http::Response<Vec<u8>>> + Send,
{
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        let http_response: Result<http::Response<_>, _> = self.try_into();
        match http_response {
            Ok(http_response) => {
                let mut response = rocket::response::Response::build();

                let status = http_response.status();
                response.raw_status(status.into(), "");

                for header in http_response.headers() {
                    response
                        .raw_header(header.0.to_string(), header.1.to_str().unwrap().to_owned());
                }

                response
                    .sized_body(Cursor::new(http_response.into_body()))
                    .await;

                response.raw_header("Access-Control-Allow-Origin", "*");
                response.raw_header(
                    "Access-Control-Allow-Methods",
                    "GET, POST, PUT, DELETE, OPTIONS",
                );
                response.raw_header(
                    "Access-Control-Allow-Headers",
                    "Origin, X-Requested-With, Content-Type, Accept, Authorization",
                );
                response.ok()
            }
            Err(_) => Err(Status::InternalServerError),
        }
    }
}

impl<T, E> Deref for MatrixResult<T, E> {
    type Target = Result<T, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
