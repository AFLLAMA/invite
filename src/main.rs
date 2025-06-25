mod schema;
mod models;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_web::middleware::Logger;
use diesel::{r2d2::{self, ConnectionManager}, PgConnection, prelude::*};
use minijinja::{Environment, value::Value as MValue};
use actix_web::web::Data;
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use models::*;
use env_logger;
use serde_json;

use actix_web::http::header;
use serde::Serialize;
use serde::Deserialize;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

async fn welcome_page(
    env: web::Data<Arc<Environment<'_>>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let lang = query.get("lang").map(|s| s.as_str()).unwrap_or("en");
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("lang", MValue::from(lang));
    ctx.insert("current_path", MValue::from("/"));
    // Pass an empty HashMap for hidden_fields
    ctx.insert("hidden_fields", MValue::from(std::collections::HashMap::<String, String>::new()));
    
    let tmpl = env.get_template("welcome.html").unwrap();
    let html = tmpl.render(ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn register_user(
    pool: web::Data<DbPool>,
    form: web::Form<NewUser>,
) -> impl Responder {
    use schema::users;

    let mut conn = pool.get().unwrap();
    let new_user = form.into_inner();
    let lang = new_user.lang.clone().unwrap_or_else(|| "en".to_string());

    let result = diesel::insert_into(users::table)
        .values(new_user)
        .get_result::<User>(&mut conn);

    match result {
        Ok(_user) => {
            // Redirect to invitation page after registration with language
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/invitation?lang={}", lang)))
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
    
    let lang = query.get("lang").map(|s| s.as_str()).unwrap_or("en");
    
    let mut ctx = std::collections::HashMap::new();
    
    // Serialize directly using serde_json's to_value
    let json_string = serde_json::to_string(&presents_ctx).unwrap_or_else(|_| "[]".to_string());
    
    // Create a string-based represents of the presents data
    // This avoids the MValue serialization issues
    ctx.insert("presents_json", MValue::from(json_string));
    
    // Add an empty presents list for backward compatibility with the template
    // Using empty object instead of empty vec to avoid type inference issues
    ctx.insert("presents", MValue::from(()));
    
    // We'll modify the template to parse this JSON string
    
    ctx.insert("lang", MValue::from(lang));
    ctx.insert("current_path", MValue::from("/presents"));
    // Pass an empty HashMap for hidden_fields
    ctx.insert("hidden_fields", MValue::from(std::collections::HashMap::<String, String>::new()));
    
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
    ctx.insert("lang", MValue::from(lang));
    ctx.insert("current_path", MValue::from("/invitation"));
    // Pass an empty HashMap for hidden_fields
    ctx.insert("hidden_fields", MValue::from(std::collections::HashMap::<String, String>::new()));
    let tmpl = env.get_template("invitation.html").unwrap();
    let html = tmpl.render(ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[derive(Deserialize)]
struct AddPresentForm {
    present_name: String,
    secret: Option<String>,
    lang: Option<String>,
}

async fn add_present(
    pool: web::Data<DbPool>,
    form: web::Form<AddPresentForm>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    use schema::presents;
    
    let mut conn = pool.get().unwrap();
    let form_data = form.into_inner();
    
    // First check form for lang, then fallback to query parameter, then default to "en"
    let lang = match &form_data.lang {
        Some(l) => l.clone(),
        None => query.get("lang").map(|s| s.to_string()).unwrap_or_else(|| "en".to_string()),
    };
    
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
            // Redirect back to presents page with language
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/presents?lang={}", lang)))
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
    lang: Option<String>,
}

async fn select_present(
    pool: web::Data<DbPool>,
    form: web::Form<SelectPresentForm>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    use schema::presents::dsl::*;
    
    let mut conn = pool.get().unwrap();
    let present_id = form.present_id;
    
    // First check form for lang, then fallback to query parameter, then default to "en"
    let lang = match &form.lang {
        Some(l) => l.clone(),
        None => query.get("lang").map(|s| s.to_string()).unwrap_or_else(|| "en".to_string()),
    };
    
    // Update the present status to 'taken'
    let result = diesel::update(presents.find(present_id))
        .set((
            status.eq("taken".to_string()),
            user_id.eq(1) // For simplicity, assign to user_id=1
        ))
        .execute(&mut conn);
        
    match result {
        Ok(_) => {
            // Redirect back to presents page with language
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, format!("/presents?lang={}", lang)))
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

    // Get database URL from environment
    let db_url = match env::var("DATABASE_URL") {
        Ok(url) => {
            println!("Using database URL from environment: {}", url);
            url
        },
        Err(_) => {
            eprintln!("DATABASE_URL not found in environment! Please add it to .env file");
            panic!("DATABASE_URL environment variable is required");
        }
    };

    // Create database connection for migrations
    let mut conn = match PgConnection::establish(&db_url) {
        Ok(conn) => {
            println!("✅ Database connection established successfully");
            conn
        },
        Err(e) => {
            eprintln!("❌ Error connecting to database: {}", e);
            panic!("Failed to connect to database: {}", e);
        }
    };

    // Run migrations
    match conn.run_pending_migrations(MIGRATIONS) {
        Ok(_) => println!("✅ Database migrations completed successfully"),
        Err(e) => eprintln!("❌ Failed to run migrations: {}", e)
    }

    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool = r2d2::Pool::builder().build(manager).expect("Failed to create pool.");

    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    let env = Arc::new(env);

    // Better approach using environment variables
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    
    // Use 127.0.0.1 for local development and 0.0.0.0 for production (fly.io)
    let host = std::env::var("APP_ENV")
        .map(|env| if env == "production" { "0.0.0.0" } else { "127.0.0.1" })
        .unwrap_or("127.0.0.1");
    
    println!("Binding server to {}:{}", host, port);
    let bind_address = format!("{}:{}", host, port);

    println!("🚀 Server running at {}/", bind_address);

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
    .bind(bind_address)?  // This will work on any platform
    .run()
    .await
}
