use crate::models::{Recipe, RecipeVec, UserId, Ingredient};
use neo4rs::*;
use rand::prelude::*;
use rocket::response::Redirect;
use rocket::State;
use rocket_contrib::json::Json;
use tokio::runtime::Runtime;
use uuid::Uuid;
use crate::helpers::process_steps;


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
#[post("/new", format = "application/json", data = "<recipe_form>")]
pub fn new_recipe(
    recipe_form: Json<Recipe>,
    graph: State<Graph>,
    rt: State<Runtime>,
    u_id: UserId,
) -> Redirect {
    // Needed to turn any Options returning None into empty strings
    let empty_string = String::new();
    let recipe_uuid = Uuid::new_v4().to_string();
    let recipe_name = &recipe_form.name;
    let recipe_public = recipe_form.public.as_ref().unwrap_or(&false);
    let recipe_tipo = recipe_form.tipo.as_ref().unwrap_or(&empty_string);
    let recipe_calories = recipe_form.calories.as_ref().unwrap_or(&0u16);
    let recipe_carbs = recipe_form.carbohydrates.as_ref().unwrap_or(&0f32);
    let recipe_fat = recipe_form.fat.as_ref().unwrap_or(&0f32);
    let recipe_protein = recipe_form.protein.as_ref().unwrap_or(&0f32);
    let recipe_servings = recipe_form.servings.as_ref().unwrap_or(&empty_string);
    let recipe_meal_type = recipe_form.meal_type.as_ref().unwrap_or(&empty_string);
    let mut steps_string = String::new();

    if recipe_form.steps.is_some() {
        for (i, step) in recipe_form.steps.as_ref().unwrap().iter().enumerate() {
            steps_string.push_str(format!("{}. {}\n", i + 1, step).as_str())
        }
    }

    // Using this instead of neo4rs query().param() pattern since type conversions for bolt
    // protocol are funky right now. It only converts Strings properly, anything else throws the
    // trait Into not implemented for this type.
    // Any strings need to be in quotations. other types and names of params can be w/o quotes
    let param_string = format!(
        "id: \"{id}\", name: \"{name}\", public: {public}, tipo: \"{tipo}\", steps:
        \"{steps}\", calories: {calories}, carbohydrates: {carbs}, fat: {fat}, \
        protein: {protein}, servings: \"{servings}\", meal_type: \"{meal_type}\"",
        id = recipe_uuid,
        name = recipe_name,
        public = recipe_public,
        tipo = &recipe_tipo,
        steps = steps_string,
        calories = recipe_calories,
        carbs = recipe_carbs,
        fat = recipe_fat,
        protein = recipe_protein,
        servings = recipe_servings,
        meal_type = recipe_meal_type,
    );

    // Using this runtime since rocket runs synchronously right now. That will change with rocket
    // 0.5 but until then we need this specific tokio runtime to run any async tasks using the
    // neo4rs driver. Hope we can change driver when a better one comes.
    rt.block_on(async {
        graph
            .run(
                // Triple {{{}}} because {{}} turns into "{}"
                query(
                    format!(
                        "MATCH (u:User) WHERE u.id = $uid \
                    MERGE (u)-[:OWNS]->(:Recipe {{{}}})",
                        param_string
                    )
                    .as_str(),
                )
                .param("uid", u_id.0),
            )
            .await
            .expect("Couldn't add the recipe");

        if recipe_form.ingredients.is_some() {
            let ingredients_vec = recipe_form.ingredients.as_ref().unwrap();
            for ingredient in ingredients_vec {
                let ingredient_tipo = ingredient.tipo.as_ref().unwrap_or(&empty_string);

                graph
                    .run(
                        query("MERGE (:Ingredient {name: $name, tipo: $tipo})")
                            .param("name", ingredient.name.to_lowercase().clone())
                            .param("tipo", ingredient_tipo.to_lowercase().clone()),
                    )
                    .await
                    .expect("Couldn't add the ingredients");

                graph
                    .run(
                        query(
                            "MATCH (i:Ingredient {name: $name}), (r:Recipe {id: $id}) \
                        CREATE (r)-[:USES {amount: $amount}]->(i)",
                        )
                        .param("name", ingredient.name.to_lowercase().clone())
                        .param("id", recipe_uuid.clone())
                        .param("amount", ingredient.amount.clone()),
                    )
                    .await
                    .expect("Couldn't create the relationship")
            }
        }
    });
    println!("{:?}", &recipe_form);
    Redirect::to(uri!(ask_db))
}

#[get("/weekly")]
pub fn random_recipes(rt: State<Runtime>, graph: State<Graph>, usr: UserId) -> Json<RecipeVec> {
    let mut rng = &mut rand::thread_rng();
    let recipes_vector = rt.block_on(async {
        let mut result = graph
            .execute(
                query("MATCH (r:Recipe)-[:OWNS|:LIKED]-(u:User) WHERE u.id = $id RETURN r")
                    .param("id", usr.0),
            )
            .await
            .expect("Error fetching recipes");

        let mut nodes_vector = Vec::new();

        while let Ok(Some(row)) = result.next().await {
            let node = row.get::<Node>("r").expect("Empty row");
            let id = node.get::<String>("id").expect("No id found for node");
            let name = node.get::<String>("name").unwrap_or("No name found for node".to_string());
            let steps = node.get::<String>("steps").unwrap_or("".to_string());
            let public = node.get::<bool>("public");
            let tipo = node.get::<String>("tipo");
            let calories = node.get("calories").unwrap_or(0);
            let carbohydrates = node.get("carbohydrates").unwrap_or(0.0);
            let fat = node.get("fat").unwrap_or(0.0);
            let protein = node.get("protein").unwrap_or(0.0);
            let servings = node.get::<String>("servings");
            let meal_type = node.get::<String>("meal_type");

            let steps = process_steps(steps);

            let recipe = Recipe {
                id: Option::from(Uuid::parse_str(id.as_str()).expect("Couldn't parse uuid")),
                name,
                public,
                steps,
                tipo,
                calories: Option::from(calories as u16),
                carbohydrates: Option::from(carbohydrates as f32),
                fat: Option::from(fat as f32),
                protein: Option::from(protein as f32),
                servings,
                meal_type,
                ingredients: None,
            };

            nodes_vector.push(recipe)
        }
        for mut recipe in &mut nodes_vector {
            let mut response = graph.execute(
                query("MATCH (r:Recipe)-[u:USES]->(i:Ingredient) WHERE r.id = $rid RETURN i, u")
                    .param("rid", recipe.id.unwrap().to_string())
            )
            .await
            .expect("Coulnd't query the ingredients");

            let mut ingredients_vector = Vec::new();

            while let Ok(Some(row)) = response.next().await {
                let node = row.get::<Node>("i").expect("Empty ingredient node");
                let name = node.get::<String>("name").expect("No ingredient name");
                let tipo = node.get::<String>("tipo").unwrap_or("".to_string());
                let relation = row.get::<Relation>("u").expect("No relation");
                let amount = relation.get::<String>("amount").expect("No amount");

                let ingredient = Ingredient{
                    name,
                    tipo: Option::from(tipo),
                    amount
                };

                ingredients_vector.push(ingredient)

            }
            recipe.ingredients = Option::from(ingredients_vector)
        }
        nodes_vector
    });
    let shuffled_recipes = recipes_vector
        .choose_multiple(&mut rng, 5)
        .cloned()
        .collect();

    Json(RecipeVec {
        recipes: shuffled_recipes,
    })
}
