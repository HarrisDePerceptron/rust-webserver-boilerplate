use std::sync::Mutex;

use actix_web::{error, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder, body::BoxBody, FromRequest, cookie::Key};
use std::time::{ Duration};
use serde::{Serialize, Deserialize};

use std::fmt::{Debug};

use serde_json;


use actix_web::middleware::{Logger, DefaultHeaders};
use env_logger::Env;

use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};

use hello_web::upload::Upload;

use actix_multipart::Multipart;


#[get("/")]
async fn index(data: web::Data<AppStateShare>) -> String {
    let mut m = data.counter.lock().unwrap();
    let mut counter = *m;
    counter += 1;
    *m = counter;

    format!("hello world, your app count is {}", counter)
}



#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {

   
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("hey there ther")
}


#[derive(Serialize,Deserialize)]
struct UserParam {
    user_id: u32,
    friend: String

}

#[get("/users/{user_id}/{friend}")]
async fn users(path: web::Path<UserParam>) -> Result<String, Box<dyn std::error::Error>>{
    
    Ok(format!("i got {} user and friend {}", path.user_id, path.friend))
}


struct AppState {
    app_name: String
}


struct AppStateShare {
    counter: Mutex<i32>
}


#[derive(Deserialize, Debug)]
struct QueryParam {
    username: String,
    password: Option<String>
}



#[get("/testquery")]
async fn test_query(info: web::Query<QueryParam>) -> impl Responder{

    let query: String = format!("Query {{ {:#?} }}", info);
    
    HttpResponse::Ok().body(query)

}


#[derive(Debug, Deserialize, Serialize)]
struct BodyParm {
    username: String,
    age: i32,
    weight: f32
}

impl Responder for BodyParm {
    type Body =BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();
        HttpResponse::Ok().body(body)
    }

   
}

#[post("/test_json")]
async fn test_json(param: web::Json<BodyParm>, session: Session)-> impl Responder{
    if let Some(count) = session.get::<i32>("counter").unwrap() {
        println!("SESSION value: {}", count);
        // modify the session state
        session.insert("counter", count + 1).unwrap();
    } else {
        session.insert("counter", 1).unwrap();
    }

    let serialize = format!("Here are the body params: {:#?}",  param);
    
    param
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    status: String,
    message: String
}

#[post("/upload-file")]
async fn upload_file (payload: Multipart)-> impl Responder{
    let res = Upload::save_file(payload, "./tmp").await;

    if let Ok(res) = res {
        for r in res.iter() {
            if let Upload::MultipartResponse::TEXT(r) = r {
                println!("got value: {:#?}", r);
            }
        }
        HttpResponse::Ok().body("Files Uploaded")

    }else if let Err(e) = res{
        let msg = e.to_string();
        HttpResponse::BadRequest().body(msg)
    }else{
        HttpResponse::Ok().body("Files Uploaded")
    }

}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world!");

    std::fs::create_dir_all("./tmp").expect("unable to create directory");

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    let secret_key = Key::generate();

    let state = web::Data::new(AppStateShare {
        counter: Mutex::new(0)
    });

    HttpServer::new(move || {
        let json_config = web::JsonConfig::default()
            .limit(4096)
            .error_handler( |err, _req| {    

                // error::InternalError::from_response(err,HttpResponse::Conflict().finish())
                //     .into()

                let err = ErrorResponse {
                    message: err.to_string(),
                    status: "400".to_owned()
                };
                let res = serde_json::to_string(&err).unwrap();

                 error::InternalError::from_response("",HttpResponse::BadRequest().content_type("application/json").body(res))
                    .into()
                
            });


        App::new()
        .wrap(Logger::new("%a %{User-Agent}i"))
        .wrap(DefaultHeaders::new().add(("X-Version", "0.2")))
        .wrap(
            SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone()
            )
        )
            .app_data(state.clone())
            .app_data(json_config)
            .service(index)
            .service(web::scope("/hello")
                .service(echo)
                .service(test_query)
                .service(test_json)
                .route("/hey", web::get().to(manual_hello)))
            .service(echo)
            .service(users)
            .route("/hey", web::get().to(manual_hello))
            .service(
                web::scope("/file")
                .service(upload_file)
            )
    })
    .bind(("127.0.0.1", 8085))?
    .run()
    .await
}
