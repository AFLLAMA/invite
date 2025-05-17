use diesel::{Identifiable, Insertable, Queryable, Associations};
use serde::{Deserialize, Serialize};
use crate::schema::{users, presents};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = presents)]
pub struct Present {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: String,
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = presents)]
pub struct NewPresent {
    pub user_id: Option<i32>,
    pub name: String,
    pub link: String,
}