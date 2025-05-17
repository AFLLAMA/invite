mod schema;
mod models;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_web::middleware::Logger;
use diesel::{r2d2::{self, ConnectionManager}, PgConnection, prelude::*};
use minijinja::Environment;
use actix_web::web::Data;
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use models::*;
use env_logger;
 

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

async fn welcome_page(env: web::Data<Arc<Environment<'_>>>) -> impl Responder {
    let tmpl = env.get_template("welcome.html").unwrap();
    let html = tmpl.render(()).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn register_user(
    pool: web::Data<DbPool>,
    form: web::Form<NewUser>,
) -> impl Responder {
    use schema::users;

    let mut conn = pool.get().unwrap();
    let new_user = form.into_inner();

    let result = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&mut conn);

    match result {
        Ok(user) => HttpResponse::Ok().body(format!("Welcome, {}!", user.name)),
        Err(_) => HttpResponse::InternalServerError().body("Registration failed"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool = r2d2::Pool::builder().build(manager).expect("Failed to create pool.");

    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    let env = Arc::new(env);

    println!("🚀 Server running at http://127.0.0.1:8080/");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(env.clone()))
            .wrap(Logger::default())
            .route("/", web::get().to(welcome_page))
            .route("/register", web::post().to(register_user))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
