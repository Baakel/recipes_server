#![feature(proc_macro_hygiene, decl_macro)]

mod models;

#[macro_use]
extern crate rocket;
use dotenv::dotenv;
use neo4rs::*;
use rocket::State;
use tokio::runtime::Runtime;
use crate::models::{Ingredient, Recipe};
use rocket::response::Redirect;
use rocket_contrib::json::Json;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

// TODO: Add the recipe to db and add relationship between ingredients and recipe
#[post("/new", format="application/json", data="<recipe_form>")]
fn new_recipe(recipe_form: Json<Recipe>, graph: State<Graph>, rt: State<Runtime>) -> Redirect {
    rt.block_on(async {
        if let Some(ingredients) = &recipe_form.ingredients {
            for ingredient in ingredients {
                graph.run(
                    query("MERGE (:Ingredient {name: $name, tipo: $tipo})")
                        .param("name", ingredient.name.to_lowercase().clone())
                        .param("tipo", ingredient.tipo.as_ref().unwrap().to_lowercase().clone())
                ).await.expect("Couldn't add the ingredients");
            }
        }
    });
    println!("{:?}", &recipe_form);
    Redirect::to("/query")
}

#[get("/query")]
fn ask_db(rt: State<Runtime>, graph: State<Graph>) -> String {
    // let res = rt.block_on(make_query());
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
        .mount("/", routes![ask_db])
        .manage(rt)
        .manage(graph)
        .launch();
}
