#[macro_use]
extern crate redis_async;
extern crate uuid;

use actix::prelude::*;
use actix_web::{HttpServer, App, middleware, Error as AWError, web, HttpResponse};
use actix_redis::{RedisActor, Command, RespValue};
use actix_web::web::Path;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct Note {
    title: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct NoteWithId {
    id: String,
    note: Note
}

async fn create(
    info: web::Json<Note>,
    redis: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, AWError> {
    let id = Uuid::new_v4().to_string();
    let with_id = NoteWithId {
        id,
        note: info.into_inner()
    };
    let json = serde_json::to_string(&with_id.note)?;
    let result = redis.send(Command(resp_array!["SET", &with_id.id, json])).await?;
    match result {
        Ok(_) => {
            Ok(HttpResponse::Created().json(with_id))
        }
        _ => {
            println!("---->{:?}", result);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

async fn read(
    id: Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, AWError> {
    let id_string = id.into_inner();
    let result = redis.send(Command(resp_array!["GET", &id_string])).await?;
    match result {
        Ok(RespValue::SimpleString(response)) => {
            let with_id = NoteWithId {
                id: id_string,
                note: serde_json::from_str(&response)?
            };
            Ok(HttpResponse::Ok().json(with_id))
        }
        _ => {
            println!("---->{:?}", result);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

async fn update(
    id: Path<String>,
    info: web::Json<Note>,
    redis: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, AWError> {
    let json = serde_json::to_string(&info.into_inner())?;
    let result = redis.send(Command(resp_array!["SET", id.into_inner(), json])).await?;
    match result {
        Ok(RespValue::SimpleString(response)) => {
            Ok(HttpResponse::Ok().json(response))
        }
        _ => {
            println!("---->{:?}", result);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

async fn delete(
    id: Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, AWError> {
    let result = redis.send(Command(resp_array!["DEL", id.into_inner()])).await?;
    match result {
        Ok(_) => {
            Ok(HttpResponse::Accepted().finish())
        }
        _ => {
            println!("---->{:?}", result);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let redis_addr = RedisActor::start("127.0.0.1:6379");
        App::new()
            .data(redis_addr)
            .wrap(middleware::Logger::default())
            .service(web::resource("/todo")
                    .route(web::post().to(create)))
            .service(web::resource("/todo/{id}")
                .route(web::get().to(read))
                .route(web::put().to(update))
                .route(web::delete().to(delete)))
    })
    .bind("127.0.0.1:8080")
    .expect("Can not bind to 127.0.0.1:8080")
    .run()
    .await
}