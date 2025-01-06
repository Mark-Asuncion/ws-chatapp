use std::collections::{HashMap, HashSet};
use actix::Actor;
use actix::Context;
use actix::Handler;
use actix::Message;
use actix::Recipient;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

pub const T_SENDER_USER: &str = "user";
pub const T_SENDER_SYSTEM: &str = "system";
pub const T_SENDER_SYSTEM_ERR: &str = "error";

/// Chat server sends this messages to session
#[derive(Message, Debug, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct MMessage {
    pub message        : String,
    pub sender_type    : String,
    pub sender_name    : String,
    pub set_info       : Option<(String, String)>
}

impl MMessage {
    pub fn from(message: &str, sender_type: &str, sender_name: &str) -> Self {
        Self {
            message: message.to_string(),
            sender_type: sender_type.to_string(),
            sender_name: sender_name.to_string(),
            set_info: None
        }
    }

    pub fn from_set_info(user: &User) -> Self {
        Self {
            message: String::new(),
            sender_type: String::new(),
            sender_name: String::new(),
            set_info: Some((user.name.to_string(), user.room.to_string()))
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GetInfo {
    pub id: Uuid
}

/// New chat session is created
#[derive(Message)]
#[rtype(String)]
pub struct Connect {
    pub addr: Recipient<MMessage>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: Uuid,
    pub room: String
}

/// User disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: Uuid,
    pub msg: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ListRooms {
    pub id: Uuid
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MError {
    pub id: Uuid,
    pub etype: String,
    pub what: String,
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct UserRename {
    pub id: Uuid,
    pub new_name: String
}


#[derive(Debug, Clone, Default)]
pub struct User {
    pub name: String,
    pub room: String
}

#[derive(Debug)]
pub struct ChatServer {
    // HashMap<id of room, list of users in id (can be found in sessions)>
    // to get addr of user from id pass id to sessions
    rooms: HashMap<String, HashSet<Uuid>>,
    sessions: HashMap<Uuid, (User, Recipient<MMessage>)>,
}

impl ChatServer {
    pub fn new() -> Self {
        let mut rooms = HashMap::new();
        // default room
        rooms.insert("main".to_string(), HashSet::new());

        Self {
            rooms,
            sessions: HashMap::new()
        }
    }

    pub fn send_to(&self, room_id: &str, uid_sender: Option<Uuid>, msg: &str) {
        let room = self.rooms.get(room_id);
        if room.is_none() {
            return;
        }
        let room = room.unwrap();
        let mut sender_info = None;
        if let Some(sender_id) = uid_sender {
            if let Some((u, _)) = self.sessions.get(&sender_id) {
                sender_info = Some(u.clone());
            }
        }
        for uid_user in room {
            if let Some((_, addr)) = self.sessions.get(uid_user) {
                let st = {
                    if uid_sender.is_none() {
                        T_SENDER_SYSTEM.to_string()
                    }
                    else {
                        T_SENDER_USER.to_string()
                    }
                };
                let sn = {
                    if let Some(u) = &sender_info {
                        u.name.clone()
                    }
                    else {
                        T_SENDER_SYSTEM.to_string()
                    }
                };

                let m = MMessage::from(msg, &st, &sn);
                dbg!(&m);
                addr.do_send(m);
            }
        }
    }

    pub fn leave_room(&mut self,id: &Uuid, room_name: &str) {
        if let Some(list) = self.rooms.get_mut(room_name) {
            list.remove(id);
            if let Some((user, _)) = self.sessions.get_mut(id) {
                user.room = String::new();
            }
            if list.is_empty() {
                self.rooms.remove(room_name);
            }
        }
    }

    pub fn join_room(&mut self,id: Uuid, room_name: &str) {
        self.rooms.entry(room_name.to_string())
            .or_default().insert(id);
        if let Some((user, _)) = self.sessions.get_mut(&id) {
            user.room = room_name.to_string();
        }
    }
}

// Simple Actor to communicate with other Actors (which will be the users)
impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = String;

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4();

        self.sessions.insert(id, (User {
            name: id.to_string(),
            room: String::new()
        }, msg.addr));
        self.join_room(id, "main");
        self.send_to(
            "main",
            None,
            format!("[new user {} Joined to main room]", id).as_str()
        );

        id.to_string()
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) -> Self::Result {
        // self.sessions.remove(&msg.id);
        let user_info = self.sessions.get(&msg.id);
        if let Some((u, _)) = user_info {
            let u = u.clone();
            self.sessions.remove(&msg.id);
            self.leave_room(&msg.id, &u.room);
            self.send_to(u.room.as_str(), None, format!("[user {} disconnected]", u.name).as_str());
        }
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Self::Context) -> Self::Result {
        let user_info = self.sessions.get(&msg.id);
        if let Some((u, _)) = user_info {
            self.send_to(u.room.as_str(), Some(msg.id), msg.msg.as_str());
        }
    }
}

impl Handler<ListRooms> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ListRooms, _: &mut Self::Context) -> Self::Result {

        if let Some((_, addr)) = self.sessions.get(&msg.id) {
            let keys: Vec<String> = self.rooms.clone().into_keys()
                .collect();
            let mut names_str = "\n".to_string();
            for key in keys {
                names_str += format!("{}\n", key).as_str();
            }

            addr.do_send(MMessage::from(&names_str, T_SENDER_SYSTEM, T_SENDER_SYSTEM));
        }
    }
}

impl Handler<UserRename> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: UserRename, _: &mut Self::Context) -> Self::Result {
        let invalid_chars = " '\"".as_bytes();
        for c in msg.new_name.clone().into_bytes() {
            if invalid_chars.contains(&c) {
                if let Some((_, addr)) = self.sessions.get_mut(&msg.id) {
                    addr.do_send(MMessage::from(
                        format!("[Error \"{}\" contains invalid character]", msg.new_name).as_str(),
                        T_SENDER_SYSTEM, T_SENDER_SYSTEM
                    ));
                }
                return;
            }
        }

        #[allow(clippy::for_kv_map)]
        for (_, (user, _)) in &self.sessions {
            if user.name == msg.new_name {
                if let Some((_, addr)) = self.sessions.get_mut(&msg.id) {
                    addr.do_send(
                        MMessage::from(
                            format!("[Error changing name. Name {} already exists]", msg.new_name).as_str(),
                            T_SENDER_SYSTEM, T_SENDER_SYSTEM
                        )
                    );
                }
                return;
            }
        }

        if let Some((user, addr)) = self.sessions.get_mut(&msg.id) {
            let oldname = user.name.clone();
            user.name = msg.new_name.clone();
            let user = user.clone();
            let m = format!("[Successfully changed name from {} -> {}]", oldname, user.name);
            addr.do_send(MMessage::from_set_info(&user));
            self.send_to(&user.room, Some(msg.id), &m);
        }
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Self::Context) -> Self::Result {
        let user_info = self.sessions.get(&msg.id);
        if let Some((u, addr)) = user_info {
            let u = u.clone();
            let uu = User {
                name: u.name.clone(),
                room: msg.room.clone()
            };
            addr.do_send(MMessage::from_set_info(&uu));
            self.leave_room(&msg.id, &u.room);
            self.send_to(&u.room, None, format!("[User {} leaved the room {}]", u.name, u.room).as_str());
            self.join_room(msg.id, &msg.room);
            self.send_to(&msg.room, None, format!("[User {} joined the room {}]", u.name, msg.room).as_str());
        }
    }
}

impl Handler<MError> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: MError, _: &mut Self::Context) -> Self::Result {
        if let Some((_, addr)) = self.sessions.get(&msg.id) {
            let err = format!("[{} {}]", msg.etype, msg.what);
            addr.do_send(MMessage::from(err.as_str(), T_SENDER_SYSTEM_ERR, T_SENDER_SYSTEM));
        }
    }
}

impl Handler<GetInfo> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: GetInfo, _: &mut Self::Context) -> Self::Result {
        if let Some((u, addr)) = self.sessions.get(&msg.id) {
            addr.do_send(MMessage::from_set_info(u));
        }
    }
}

