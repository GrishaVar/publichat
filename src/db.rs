use rusqlite::Connection;

use crate::{Message, Hash, Query};

pub fn create(path: Option<&str>) -> Connection {
    let conn = if let Some(path) = path {
        Connection::open(path).expect("Failed to open path")
    } else {
        Connection::open_in_memory().expect("Failed to make new db")
    };
    conn.execute_batch("\
        BEGIN; \
        CREATE TABLE Messages ( \
            chat      blob(64)  NOT NULL, \
            user      blob(64)  NOT NULL, \
            time      blob(16)  NOT NULL, \
            rsa_pub   blob(64)  NOT NULL, \
            signature blob(63)  NOT NULL, \
            message   blob(512) NOT NULL \
        ); \
        COMMIT; \
    ").expect("Failed to create table");
    conn
}

pub fn add_msg(conn: &Connection, msg: Message) {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO Messages (chat, user, time, rsa_pub, signature, message) \
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    ).expect("Failed to make cached add_msg query");
    stmt.execute(&[
        &msg.chat_id,
        &msg.user_id,
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).expect("Time travelers!")
            .as_nanos().to_be_bytes()[..16],
        &msg.rsa_pub,
        &msg.signature,
        &msg.contents,
    ]).expect("Failed to add message");
}

pub fn fetch(conn: &Connection, chat_id: Hash) -> Vec<Message> {  // todo: consider returning iterator somehow?
    let mut res = Vec::with_capacity(50);
    let mut stmt = conn.prepare_cached(
        "SELECT * FROM Messages WHERE chat = ? \
        ORDER BY rowid DESC LIMIT 50"
    ).expect("Failed to make cached fetch query");
    let msgs = stmt.query_map([&chat_id], |row| Ok(Message {
        chat_id: row.get(0).unwrap(),
        user_id: row.get(1).unwrap(),
        time: row.get(2).unwrap(),
        rsa_pub: row.get(3).unwrap(),
        signature: row.get(4).unwrap(),
        contents: row.get(5).unwrap(),
    })).expect("Failed to convert");
    res.extend(msgs.map(|msg| msg.unwrap()));
    println!("debug: {:?}", res.len());
    res  // returns newest message first!!!
}

pub fn query(conn: &Connection, query: Query) -> Vec<Message> {
    todo!();
}