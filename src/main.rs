#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use dotenv::dotenv;
use neo4rs::*;
use rocket::State;
use tokio::runtime::Runtime;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
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
        .mount("/recipes", routes![index])
        .mount("/", routes![ask_db])
        .manage(rt)
        .manage(graph)
        .launch();
}
