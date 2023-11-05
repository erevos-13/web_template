use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Result;
use std::io::Write;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    id: u64,
    name: String,
    completed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u64,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Database {
    tasks: HashMap<u64, Task>,
    users: HashMap<u64, User>,
}

impl Database {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            users: HashMap::new(),
        }
    }
    // TODO CRUD DATA
    fn insert(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }
    fn get(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }
    fn getAll(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }
    fn delete(&mut self, id: u64) {
        self.tasks.remove(&id);
    }
    fn update(&mut self, id: u64, task: Task) {
        self.tasks.insert(id, task);
    }
    // User Data
    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }
    fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    fn get_user_by_name(&self, username: &str) -> Option<&User> {
        self.users.values().find(|u| u.username == username)
    }

    fn save_to_file(&self) -> Result<()> {
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create("db.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }
    fn load_from_file() -> Result<Self> {
        let file = fs::read_to_string("db.json")?;
        let data: Database = serde_json::from_str(&file)?;
        Ok(data)
    }
}

struct AppState {
    db: Mutex<Database>,
}

async fn create_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state.db.lock().expect("Fail to lock DB");
    db.insert(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().body("Task created")
}

async fn read_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut db = app_state.db.lock().expect("Fail to lock DB");
    match db.get(id.into_inner()) {
        Some(task) => HttpResponse::Ok().json(task),
        None => return HttpResponse::NotFound().body("Task not found"),
    }
}
async fn delete_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut db = app_state.db.lock().expect("Fail to lock DB");
    db.delete(id.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().body("Task deleted")
}
async fn update_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state.db.lock().expect("Fail to lock DB");
    db.insert(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().body("Task updated")
}
async fn read_all_task(app_state: web::Data<AppState>) -> impl Responder {
    let mut db = app_state.db.lock().expect("Fail to lock DB");
    let tasks = db.getAll();
    HttpResponse::Ok().json(tasks)
}

async fn register_user(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db: std::sync::MutexGuard<'_, Database> = app_state.db.lock().expect("Fail to lock DB");
    db.insert_user(user.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn login(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db: std::sync::MutexGuard<'_, Database> = app_state.db.lock().expect("Fail to lock DB");
    match db.get_user_by_name(&user.username) {
        Some(stored_user) if stored_user.password == user.password => {
            HttpResponse::Ok().body("Login success")
        }
        _ => HttpResponse::BadRequest().body("User not found"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new(),
    };
    let data = web::Data::new(AppState { db: Mutex::new(db) });
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .app_data(data.clone())
            .route("/task", web::post().to(create_task))
            .route("/task/{id}", web::get().to(read_task))
            .route("/tasks", web::get().to(read_all_task))
            .route("/task", web::patch().to(update_task))
            .route("/task/{id}", web::delete().to(delete_task))
            .route("/task", web::put().to(update_task))
            .route("/register", web::post().to(register_user))
            .route("/login", web::post().to(login))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
