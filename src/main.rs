#![feature(proc_macro_hygiene, decl_macro)]

mod models;
mod routes;

#[macro_use]
extern crate rocket;
use dotenv::dotenv;
use neo4rs::*;
// use rocket::State;
use tokio::runtime::Runtime;
// use crate::models::{Recipe};
// use crate::users::{new_user, login, query_users};
// use rocket::response::{Redirect};
// use rocket_contrib::json::Json;
// use uuid::Uuid;
// use rocket::request::FlashMessage;
// use rocket::http::Status;


const USER_MOUNT: &str = "/users";
const ROOT_MOUNT: &str = "/";
const RECIPES_MOUNT: &str = "/recipes";

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

async fn create_graph(uri: String, user: String, pass: String) -> Graph {
    Graph::new(uri.as_str(), user.as_str(), pass.as_str())
        .await
        .expect("Couldn't connect")
}

fn main() {
    dotenv().ok();
    let uri = std::env::var("DB_URI").expect("set DB_URI");
    let user = std::env::var("DB_USER").expect("set DB_USER");
    let pass = std::env::var("DB_PASS").expect("set DB_PASS");
    let rt = Runtime::new().expect("Unable to create rt");
    let graph = rt.block_on(create_graph(uri, user, pass));
    rocket::ignite()
        .mount(RECIPES_MOUNT, routes![index, routes::recipes::new_recipe])
        .mount(USER_MOUNT,
               routes![
               routes::users::new_user,
               routes::users::query_users,
               routes::users::get_user,
               ])
        .mount(ROOT_MOUNT, routes![routes::recipes::ask_db, routes::users::login])
        .manage(rt)
        .manage(graph)
        .launch();
}
