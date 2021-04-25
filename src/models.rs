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
    pub carbohydrates: Option<u16>,
    pub fat: Option<u16>,
    pub protein: Option<u16>,
    pub ingredients: Option<Vec<Ingredient>>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct User {
    pub username: String,
    #[validate(length(min = 10))]
    pub password: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Ingredient {
    pub name: String,
    pub tipo: Option<String>,
}