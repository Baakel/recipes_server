use neo4rs::*;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
pub struct RecipeVec {
    pub recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all="camelCase")]
pub struct Recipe {
    pub id: Option<Uuid>,
    pub name: String,
    pub public: Option<bool>,
    pub steps: Option<Vec<String>>,
    pub tipo: Option<String>,
    pub calories: Option<u16>,
    pub carbohydrates: Option<f32>,
    pub fat: Option<f32>,
    pub protein: Option<f32>,
    pub servings: Option<String>,
    pub meal_type: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct User {
    pub id: Option<Uuid>,
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(length(min = 10))]
    pub password: String,
    #[validate(email)]
    pub email: Option<String>,
    pub role: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct Ingredient {
    pub name: String,
    pub tipo: Option<String>,
    pub amount: String,
}

// Used as a sort of JWT just verifying that the user has a valid id in the db.
#[derive(Debug)]
pub struct UserId(pub String);

// Used to return the error in outcome
#[derive(Debug)]
pub enum UsedIdError {
    Missing,
    Invalid,
}

// Request guards for authentication. If they fail the page won't be visible
// Similar to the Flask @login_required decorators.
impl<'a, 'r> FromRequest<'a, 'r> for UserId {
    type Error = UsedIdError;

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let rt = request
            .guard::<State<Runtime>>()
            .expect("Couldn't get the rt guard");
        let graph = request
            .guard::<State<Graph>>()
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
            rt.block_on(async {
                let mut res = graph
                    .execute(
                        query("MATCH (u:User) WHERE u.id = $id RETURN u")
                            .param("id", cookie_id.clone()),
                    )
                    .await
                    .expect("Couldn't find that Uuid");

                let row = res.next().await.expect("Couldn't fetch row");
                row.as_ref()?.get::<Node>("u")
            })
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
        let graph = request.guard::<State<Graph>>()?;
        let uuid_guard = request.guard::<UserId>();
        if uuid_guard.is_failure() {
            return Outcome::Failure((Status::Unauthorized, ()));
        }
        let uuid = uuid_guard.unwrap().0;
        let result = request.local_cache(|| {
            rt.block_on(async {
                let mut res = graph
                    .execute(
                        query("MATCH (u:User) WHERE u.id = $id RETURN u").param("id", uuid.clone()),
                    )
                    .await
                    .expect("Couldn't find that Uuid");

                let row = res.next().await.expect("Couldn't fetch row");
                row.as_ref()?.get::<Node>("u")
            })
        });
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
