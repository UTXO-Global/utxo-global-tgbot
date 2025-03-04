use actix_web::{web, HttpResponse, Responder};

pub async fn welcome() -> impl Responder {
    HttpResponse::Ok().body("Welcome to TelegramBot API!")
}

pub fn route(conf: &mut web::ServiceConfig) {
    conf.route("/", web::get().to(welcome));
}
