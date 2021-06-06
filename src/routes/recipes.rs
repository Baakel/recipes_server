use crate::helpers::recipes::{format_recipes, get_ingredients_from_db};
use crate::models::{ChosenDeleted, GraphPool, IdsVec, Recipe, RecipeVec, UserId, RecipeRelationships};
use chrono::prelude::*;
use itertools::Itertools;
use neo4rs::*;
use rand::prelude::*;
use rocket::http::Status;
use rocket::State;
use rocket_contrib::json::Json;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[get("/query")]
pub fn ask_db(rt: State<Runtime>, graph: State<GraphPool>) -> String {
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

#[post("/new", format = "application/json", data = "<recipe_form>")]
pub fn new_recipe(
    recipe_form: Json<Recipe>,
    graph: State<GraphPool>,
    rt: State<Runtime>,
    u_id: UserId,
) -> Status {
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
    let recipe_time = recipe_form.time.as_ref().unwrap_or(&empty_string);
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
        protein: {protein}, servings: \"{servings}\", meal_type: \"{meal_type}\", time: \
        \"{time}\"",
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
        time = recipe_time
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
    Status::Created
}

#[get("/weekly?<amount>")]
pub fn random_recipes(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    usr: UserId,
    amount: Option<usize>,
) -> Json<RecipeVec> {
    let mut rng = &mut rand::thread_rng();
    let recipes_vector = rt.block_on(async {
        let mut result = graph
            .execute(
                query("MATCH (r:Recipe)-[:OWNS|:LIKES]-(u:User) WHERE u.id = $id RETURN r")
                    .param("id", usr.0),
            )
            .await
            .expect("Error fetching recipes");

        let mut nodes_vector = Vec::new();

        while let Ok(Some(row)) = result.next().await {
            nodes_vector.push(format_recipes(row))
        }

        for recipe in &mut nodes_vector {
            get_ingredients_from_db(graph.clone(), recipe).await;
        }
        nodes_vector
    });
    let amount_of_recipes = amount.unwrap_or(7);
    let shuffled_recipes = recipes_vector
        .choose_multiple(&mut rng, amount_of_recipes)
        .cloned()
        .collect();

    Json(RecipeVec {
        recipes: shuffled_recipes,
        rels: None
    })
}

// TODO: Change the return type from string to Outcome.
#[post("/weekly", format = "application/json", data = "<data>")]
pub fn choose_recipes(
    graph: State<GraphPool>,
    rt: State<Runtime>,
    usr: UserId,
    data: Json<IdsVec>,
) -> Status {
    let u_id = usr.0;
    let date = Utc::now().naive_utc();
    rt.block_on(async {
        for recipe_id in &data.ids {
            graph
                .run(
                    query(
                        "MATCH (u:User {id: $id}), (r:Recipe {id: $rid}) \
                    CREATE (u)-[:CHOSEN {created: $exp}]->(r)",
                    )
                    .param("id", u_id.as_str())
                    .param("rid", recipe_id.as_str())
                    .param("exp", date),
                )
                .await
                .expect("Couldn't query graph");
        }
    });
    Status::Created
}

#[get("/chosen")]
pub fn chosen_recipes(
    graph: State<GraphPool>,
    rt: State<Runtime>,
    usr: UserId,
    deleted: ChosenDeleted,
) -> Json<RecipeVec> {
    let u_id = usr.0;
    if deleted.0 {
        return Json(RecipeVec {
            recipes: Vec::new(),
            rels: None
        });
    }
    let recipes_vector = rt.block_on(async {
        let mut recipes = graph
            .execute(
                query("MATCH (u:User)-[:CHOSEN]-(r:Recipe) WHERE u.id = $id RETURN r")
                    .param("id", u_id),
            )
            .await
            .expect("Couldn't query graph");

        let mut recipes_vector = Vec::new();

        while let Ok(Some(row)) = recipes.next().await {
            recipes_vector.push(format_recipes(row))
        }
        for recipe in &mut recipes_vector {
            get_ingredients_from_db(graph.clone(), recipe).await;
        }
        recipes_vector
    });

    Json(RecipeVec {
        recipes: recipes_vector,
        rels: None
    })
}

#[get("/ingredient/<ingredient>")]
pub fn recipes_by_ingredient(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    ingredient: String,
    u_id: UserId,
) -> Json<RecipeVec> {
    let u_id = u_id.0;
    let recipe_vector = rt.block_on(async {
        let mut user_recipes = graph
            .execute(
                query(
                    "MATCH (u:User)-[:OWNS]->(r:Recipe)-[:USES]->(i:Ingredient) \
                WHERE u.id = $id AND i.name = $ing \
                RETURN r",
                )
                .param("id", u_id.clone())
                .param("ing", ingredient.clone()),
            )
            .await
            .expect("Couldn't run query");

        let mut recipes_vec = Vec::new();

        while let Ok(Some(row)) = user_recipes.next().await {
            recipes_vec.push(format_recipes(row))
        }

        let mut public_recipes = graph
            .execute(
                query(
                    "MATCH (r:Recipe)-[:USES]->(i:Ingredient) \
                WHERE r.public = true AND i.name = $ing \
                RETURN r",
                )
                .param("ing", ingredient.clone()),
            )
            .await
            .expect("Couldn't run query for public");

        while let Ok(Some(row)) = public_recipes.next().await {
            recipes_vec.push(format_recipes(row))
        }

        let mut unique_recipes = recipes_vec
            .iter()
            .unique_by(|r| &r.id)
            .cloned()
            .collect::<Vec<_>>();
        for recipe in &mut unique_recipes {
            get_ingredients_from_db(graph.clone(), recipe).await;
        }
        unique_recipes
    });

    Json(RecipeVec {
        recipes: recipe_vector,
        rels: None
    })
}

#[get("/list")]
pub fn recipe_list(graph: State<GraphPool>, rt: State<Runtime>, usr: UserId) -> Json<RecipeVec> {
    let recipes_vec = rt.block_on(async {
        let mut owned_recipes = graph
            .execute(
                query(
                    "MATCH (u:User)-[c:OWNS|LIKES]->(r:Recipe) \
            WHERE u.id = $u_id \
            RETURN r, c",
                )
                .param("u_id", usr.0.clone()),
            )
            .await
            .expect("Error getting the recipes!");

        let mut recipes_vector = Vec::new();
        // let mut rel_vector = Vec::new();
        let mut rel_struct = RecipeRelationships {owns: Vec::new(), likes: Vec::new() };

        while let Ok(Some(row)) = owned_recipes.next().await {
            let relationship_node = row.get::<Relation>("c").unwrap();
            let formatted_recipe = format_recipes(row);
            if &relationship_node.typ() == "OWNS" {
                rel_struct.owns.push(formatted_recipe.id.unwrap().clone())
            }
            if &relationship_node.typ() == "LIKES" {
                rel_struct.likes.push(formatted_recipe.id.unwrap().clone())
            }
            // rel_vector.push((formatted_recipe.id.unwrap().clone(), relationship_node.typ()));
            recipes_vector.push(formatted_recipe);
        }

        // let mut liked_recipes = graph.execute(
        //     query(
        //         "MATCH (u:User)-[l:LIKES]->(r:Recipe) \
        //         WHERE u.id = $u_id"
        //     )
        // )
        // .await
        // .expect("Couldn't get the liked recipes");

        let mut public_recipes = graph.execute(
            query(
                "MATCH (r:Recipe)-[:OWNS]-(u:User) \
                WHERE r.public = true AND NOT u.id = $u_id \
                RETURN r"
            )
            .param("u_id", usr.0.clone())
        )
        .await
        .expect("Error getting public recipes!");

        while let Ok(Some(row)) = public_recipes.next().await {
            recipes_vector.push(format_recipes(row))
        }

        recipes_vector.sort();
        recipes_vector.dedup();

        for recipe in &mut recipes_vector {
            get_ingredients_from_db(graph.clone(), recipe).await;
        }
        (recipes_vector, rel_struct)
    });

    Json(RecipeVec {
        recipes: recipes_vec.0,
        rels: Option::from(recipes_vec.1)
    })
}

#[delete("/remove/<r_id>")]
pub fn remove_recipe(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    u_id: UserId,
    r_id: String,
) -> Status {
    // let u_id = u_id.0;
    rt.block_on(async {
        graph
            .run(
                query(
                    "MATCH (u:User)-[:OWNS]->(r:Recipe) \
                WHERE u.id = $u_id AND r.id = $r_id \
                DETACH DELETE r",
                )
                .param("u_id", u_id.0.clone())
                .param("r_id", r_id.clone()),
            )
            .await
            .expect("Couldn't run query");
    });
    Status::NoContent
}

#[get("/<r_id>")]
pub fn get_recipe(
    graph: State<GraphPool>,
    rt: State<Runtime>,
    u_id: UserId,
    r_id: String,
) -> Json<Recipe> {
    let recipe = rt.block_on(async {
        let mut res = graph
            .execute(
                query(
                    "MATCH (u:User)-[:OWNS]->(r:Recipe) \
               WHERE (u.id = $u_id AND r.id = $r_id) OR (r.id = $r_id AND r.public = true) \
               RETURN r",
                )
                .param("u_id", u_id.0.clone())
                .param("r_id", r_id.clone()),
            )
            .await
            .expect("Error getting the recipe");
        let row = res.next().await;
        let mut recipe = format_recipes(row.expect("Error in row").expect("Empty row"));
        get_ingredients_from_db(graph.clone(), &mut recipe).await;
        recipe
    });
    Json(recipe)
}

#[get("/public/<r_id>")]
pub fn get_public_recipe(
    graph: State<GraphPool>,
    rt: State<Runtime>,
    r_id: String,
) -> std::result::Result<Json<Recipe>, Status> {
    let recipe = rt.block_on(async {
        let mut res = graph
            .execute(
                query(
                    "MATCH (r:Recipe) \
               WHERE r.id = $r_id AND r.public = true \
               RETURN r",
                )
                .param("r_id", r_id.clone()),
            )
            .await
            .expect("Error getting the recipe");
        let row = res.next().await;
        let row_option = row.expect("Error in row");
        if row_option.is_none() {
            return Err(Status::Unauthorized)
        }
        let mut recipe = format_recipes(row_option.unwrap());
        get_ingredients_from_db(graph.clone(), &mut recipe).await;
        Ok(recipe)
    });
    if recipe.is_err() {
        return Err(recipe.err().unwrap())
    }
    Ok(Json(recipe.unwrap()))
}

#[get("/share?<r_id>")]
pub fn share_recipe(graph: State<GraphPool>, rt: State<Runtime>, r_id: String) -> Json<Recipe> {
    let recipe = rt.block_on(async {
        let mut res = graph
            .execute(
                query(
                    "MATCH (r:Recipe) \
               WHERE r.id = $r_id \
               RETURN r",
                )
                .param("r_id", r_id.clone()),
            )
            .await
            .expect("Error getting the recipe");
        let row = res.next().await;
        let mut recipe = format_recipes(row.expect("Error in row").expect("Empty row"));
        get_ingredients_from_db(graph.clone(), &mut recipe).await;
        recipe
    });
    Json(recipe)
}

#[delete("/weeklyreset")]
pub fn reset_all_chosen(rt: State<Runtime>, graph: State<GraphPool>, u_id: UserId) -> Status {
    rt.block_on(async {
        graph
            .run(
                query(
                    "MATCH (u:User)-[c:CHOSEN]->() \
                WHERE u.id = $u_id \
                DETACH DELETE c",
                )
                .param("u_id", u_id.0.clone()),
            )
            .await
            .expect("Couldn't delete chosen relationships");
    });
    Status::NoContent
}

#[put("/like?<r_id>")]
pub fn like_recipe(
    rt: State<Runtime>,
    graph: State<GraphPool>,
    u_id: UserId,
    r_id: String,
) -> Status {
    rt.block_on(async {
        let mut recipe_stream =
            graph.execute(
                query(
                    "MATCH (r:Recipe)-[c:LIKES|OWNS]-(u:User) WHERE u.id = $u_id AND r.id = $r_id \
                RETURN r, c",
                )
                    .param("u_id", u_id.0.clone())
                    .param("r_id", r_id.clone()),
            )
                .await
                .expect("Couldn't find that recipe");

        let recipe_row_result = recipe_stream.next().await;
        let recipe_row = recipe_row_result.expect("Error fetching row");
        if recipe_row.is_none() {
            graph.run(
                query(
                    "MATCH (u:User), (r:Recipe) \
                    WHERE u.id = $u_id AND (r.id = $r_id AND r.public = true) \
                    MERGE (u)-[:LIKES]->(r)"
                )
                    .param("u_id", u_id.0.clone())
                    .param("r_id", r_id.clone()),
            )
                .await
                .expect("Couldn't like the recipe");
            return Status::Created;
        }
        let relationship_node = recipe_row.unwrap().get::<Relation>("c").unwrap();
        if &relationship_node.typ() == "OWNS" {
            return Status::NoContent;
        }
        graph.run(
            query(
                "MATCH (u:User)-[l:LIKES]->(r:Recipe) \
                WHERE u.id = $u_id AND r.id = $r_id \
                DETACH DELETE l"
            )
                .param("u_id", u_id.0.clone())
                .param("r_id", r_id.clone()),
        )
            .await
            .expect("Couldn't unlike the recipe");
        Status::Accepted
    })
}

#[get("/public")]
pub fn public_recipes(
    graph: State<GraphPool>,
    rt: State<Runtime>,
) -> Json<RecipeVec> {
    let recipes_vec = rt.block_on(async {
       let mut res = graph.execute(
           query(
               "MATCH (r:Recipe) WHERE r.public = true RETURN r"
           )
       )
       .await
       .expect("Error getting public recipe list");

        let mut recipes_vector = Vec::new();

        while let Ok(Some(row)) = res.next().await {
            recipes_vector.push(format_recipes(row))
        }

        for recipe in &mut recipes_vector {
            get_ingredients_from_db(graph.clone(), recipe).await;
        }
        recipes_vector
    });

    Json(RecipeVec {
        recipes: recipes_vec,
        rels: None
    })
}
