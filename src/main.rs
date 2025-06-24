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
use serde::Deserialize;

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
        Ok(_user) => {
            // Redirect to invitation page after registration
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/invitation"))
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

async fn invitation_page(
    env: web::Data<Arc<Environment<'_>>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let lang = query.get("lang").map(|s| s.as_str()).unwrap_or("en");
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("lang", minijinja::value::Value::from(lang));
    let tmpl = env.get_template("invitation.html").unwrap();
    let html = tmpl.render(ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[derive(Deserialize)]
struct AddPresentForm {
    present_name: String,
    secret: Option<String>,
}

async fn add_present(
    pool: web::Data<DbPool>,
    form: web::Form<AddPresentForm>,
) -> impl Responder {
    use schema::presents;
    
    let mut conn = pool.get().unwrap();
    let form_data = form.into_inner();
    
    // Determine status based on whether it's marked as secret
    let status = if form_data.secret.is_some() {
        "secret".to_string()
    } else {
        "available".to_string()
    };
    
    let new_present = NewPresent {
        user_id: None,
        name: form_data.present_name,
        status,
        link: String::new(), // Default empty link
    };
    
    let result = diesel::insert_into(presents::table)
        .values(&new_present)
        .execute(&mut conn);
        
    match result {
        Ok(_) => {
            // Redirect back to presents page
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/presents"))
                .finish()
        },
        Err(e) => {
            eprintln!("Failed to add present: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to add present: {}", e))
        },
    }
}

#[derive(Deserialize)]
struct SelectPresentForm {
    present_id: i32,
}

async fn select_present(
    pool: web::Data<DbPool>,
    form: web::Form<SelectPresentForm>,
) -> impl Responder {
    use schema::presents::dsl::*;
    
    let mut conn = pool.get().unwrap();
    let present_id = form.present_id;
    
    // Update the present status to 'taken'
    let result = diesel::update(presents.find(present_id))
        .set((
            status.eq("taken".to_string()),
            user_id.eq(1) // For simplicity, assign to user_id=1
        ))
        .execute(&mut conn);
        
    match result {
        Ok(_) => {
            // Redirect back to presents page
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/presents"))
                .finish()
        },
        Err(e) => {
            eprintln!("Failed to select present: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to select present: {}", e))
        },
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
            .route("/invitation", web::get().to(invitation_page))
            .route("/presents", web::get().to(presents_page))
            .route("/add_present", web::post().to(add_present))
            .route("/select_present", web::post().to(select_present))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
