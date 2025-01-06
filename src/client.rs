use std::time::{Instant, Duration};
use actix::{Addr, Actor, Handler, StreamHandler, AsyncContext, WrapFuture, ActorFutureExt, ActorContext, fut, dev::ContextFutureSpawner};
use actix_web_actors::ws;
use actix_ws::ProtocolError;
use uuid::Uuid;
use actix_web_actors::ws::Message;

use crate::wsserver::{ChatServer, MMessage, Connect, Disconnect, ClientMessage, Join, ListRooms, UserRename, MError, GetInfo};

const HB_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct UserSession {
    id: Uuid,
    hb: Instant,
    srv: Addr<ChatServer>,
}

impl UserSession {
    pub fn new(srv: Addr<ChatServer>) -> Self {
        Self {
            id: Uuid::new_v4(),
            hb: Instant::now(),
            srv,
        }
    }
}

impl Actor for UserSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb = Instant::now();
        ctx.run_interval(HB_INTERVAL, |user, ctx| {
            if Instant::now().duration_since(user.hb) > CLIENT_TIMEOUT {
                println!("[[ user {} is disconnecting due to timeout ]]", user.id);
                user.srv.do_send(Disconnect { id: user.id });
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });

        self.srv.send(Connect {
                addr: ctx.address().recipient()
            })
            .into_actor(self)
            .then(|res, user: &mut UserSession, ctx: &mut Self::Context| {
                match res {
                    Ok(res) => {
                        if let Ok(uid) = Uuid::parse_str(res.as_str()) {
                            user.id = uid;
                        }
                    },
                    Err(e) => {
                        println!("[Err occured {:?}]", e);
                        ctx.stop();
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::prelude::Running {
        self.srv.do_send(Disconnect { id: self.id });
        actix::prelude::Running::Stop
    }
}

impl Handler<MMessage> for UserSession {
    type Result = ();

    fn handle(&mut self, msg: MMessage, ctx: &mut Self::Context) -> Self::Result {
        if let Ok(v) = serde_json::to_string(&msg) {
            return ctx.text(v);
        }
        ctx.text(String::new());
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for UserSession {
    fn handle(&mut self, item: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        if let Err(e) = item {
            println!("[StreamHandler::Error occured {:?}]", e);
            ctx.stop();
            return;
        }

        let msg = item.unwrap();
        match msg {
            Message::Text(t) => {
                let str_msg = t.to_string();
                println!("[StreamHandler:: {:?} : {:?}]", self.id, str_msg);
                if str_msg.starts_with('/') {
                    let command_list: Vec<&str> = str_msg.splitn(2, ' ').collect();
                    match command_list[0] {
                        "/name" => {
                            self.srv.do_send(UserRename {
                                id: self.id,
                                new_name: command_list[1].to_string()
                            });
                        }
                        "/leave" => {
                            self.srv.do_send(Join {
                                id: self.id,
                                room: String::from("main")
                            });
                        }
                        "/join" => {
                            if command_list.len() != 2 || command_list[1].is_empty() {
                                self.srv.do_send(MError {
                                    id: self.id,
                                    etype: "Invalid Argument".to_string(),
                                    what: command_list[1].to_string()
                                });
                                return;
                            }
                            self.srv.do_send(Join {
                                id: self.id,
                                room: command_list[1].to_string()
                            });
                        }
                        "/list" => {
                            self.srv.do_send(ListRooms { id: self.id });
                        }
                        "/get-info" => {
                            self.srv.do_send(GetInfo { id: self.id });
                        }
                        _ => {
                            self.srv.do_send(MError {
                                id: self.id,
                                etype: "Unknown Command".to_string(),
                                what: command_list[0].to_string()
                            });
                        }
                    }
                }
                else {
                    self.srv.do_send(ClientMessage {
                        id: self.id,
                        msg: str_msg
                    });
                }
            }
            Message::Binary(b) => ctx.binary(b),
            Message::Continuation(_) => ctx.stop(),
            Message::Ping(b) => {
                self.hb = Instant::now();
                ctx.pong(&b);
            }
            Message::Pong(_) => {
                self.hb = Instant::now();
            }
            Message::Close(r) => {
                ctx.close(r);
                ctx.stop();
            }
            Message::Nop => (),
        }
    }
}
