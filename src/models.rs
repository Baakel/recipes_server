use neo4rs::Graph;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;
use std::cmp::Ordering;
// use rocket::request::FromForm;

pub type GraphPool = Arc<Graph>;

// Helper for destructuring recipe id's. also useful un case we wanted to have a collection of
// users or some way of POSTing multiple elements for a route.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdsVec {
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecipeVec {
    pub recipes: Vec<Recipe>,
    // pub rels: Option<Vec<(Uuid, String)>>
    pub rels: Option<RecipeRelationships>
}

// #[derive(Debug, Deserialize, Serialize)]
// pub struct RelationshipVec {
//     pub relationship: (Uuid, String)
// }

#[derive(Debug, Deserialize, Serialize)]
pub struct RecipeRelationships {
    pub owns: Vec<Uuid>,
    pub likes: Vec<Uuid>
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
    pub time: Option<String>,
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

// Implementations
// Need to implement all these for Recipe so that we can sort it and dedup it later on
impl Ord for Recipe {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}
impl PartialOrd for Recipe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Recipe {
    fn eq(&self, other:&Self) -> bool {
        self.id == other.id
    }
}
// This one we leave empty, it's just telling the compiler that we are implementing it but it's
// not really doing anything and the compiler can't check so it will use PartialEq instead
impl Eq for Recipe {}