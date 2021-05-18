// use rocket::*;
use crate::models::{GraphPool, LoginCredentials, User, UserId};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use neo4rs::*;
use rand_core::OsRng;
use rocket::http::{Cookie, Cookies, SameSite, Status};
use rocket::request::{FlashMessage, Form};
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_contrib::json::Json;
use tokio::runtime::Runtime;
use uuid::Uuid;
use validator::{validate_email, validate_length};

// TODO: check if you can return a redirect with a status code
#[post("/new", format = "application/json", data = "<user>")]
pub fn new_user(user: Json<User>, graph: State<GraphPool>, rt: State<Runtime>) -> Flash<Redirect> {
    let id = Uuid::new_v4().to_string();
    let empty_string = String::new();
    let username = &user.username;
    let email = user.email.as_ref().unwrap_or(&empty_string);

    if !(validate_email(email))
        || !(validate_length(username, Some(3), None, None))
        || !(validate_length(&user.password, Some(8), None, None))
    {
        return Flash::error(Redirect::to(uri!("/users", query_users)), "Bad Request");
    }

    // Hashing the password
    let password = &user.password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password_simple(password, salt.as_ref())
        .expect("Couldn't hash the password")
        .to_string();
    // Making sure our hash worked with the given password.
    let parsed_hash = PasswordHash::new(&password_hash).expect("Couldn't parse the hash");
    assert!(argon2.verify_password(password, &parsed_hash).is_ok());

    rt.block_on(async {
        graph.run(
            query("CREATE (:User {username: $uname, id: $uid, password: $pass, email: $mail, role:\
            \"pentacoob\"})")
                .param("uname", username.clone())
                .param("uid", id.clone())
                .param("pass", password_hash.clone())
                .param("mail", email.clone())
        ).await.expect("Couldn't add the User")
    });

    Flash::success(
        Redirect::to(uri!("/users", query_users)),
        "User added to the db.",
    )
}

#[post("/login", format = "application/json", data = "<user>")]
pub fn login(
    user: Json<User>,
    graph: State<GraphPool>,
    rt: State<Runtime>,
    mut cookies: Cookies,
) -> Status {
    let username = &user.username;
    let password = &user.password.as_bytes();
    let argon2 = Argon2::default();

    let res_tuple: (String, String, String) = rt.block_on(async {
        let mut result = graph
            .execute(
                query("MATCH (u:User) WHERE u.username = $name RETURN u")
                    .param("name", username.clone()),
            )
            .await
            .expect("Couldn't find that user");

        let row = result
            .next()
            .await
            .expect("Couldn't fetch row")
            .expect("Empty row");
        let node: Node = row.get("u").unwrap();
        (
            node.get("password").expect("No password found"),
            node.get("id").expect("No id found"),
            node.get("username").expect("No Username found"),
        )
    });
    let password_hash = res_tuple.0;
    let id = res_tuple.1;
    let username = res_tuple.2;

    let parsed_hash = PasswordHash::new(&password_hash).expect("Couldn't parse the hash");
    if argon2.verify_password(password, &parsed_hash).is_ok() {

        // Notice that apps like Insomnia won't use cookies if the secure property is set.
        let cookie = Cookie::build("user_id", id)
            .path("/")
            .same_site(SameSite::None)
            .secure(true)
            .finish();
        cookies.add_private(cookie);
        // Note that we are specifically changing the path of this public cookie to "/" so that
        // svelte is able to see it. I don't know if this a svelte problem or a me problem. But
        // if we leave it blank rocket will set the path to "/api" and then svelte won't see it
        let u_cookie = Cookie::build("username", username)
            .path("/")
            .same_site(SameSite::None)
            .secure(true)
            .http_only(true)
            .finish();
        cookies.add(u_cookie);
        // return Flash::success(Redirect::to(uri!("/users", query_users)), "Successfully logged in")
        return Status::Ok;
    }
    // Flash::error(Redirect::to(uri!("/users", query_users)), "Wrong credentials")
    Status::Unauthorized
}

#[post("/login", data = "<user>", rank = 2)]
pub fn login_form(
    user: Form<LoginCredentials>,
    graph: State<GraphPool>,
    rt: State<Runtime>,
    cookies: Cookies,
) -> Status {
    let new_user = User {
        id: None,
        username: user.username.to_owned(),
        password: user.password.to_owned(),
        email: None,
        role: None,
    };
    let user = Json(new_user);
    if login(user, graph, rt, cookies) == Status::Ok {
        return Status::Ok;
    }
    Status::Unauthorized
}

#[get("/")]
pub fn query_users(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    flash: Option<FlashMessage>,
) -> String {
    let res = rt.block_on(async {
        let mut result = graph
            .execute(query("MATCH (u:User) RETURN u"))
            .await
            .unwrap();

        let mut res = Vec::new();

        while let Ok(Some(row)) = result.next().await {
            let node: Node = row.get("u").unwrap();
            let id = node.id();
            let labels = node.labels();
            let name: String = node.get("username").unwrap();
            let role: String = node.get("role").unwrap();
            let pass: String = node.get("password").unwrap();
            res.push(format!(
                "Got id: {}, labels: {:?}, username: {}, role: {}, pass: {}",
                id, labels, name, role, pass
            ))
        }
        res
    });
    format!(
        "Flash was {}\n This is the user vector {:?}",
        flash.unwrap().msg(),
        res
    )
}

#[get("/<name>")]
pub fn get_user(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    name: String,
    key: UserId,
    usr: User,
) -> Option<String> {
    // let key = "nothing";
    println!(
        "Authorized with key {:?}, also we got this User {:?}",
        key, usr
    );
    let node: Option<Node> = rt.block_on(async {
        let mut result = graph
            .execute(query("MATCH (u:User) WHERE u.username = $name RETURN u").param("name", name))
            .await
            .expect("Couldn't find that user");

        let row = result
            .next()
            .await
            .expect("Couldn't fetch row")
            .expect("Empty row");
        row.get("u")
    });
    Some(format!("{:?}", node))
}

// #[get("/<name>", rank = 2)]
// pub fn get_user_redirect(name: String) -> Redirect {
//     Redirect::to(uri!(login))
// }

#[get("/logout")]
pub fn logout(mut cookies: Cookies) -> Status {

    // Adding this path is needed for svelte to actually modify the cookie since cookies with a
    // mismatch in paths will be ignored. And we specifically change the path on the login route
    // to "/" for svelte to read it properly
    let mut username_cookie = Cookie::named("username");
    username_cookie.set_path("/");
    cookies.remove(username_cookie);
    cookies.remove_private(Cookie::named("user_id"));
    Status::NoContent
}
