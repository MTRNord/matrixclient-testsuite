#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::State;
use ruma::{
    api::client::{
        error::{Error, ErrorKind},
        r0::{
            session::login,
            sync::sync_events::{self, AccountData, DeviceLists, Presence, Rooms, ToDevice},
            to_device::send_event_to_device,
        },
    },
    identifiers::UserId,
};

use crate::data::Data;
use crate::ruma_wrapper::{MatrixResult, Ruma};
use crate::tests::{Tests, TestsResult};
use std::collections::BTreeMap;
use std::convert::TryFrom;

mod data;
mod error;
mod ruma_wrapper;
mod tests;
mod utils;

#[post("/set_test", data = "<next_test>")]
pub fn set_next_test(data: State<Data>, next_test: Tests) -> TestsResult {
    data.set_current_test(next_test);
    TestsResult(Ok(next_test))
}

#[get("/_matrix/client/r0/sync", data = "<body>")]
pub fn sync_route(
    data: State<Data>,
    body: Ruma<sync_events::Request>,
) -> MatrixResult<sync_events::Response> {
    let user_id = body.user_id.as_ref().expect("user is authenticated");
    let device_id = body.device_id.as_ref().expect("user is authenticated");
    match *(data.current_test.lock().unwrap()) {
        Tests::Start => MatrixResult(Ok(sync_events::Response {
            next_batch: "1".to_string(),
            rooms: Rooms {
                leave: BTreeMap::new(),
                join: BTreeMap::new(),
                invite: BTreeMap::new(),
            },
            presence: Presence { events: vec![] },
            account_data: AccountData { events: vec![] },
            to_device: ToDevice { events: vec![] },
            device_lists: DeviceLists {
                changed: vec![],
                left: vec![],
            },
            device_one_time_keys_count: BTreeMap::new(),
        })),
        Tests::SyncTimeout => MatrixResult(Err(Error {
            kind: ErrorKind::Unknown,
            message: "Timeout".to_string(),
            status_code: http::StatusCode::GATEWAY_TIMEOUT,
        })),
        _ => MatrixResult(Err(Error {
            kind: ErrorKind::Unknown,
            message: "Not Implemented".to_string(),
            status_code: http::StatusCode::NOT_FOUND,
        })),
    }
}

#[post("/_matrix/client/r0/login", data = "<body>")]
pub fn login_route(db: State<Data>, body: Ruma<login::Request>) -> MatrixResult<login::Response> {
    // TODO testing
    MatrixResult(Ok(login::Response {
        user_id: UserId::try_from("@carl:example.com").unwrap(),
        access_token: "123456".to_string(),
        home_server: None,
        device_id: "KCZFUCGSLZ".to_string(),
        well_known: None,
    }))
}

#[options("/<_segments..>")]
pub fn options_route(
    _segments: rocket::http::uri::Segments<'_>,
) -> MatrixResult<send_event_to_device::Response> {
    MatrixResult(Ok(send_event_to_device::Response))
}

fn main() -> anyhow::Result<()> {
    rocket::ignite()
        .mount(
            "/",
            routes![sync_route, set_next_test, login_route, options_route,],
        )
        .manage(Data::default())
        .launch()?;
    Ok(())
}
