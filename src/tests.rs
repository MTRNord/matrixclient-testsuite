use crate::error::TestErrors;
use rocket::data::{FromData, FromDataFuture, Transform, TransformFuture, Transformed};
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::response::{self, Response};
use rocket::{Data, Outcome::*, Request};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tokio::io::AsyncReadExt;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[serde(tag = "test")]
#[serde(rename_all = "snake_case")]
pub enum Tests {
    Start,
    SyncTimeout,
}

impl Default for Tests {
    fn default() -> Self {
        Tests::Start
    }
}

impl<'a> FromData<'a> for Tests {
    type Error = TestErrors;
    type Owned = String;
    type Borrowed = str;

    fn transform<'r>(
        request: &'r Request<'_>,
        data: Data,
    ) -> TransformFuture<'r, Self::Owned, Self::Error> {
        Box::pin(async move {
            let mut stream = data.open();
            let mut string = String::new();
            let outcome = match stream.read_to_string(&mut string).await {
                Ok(_) => Success(string),
                Err(e) => Failure((Status::InternalServerError, TestErrors::Io(e))),
            };

            // Returning `Borrowed` here means we get `Borrowed` in `from_data`.
            Transform::Borrowed(outcome)
        })
    }

    fn from_data(
        request: &'a Request<'_>,
        outcome: Transformed<'a, Self>,
    ) -> FromDataFuture<'a, Self, Self::Error> {
        Box::pin(async move {
            // Retrieve a borrow to the now transformed `String` (an &str). This
            // is only correct because we know we _always_ return a `Borrowed` from
            // `transform` above.
            let data = try_outcome!(outcome.borrowed());

            let test_enum: Tests = serde_json::from_str(data).unwrap();
            Success(test_enum)
        })
    }
}

/// This struct converts test responses into rocket http responses.
pub struct TestsResult(pub std::result::Result<Tests, TestErrors>);

#[rocket::async_trait]
impl<'r> Responder<'r> for TestsResult {
    async fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        match self.0 {
            Ok(v) => Response::build()
                .header(ContentType::JSON)
                .sized_body(Cursor::new(serde_json::to_string(&v).unwrap()))
                .await
                .ok(),
            Err(v) => Err(Status::InternalServerError),
        }
    }
}
