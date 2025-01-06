use std::env;

use actix_web::{
    middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer
};
use actix::Addr;
use actix::Actor;
use actix_web_actors::ws;

mod wsserver;
mod client;

async fn ws(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<wsserver::ChatServer>>,
) -> Result<HttpResponse, Error> {
    // start websocket connection
    ws::start(
        client::UserSession::new(srv.get_ref().clone()),
        &req,
        stream,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let mut args = env::args();
    let mut port: u16 = 0;
    args.next();
    while let Some(arg) = args.next() {
        if arg == "-p" || arg == "--port" {
            if let Some(v) = args.next() {
                match v.parse() {
                    Ok(arg_port) => {
                        port = arg_port
                    },
                    Err(e) => {
                        println!("Error Occured {:?}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    if port == 0 {
        println!("Port not specified.\n Specify with [--port|-p <PORT>] option");
        std::process::exit(1);
    }

    let server = wsserver::ChatServer::new().start();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .route("/ws", web::get().to(ws))
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
