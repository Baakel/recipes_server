use neo4rs::*;
// use uuid::Uuid;
use crate::models::GraphPool;
use rocket::http::{Cookie, Cookies, SameSite};

pub async fn get_user_from_db(graph: GraphPool, u_id: &str) -> Option<Node> {
    let mut res = graph
        .execute(query("MATCH (u:User) WHERE u.id = $id RETURN u").param("id", u_id))
        .await
        .expect("Couldn't find that Uuid");

    let row = res.next().await.expect("Couldn't fetch row");
    row.as_ref()?.get::<Node>("u")
}

pub fn set_user_cookies(mut cookies: Cookies, id: String, username: String) {
    let cookie = Cookie::build("user_id", id)
        .path("/")
        .same_site(SameSite::None)
        .secure(true)
        .finish();
    cookies.add_private(cookie);
    // Note that we are specifically changing the path of this public cookie to "/" so that
    // svelte is able to see it. I don't know if this a svelte problem or a me problem. But
    // if we leave it blank rocket will set the path to "/api" and then svelte won't see it
    let exp_date = cookies.get_private("user_id").unwrap().expires().unwrap();
    // .copy();
    let u_cookie = Cookie::build("username", username)
        .path("/")
        .same_site(SameSite::None)
        .expires(exp_date)
        .secure(true)
        .http_only(true)
        .finish();
    cookies.add(u_cookie);
}
