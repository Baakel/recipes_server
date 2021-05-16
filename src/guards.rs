use crate::helpers::users::get_user_from_db;
use crate::models::{ChosenDeleted, ChosenTimeError, GraphPool, UsedIdError, User, UserId};
use chrono::{Duration, NaiveDateTime, Utc};
use neo4rs::*;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};
use tokio::runtime::Runtime;
use uuid::Uuid;

// Request guards for authentication. If they fail the page won't be visible
// Similar to the Flask @login_required decorators.
impl<'a, 'r> FromRequest<'a, 'r> for UserId {
    type Error = UsedIdError;

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let rt = request
            .guard::<State<Runtime>>()
            .expect("Couldn't get the rt guard");
        let graph = request
            .guard::<State<GraphPool>>()
            .expect("Couldn't get the graph guard");
        let cookie_id_option: Option<String> = request
            .cookies()
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse().ok());
        if cookie_id_option.is_none() {
            return Outcome::Failure((Status::Unauthorized, UsedIdError::Missing));
        }
        let cookie_id = cookie_id_option.unwrap();
        let result = request.local_cache(|| {
            rt.block_on(async { get_user_from_db(graph.clone(), &cookie_id).await })
        });
        if result.is_none() {
            return Outcome::Failure((Status::NotFound, UsedIdError::Invalid));
        }
        Outcome::Success(UserId(cookie_id))
    }
}

// We are probably never using this trait. You get the user back without the password but we
// don't really need it. With our current implementation we usually query the db every time
// anyways. A User type doesn't really help us unless we wanted to return it as a JSON format for
// some specific task in the frontend.
impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, ()> {
        let rt = request.guard::<State<Runtime>>()?;
        let graph = request.guard::<State<GraphPool>>()?;
        let uuid_guard = request.guard::<UserId>();
        if uuid_guard.is_failure() {
            return Outcome::Failure((Status::Unauthorized, ()));
        }
        let uuid = uuid_guard.unwrap().0;
        let result = request
            .local_cache(|| rt.block_on(async { get_user_from_db(graph.clone(), &uuid).await }));
        if result.is_none() {
            return Outcome::Failure((Status::NotFound, ()));
        }
        let id_string: Option<String> = result.as_ref().unwrap().get("id");
        let uuid = Uuid::parse_str(id_string.unwrap().as_str()).expect("Couldn't parse string");
        let name = result.as_ref().unwrap().get("username").unwrap();
        let email = result.as_ref().unwrap().get("email").unwrap();
        let password = "";
        let role = result.as_ref().unwrap().get("role").unwrap();
        Outcome::Success(User {
            id: Some(uuid),
            username: name,
            email: Some(email),
            password: password.to_string(),
            role: Some(role),
        })
    }
}

// Use this guard to delete the recipes after 8 days when checking thºe dashboard. This
// guard will always succeed, the only difference will be that the recipes get either deleted or
// not. If they do get deleted then send a boolean true, if not then send a bool false.
impl<'a, 'r> FromRequest<'a, 'r> for ChosenDeleted {
    type Error = ChosenTimeError;

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let rt = request
            .guard::<State<Runtime>>()
            .expect("Couldn't get the rt guard");
        let graph = request
            .guard::<State<GraphPool>>()
            .expect("Couldn't get the graph guard");
        let uuid_guard = request.guard::<UserId>();
        if uuid_guard.is_failure() {
            return Outcome::Failure((Status::Unauthorized, ChosenTimeError::Missing));
        }
        let uuid = uuid_guard.unwrap().0;
        let date_created = request.local_cache(|| {
            rt.block_on(async {
                let mut res = graph
                    .execute(
                        query("MATCH (u:User)-[c:CHOSEN]-() WHERE u.id = $id RETURN c")
                            .param("id", uuid.clone()),
                    )
                    .await
                    .expect("Couldn't find that Uuid");

                let row = res
                    .next()
                    .await
                    .expect("Couldn't fetch row")
                    .expect("Empty row");
                let relationship_node = row.get::<Relation>("c").unwrap();
                let date_created: NaiveDateTime = relationship_node.get("created").unwrap();

                date_created
            })
        });
        let eight_days_offset = date_created
            .checked_add_signed(Duration::seconds(8 * 24 * 60 * 60))
            .unwrap();
        if Utc::now().naive_utc() > eight_days_offset {
            rt.block_on(async {
                graph
                    .run(
                        query("MATCH (u:User)-[c:CHOSEN]-() WHERE u.id=$id DETACH DELETE c")
                            .param("id", uuid.clone()),
                    )
                    .await
                    .expect("Couldn't run query")
            });
            return Outcome::Success(ChosenDeleted(true));
        }
        Outcome::Success(ChosenDeleted(false))
    }
}
