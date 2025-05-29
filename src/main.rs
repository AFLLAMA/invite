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

use actix_web::http::header;
use serde::Serialize;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

async fn welcome_page(env: web::Data<Arc<Environment<'_>>>) -> impl Responder {
    let tmpl = env.get_template("welcome.html").unwrap();
    let html = tmpl.render(()).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn register_user(
    pool: web::Data<DbPool>,
    env: web::Data<Arc<Environment<'_>>>,
    form: web::Form<NewUser>,
) -> impl Responder {
    use schema::users;

    let mut conn = pool.get().unwrap();
    let new_user = form.into_inner();

    let result = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&mut conn);

    match result {
        Ok(user) => {
            HttpResponse::Found()
                .append_header((header::LOCATION, format!("/presents?user_id={}", user.id)))
                .finish()
        },
        Err(e) => {
            eprintln!("Registration failed: {}", e);
            HttpResponse::InternalServerError().body(format!("Registration failed: {}", e))
        },
    }
}

#[derive(Serialize)]
struct PresentCtx {
    id: i32,
    name: String,
    status: String,
    taken_by: Option<String>,
}

async fn presents_page(
    pool: web::Data<DbPool>,
    env: web::Data<Arc<Environment<'_>>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    use schema::presents::dsl::*;
    let mut conn = pool.get().unwrap();
    let present_list = presents.load::<Present>(&mut conn).unwrap_or_default();
    let presents_ctx: Vec<PresentCtx> = present_list.iter().map(|p| {
        let (present_status, taken_by) = if p.name.to_lowercase().contains("secret") {
            if p.user_id.is_some() {
                ("secret".to_string(), Some("taken".to_string()))
            } else {
                ("secret".to_string(), None)
            }
        } else if p.user_id.is_some() {
            ("taken".to_string(), Some("taken".to_string()))
        } else {
            ("available".to_string(), None)
        };
        PresentCtx {
            id: p.id,
            name: p.name.clone(),
            status: present_status, // <-- use the new variable here
            taken_by,
        }
    }).collect();
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("presents", minijinja::value::Value::from_serialize(&presents_ctx));
    let tmpl = env.get_template("presents.html").unwrap();
    let html = tmpl.render(ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
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
            .route("/presents", web::get().to(presents_page))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
