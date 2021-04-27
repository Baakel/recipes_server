use validator::Validate;
use uuid::Uuid;
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Recipe {
    pub id: Option<Uuid>,
    pub name: String,
    pub public: bool,
    pub steps: Option<Vec<String>>,
    pub tipo: Option<String>,
    pub calories: Option<u16>,
    pub carbohydrates: Option<f32>,
    pub fat: Option<f32>,
    pub protein: Option<f32>,
    pub servings: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct User {
    pub id: Option<Uuid>,
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(length(min = 10))]
    pub password: String,
    #[validate(email)]
    pub email: String,
    pub role: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Ingredient {
    pub name: String,
    pub tipo: Option<String>,
    pub amount: String,
}