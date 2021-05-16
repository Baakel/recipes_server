use neo4rs::*;
use uuid::Uuid;
use crate::models::{Recipe, Ingredient, GraphPool};


pub fn process_steps(steps_string: String) -> Option<Vec<String>> {
    let split_string: Vec<_> = steps_string.lines().map(|s| s.to_string()).collect();
    Option::from(split_string)
}

pub fn format_recipes(row: Row) -> Recipe {
    let node = row.get::<Node>("r").expect("Empty row");
    let id = node.get::<String>("id").expect("No id found for node");
    let name = node
        .get::<String>("name")
        .unwrap_or_else(|| "No name found for node".to_string());
    let steps = node
        .get::<String>("steps")
        .unwrap_or_else(|| "".to_string());
    let public = node.get::<bool>("public");
    let tipo = node.get::<String>("tipo");
    let calories = node.get("calories").unwrap_or(0);
    let carbohydrates = node.get("carbohydrates").unwrap_or(0.0);
    let fat = node.get("fat").unwrap_or(0.0);
    let protein = node.get("protein").unwrap_or(0.0);
    let servings = node.get::<String>("servings");
    let meal_type = node.get::<String>("meal_type");
    let time = node.get::<String>("time");

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
        time
    };

    recipe
}

pub fn format_ingredients(row: Row) -> Ingredient {
    let node = row.get::<Node>("i").expect("Empty ingredient node");
    let name = node.get::<String>("name").expect("No ingredient name");
    let tipo = node.get::<String>("tipo").unwrap_or_else(|| "".to_string());
    let relation = row.get::<Relation>("u").expect("No relation");
    let amount = relation.get::<String>("amount").expect("No amount");

    Ingredient {
        name,
        tipo: Option::from(tipo),
        amount,
    }
}

pub async fn get_ingredients_from_db(
    graph: GraphPool,
    recipe: &mut Recipe
) {
    let mut response = graph.execute(
        query("MATCH (r:Recipe)-[u:USES]->(i:Ingredient) WHERE r.id = $rid RETURN i, u")
            .param("rid", recipe.id.unwrap().to_string())
    )
        .await
        .expect("Couldn't query the ingredients");

    let mut ingredients_vector = Vec::new();

    while let Ok(Some(row)) = response.next().await {
        ingredients_vector.push(format_ingredients(row))
    }
    recipe.ingredients = Option::from(ingredients_vector)
}
