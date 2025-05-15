mod schema;
mod models;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use diesel::{r2d2::{self, ConnectionManager}, PgConnection, prelude::*};
use models::*;
use dotenvy::dotenv;
use std::env;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

async fn create_user(pool: web::Data<DbPool>, form: web::Json<NewUser>) -> impl Responder {
    use schema::users;

    let conn = pool.get().unwrap();
    let inserted = diesel::insert_into(users::table)
        .values(&form.into_inner())
        .get_result::<User>(&mut conn);

    match inserted {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn add_present(pool: web::Data<DbPool>, form: web::Json<NewPresent>) -> impl Responder {
    use schema::presents;

    let conn = pool.get().unwrap();
    let inserted = diesel::insert_into(presents::table)
        .values(&form.into_inner())
        .get_result::<Present>(&conn);

    match inserted {
        Ok(present) => HttpResponse::Ok().json(present),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool = r2d2::Pool::builder().build(manager).expect("Failed to create pool.");

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/user", web::post().to(create_user))
            .route("/present", web::post().to(add_present))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}