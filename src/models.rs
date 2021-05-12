use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use std::sync::Arc;
use neo4rs::Graph;
// use rocket::request::FromForm;


pub type GraphPool = Arc<Graph>;

// Helper for destructuring recipe id's. also useful un case we wanted to have a collection of
// users or some way of POSTing multiple elements for a route.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdsVec {
    pub ids: Vec<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecipeVec {
    pub recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: Option<Uuid>,
    pub name: String,
    pub public: Option<bool>,
    pub steps: Option<Vec<String>>,
    pub tipo: Option<String>,
    pub calories: Option<u16>,
    pub carbohydrates: Option<f32>,
    pub fat: Option<f32>,
    pub protein: Option<f32>,
    pub servings: Option<String>,
    pub meal_type: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct User {
    pub id: Option<Uuid>,
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(length(min = 10))]
    pub password: String,
    #[validate(email)]
    pub email: Option<String>,
    pub role: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct Ingredient {
    pub name: String,
    pub tipo: Option<String>,
    pub amount: String,
}

#[derive(Debug, FromForm)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

// Used as a sort of JWT just verifying that the user has a valid id in the db.
#[derive(Debug)]
pub struct UserId(pub String);

// Used to return the error in outcome
#[derive(Debug)]
pub enum UsedIdError {
    Missing,
    Invalid,
}

#[derive(Debug)]
pub struct ChosenDeleted(pub bool);

#[derive(Debug)]
pub enum ChosenTimeError {
    Missing,
    Invalid,
}
