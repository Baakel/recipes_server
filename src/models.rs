use validator::Validate;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};
use tokio::runtime::Runtime;
use neo4rs::*;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Recipe {
    pub id: Option<Uuid>,
    pub name: String,
    pub public: bool,
    pub steps: Option<Vec<String>>,
    pub tipo: Option<String>,
    pub calories: Option<u16>,
    pub carbohydrates: Option<f32>,
    pub fat: Option<f32>,
    pub protein: Option<f32>,
    pub servings: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>
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
    pub role: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Ingredient {
    pub name: String,
    pub tipo: Option<String>,
    pub amount: String,
}

pub struct UserId(String);

// TODO: Try to implement the request.local_cache thing https://api.rocket.rs/v0.4/rocket/request/trait.FromRequest.html 
impl<'a, 'r> FromRequest<'a, 'r> for UserId {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, ()> {
        let rt = request.guard::<State<Runtime>>()?;
        let graph = request.guard::<State<Graph>>()?;
        let cookie_id: String = request.cookies().get_private("user_id")
            .and_then(|cookie| cookie.value().parse().ok()).expect("Couldn't get the cookie value");
        println!("{}", &cookie_id);
        let result: Option<Node> = rt.block_on(async {
            let mut res = graph.execute(
               query("MATCH (u:User) WHERE u.id = $id RETURN u")
                   .param("id", cookie_id.clone())
            ).await.expect("Couldn't find that Uuid");

            let row = res.next().await.expect("Couldn't fetch row");
            if row.is_none() {
                return None
            }
            row.unwrap().get("u")
        });
        if result.is_none() {
            return Outcome::Forward(())
        }
        Outcome::Success(UserId(cookie_id))
    }
}
