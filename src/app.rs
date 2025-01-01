use crate::handlers::member;
use crate::{
    config,
    repositories::{
        self,
        db::{migrate_db, DB_POOL},
    },
    services,
};
use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    member::route(cfg);
}

pub async fn create_app() -> std::io::Result<()> {
    env_logger::init();
    // Init DB
    let db = &DB_POOL.clone();

    // migrate db
    if let Err(e) = migrate_db().await {
        println!("\nMigrate db failed: {}", e);
    }
    let member_dao = repositories::member::MemberDao::new(db.clone());
    let member_service = web::Data::new(services::member::MemberSrv::new(member_dao.clone()));

    let listen_address: String = config::get("listen_address");

    println!("\nListening and serving HTTP on {}", listen_address);

    HttpServer::new(move || {
        let cors: Cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);

        App::new()
            .app_data(member_service.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .configure(init_routes)
    })
    .bind(listen_address)?
    .run()
    .await
}
