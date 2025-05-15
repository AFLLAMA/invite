use diesel::prelude::*;
use crate::schema::presents::dsl::*;
use crate::schema::users::dsl::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub name: String,
}

#[derive(Queryable, Serialize, Associations, Identifiable)]
#[diesel(belongs_to(User))]
pub struct Present {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub link: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::presents)]
pub struct NewPresent {
    pub user_id: i32,
    pub name: String,
    pub link: String,
}