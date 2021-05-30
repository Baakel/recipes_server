#![feature(proc_macro_hygiene, decl_macro)]

mod guards;
mod helpers;
mod models;
mod routes;

#[macro_use]
extern crate rocket;
// use rocket_contrib::serve::StaticFiles;
use dotenv::dotenv;
use neo4rs::*;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
// use rocket::State;
use rocket::http::Method;
use std::sync::Arc;
use tokio::runtime::Runtime;
// use std::collections::HashSet;
// use crate::models::{Recipe};
// use crate::users::{new_user, login, query_users};
// use rocket::response::{Redirect};
// use rocket_contrib::json::Json;
// use uuid::Uuid;
// use rocket::request::FlashMessage;
// use rocket::http::Status;

const USER_MOUNT: &str = "/api/users";
const ROOT_MOUNT: &str = "/api";
const RECIPES_MOUNT: &str = "/api/recipes";

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
    let graph = Arc::new(rt.block_on(create_graph(uri, user, pass)));

    // In theory these are needed because the app is working as an API. If i can figure out how
    // to work with the static sites from Svelte I could maybe get rid of this and server
    // everything from here. Lack of documentation is killing me.
    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:3000"]);
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post, Method::Options, Method::Get, Method::Delete]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept", "Content-Type"]),
        allow_credentials: true,
        expose_headers: ["Content-Type", "X-Custom"]
            .iter()
            .map(|val| val.to_string())
            .collect(),
        max_age: None,
        send_wildcard: false,
        fairing_route_base: "/".to_string(),
        fairing_route_rank: 0,
    }
    .to_cors()
    .expect("Cant make cors");

    rocket::ignite()
        .mount(
            RECIPES_MOUNT,
            routes![
                index,
                routes::recipes::new_recipe,
                routes::recipes::random_recipes,
                routes::recipes::choose_recipes,
                routes::recipes::chosen_recipes,
                routes::recipes::recipes_by_ingredient,
                routes::recipes::remove_recipe,
                routes::recipes::get_recipe,
                routes::recipes::reset_all_chosen,
            ],
        )
        .mount(
            USER_MOUNT,
            routes![
                routes::users::new_user,
                routes::users::query_users,
                routes::users::get_user,
                // routes::users::get_user_redirect,
            ],
        )
        .mount(
            ROOT_MOUNT,
            routes![
                routes::recipes::ask_db,
                routes::users::login,
                routes::users::login_form,
                routes::users::logout,
            ],
        )
        // .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
        .manage(rt)
        .manage(graph)
        .attach(cors)
        .launch();
}
