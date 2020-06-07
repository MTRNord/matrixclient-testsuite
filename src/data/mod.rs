use crate::error::Result;
use crate::tests::Tests;
use ruma::identifiers::UserId;
use std::convert::TryFrom;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct Data {
    pub users: Users,
    pub current_test: Mutex<Tests>,
}

impl Data {
    pub fn set_current_test(&self, next_test: Tests) {
        let mut state = self.current_test.lock().expect("Could not lock mutex");
        *state = next_test;
    }
}

#[derive(Debug, Default)]
pub struct Users;

impl Users {
    pub fn find_from_token(&self, _token: &str) -> Result<Option<(UserId, String)>> {
        // TODO run tests
        Ok(Some((
            UserId::try_from("@carl:example.com").unwrap(),
            "KCZFUCGSLZ".to_string(),
        )))
    }
}
