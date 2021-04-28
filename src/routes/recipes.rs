use tokio::runtime::Runtime;
use rocket::State;
use rocket::response::Redirect;
use rocket_contrib::json::Json;
use neo4rs::*;
use crate::models::Recipe;
use uuid::Uuid;


#[get("/query")]
pub fn ask_db(rt: State<Runtime>, graph: State<Graph>) -> String {
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

// TODO: add the (:User)-[:OWNS]->(:Recipe) relationship
#[post("/new", format="application/json", data="<recipe_form>")]
pub fn new_recipe(recipe_form: Json<Recipe>, graph: State<Graph>, rt: State<Runtime>) -> Redirect {
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