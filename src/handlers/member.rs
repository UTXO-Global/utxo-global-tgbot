use actix_web::{web, HttpResponse};
use pg_bigdecimal::BigDecimal;

use crate::{
    serialize::{error::AppError, member::VerifyMemberReq},
    services::member::MemberSrv,
};

async fn verify(
    member_srv: web::Data<MemberSrv>,
    req: web::Json<VerifyMemberReq>,
) -> Result<HttpResponse, AppError> {
    member_srv.verify_signature(req.clone()).await?;
    // TODO: load balance
    let balance = pg_bigdecimal::PgNumeric::new(Some(BigDecimal::from(0)));
    member_srv
        .update_member(req.tgid, req.ckb_address.clone(), balance, req.dob, 1)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn route(conf: &mut web::ServiceConfig) {
    conf.service(web::scope("/users").route("/verify", web::post().to(verify)));
}
