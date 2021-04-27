#![feature(proc_macro_hygiene, decl_macro)]

mod models;

#[macro_use]
extern crate rocket;
use dotenv::dotenv;
use neo4rs::*;
use rocket::State;
use tokio::runtime::Runtime;
use crate::models::{Recipe, User};
use rocket::response::Redirect;
use rocket_contrib::json::Json;
use uuid::Uuid;
use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use rand_core::OsRng;


#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

// TODO: add the (:User)-[:OWNS]->(:Recipe) relationship
#[post("/new", format="application/json", data="<recipe_form>")]
fn new_recipe(recipe_form: Json<Recipe>, graph: State<Graph>, rt: State<Runtime>) -> Redirect {
    // Needed to turn any Options returning None into empty strings
    let empty_string = String::new();
    let recipe_uuid = Uuid::new_v4().to_string();
    let recipe_name = &recipe_form.name;
    let recipe_public = &recipe_form.public;
    let recipe_tipo = recipe_form.tipo.as_ref().unwrap_or(&empty_string);
    let recipe_calories = recipe_form.calories.as_ref().unwrap_or(&0u16);
    let recipe_carbs = recipe_form.carbohydrates.as_ref().unwrap_or(&0f32);
    let recipe_fat = recipe_form.fat.as_ref().unwrap_or(&0f32);
    let recipe_protein = recipe_form.protein.as_ref().unwrap_or(&0f32);
    let recipe_servings = recipe_form.servings.as_ref().unwrap_or(&empty_string);
    let mut steps_string = String::new();

    if recipe_form.steps.is_some() {
        for (i, step) in recipe_form.steps.as_ref().unwrap().iter().enumerate() {
            steps_string.push_str(format!("{}. {}\n\n", i+1, step).as_str())
        }
    }

    // Using this instead of neo4rs query().param() pattern since type conversions for bolt
    // protocol are funky right now. It only converts Strings properly, anything else throws the
    // trait Into not implemented for this type.
    // Any strings need to be in quotations. other types and names of params can be w/o quotes
    let param_string = format!(
        "id: \"{id}\", name: \"{name}\", public: {public}, tipo: \"{tipo}\", steps:
        \"{steps}\", calories: {calories}, carbohydrates: {carbs}, fat: {fat}, \
        protein: {protein}, servings: \"{servings}\"", id=recipe_uuid,
        name=recipe_name, public=recipe_public, tipo=&recipe_tipo, steps=steps_string,
        calories=recipe_calories, carbs=recipe_carbs, fat=recipe_fat, protein=recipe_protein,
        servings=recipe_servings
    );

    // Using this runtime since rocket runs synchronously right now. That will change with rocket
    // 0.5 but until then we need this specific tokio runtime to run any async tasks using the
    // neo4rs driver. Hope we can change driver when a better one comes.
    rt.block_on(async {
        graph.run(
            // Triple {{{}}} because {{}} turns into "{}"
            query(format!("CREATE (:Recipe {{{}}})", param_string).as_str())
        ).await.expect("Couldn't add the recipe");

        if recipe_form.ingredients.is_some() {
            let ingredients_vec = recipe_form.ingredients.as_ref().unwrap();
            for ingredient in ingredients_vec {

                let ingredient_tipo = ingredient.tipo.as_ref().unwrap_or(&empty_string);

                graph.run(
                    query("MERGE (:Ingredient {name: $name, tipo: $tipo})")
                        .param("name", ingredient.name.to_lowercase().clone())
                        .param("tipo", ingredient_tipo.to_lowercase().clone())
                ).await.expect("Couldn't add the ingredients");

                graph.run(
                    query(
                        "MATCH (i:Ingredient {name: $name}), (r:Recipe {id: $id}) \
                        CREATE (r)-[:USES {amount: $amount}]->(i)")
                        .param("name", ingredient.name.to_lowercase().clone())
                        .param("id", recipe_uuid.clone())
                        .param("amount", ingredient.amount.clone())
                ).await.expect("Couldn't create the relationship")
            }
        }
    });
    println!("{:?}", &recipe_form);
    Redirect::to("/query")
}

// TODO: add users to db and create a route to query them.
#[post("/new", format="application/json", data="<user>")]
fn new_user(user: Json<User>, graph: State<Graph>, rt: State<Runtime>) -> Redirect {
    let id = Uuid::new_v4().to_string();
    let username = &user.username;
    let email = &user.email;

    // Hashing the password
    let password = &user.password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password_simple(password, salt.as_ref())
        .expect("Couldn't hash the password").to_string();
    // Making sure our hash worked with the given password.
    let parsed_hash = PasswordHash::new(&password_hash).expect("Couldn't parse the hash");

    assert!(argon2.verify_password(password, &parsed_hash).is_ok());
    Redirect::to("/users")
}

#[get("/query")]
fn ask_db(rt: State<Runtime>, graph: State<Graph>) -> String {
    let res = rt.block_on(async {
        let mut result = graph
            .execute(query("MATCH (n:Recipe) RETURN n"))
            .await
            .unwrap();

        let mut res = Vec::new();

        while let Ok(Some(row)) = result.next().await {
            let node: Node = row.get("n").unwrap();
            let id = node.id();
            let labels = node.labels();
            let name: String = node.get("name").unwrap();
            res.push(format!(
                "Got id: {}, labels: {:?}, name: {}",
                id, labels, name
            ))
        }
        res
    });

    format!("This is the vec we got {:?}", res)
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
        .mount("/recipes", routes![index, new_recipe])
        .mount("/users", routes![new_user])
        .mount("/", routes![ask_db])
        .manage(rt)
        .manage(graph)
        .launch();
}
