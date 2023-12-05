use std::collections::HashMap;

use deadpool_sqlite::{Config, Object, Runtime};
use regex::Regex;
use rusqlite::ToSql;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Field {
    pub form_type: HTMLFieldType,
    pub required: bool,
    pub pattern: String,
    pub order: u8,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum HTMLFieldType {
    Text,
    Password,
    Email,
}

pub type UserSchema = HashMap<String, Field>;

struct DefaultUserSchema;

impl DefaultUserSchema {
    fn new() -> UserSchema {
        let mut schema = UserSchema::new();
        schema.insert(
            "Email".to_string(),
            Field {
                form_type: HTMLFieldType::Email,
                required: true,
                pattern: "^[^\\s]+@[^\\s]+\\.\\w+$".to_string(),
                order: 1,
            },
        );
        schema.insert(
            "Password".to_string(),
            Field {
                form_type: HTMLFieldType::Password,
                required: true,
                pattern: "^[^\\s]+$".to_string(),
                order: 2,
            },
        );
        schema.insert(
            "Username".to_string(),
            Field {
                form_type: HTMLFieldType::Text,
                required: true,
                pattern: "^[a-zA-Z0-9]+$".to_string(),
                order: 3,
            },
        );
        return schema;
    }
}

pub async fn query_admin_table(mut form_data: HashMap<String, String>) {
    let form_data_clone = form_data.clone();
    let keys = form_data_clone.keys();
    for key in keys {
        if Regex::new(r"(?i)password").expect("Failed to compile regex").is_match(key) {
            form_data.remove_entry(key);
        }
    }
    let fir_qry = format!("SELECT * FROM users LIMIT 1 WHERE {}", form_data.keys().map(|x| format!("{} = ?", x)).collect::<Vec<String>>().join(" AND "));
    println!("{}", fir_qry);
}

pub async fn insert_form_data(data: HashMap<String, String>) {
    match get_connection().await.interact(|conn| {     
    let mut columns_map: HashMap<String, String> = HashMap::new();
    for (key, value) in data {
        columns_map.insert(key, value);
    }
    let final_query = format!(
        "INSERT INTO users ({}) VALUES ({});",
        columns_map
            .keys()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(", "),
        columns_map
            .values()
            .map(|_x| "?".to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );
    println!("{}", final_query);
    let mut fir_ret = conn.prepare(&final_query).expect("Failed to prepare query");
    let sec_ret_params: Vec<&dyn ToSql> = columns_map
        .values()
        .map(|x| x as &dyn ToSql)
        .collect();
    let sec_ret = fir_ret
        .execute(sec_ret_params.as_slice());
        match sec_ret {
            Ok(_) => {
                println!("Inserted data");
            }
            Err(_) => {
                println!("Failed to insert data");
            }
        }
}).await
    {
        Ok(_) => {
            println!("Finished");
        }
        Err(_) => {
            println!("Failed to insert data");
        }
    }
}

pub async fn gen_admin_table() {
    let schema = gen_admin_schema().await;
    match get_connection()
        .await
        .interact(move |conn| {
    let fir_ret = conn.prepare("SELECT * FROM users LIMIT 1;");
    match fir_ret {
        Ok(_) => {
            println!("Table already exists");
            return;
        }
        Err(_) => {
            println!("Creating table");
            let parsed_schema: HashMap<String, Field> =
                serde_json::from_str(&schema).expect("Failed to parse schema");
            let mut columns: Vec<String> = Vec::new();
            columns.push("id INTEGER PRIMARY KEY AUTOINCREMENT".to_string());
            for (key, value) in parsed_schema {
                let mut column = format!("{} ", key);
                match value.form_type {
                    HTMLFieldType::Text => {
                        column.push_str("TEXT");
                    }
                    HTMLFieldType::Password => {
                        column.push_str("VARCHAR(255)");
                    }
                    HTMLFieldType::Email => {
                        column.push_str("VARCHAR(255) UNIQUE");
                    }
                }
                if value.required {
                    column.push_str(" NOT NULL");
                }
                columns.push(column);
            }
            let final_query = format!("CREATE TABLE IF NOT EXISTS users ({});", columns.join(", "));
            println!("{}", final_query);
            let mut sec_ret_prep = conn.prepare(&final_query).expect("Failed to prepare query");
            let sec_ret = sec_ret_prep.execute([]).expect("Failed to execute query");
            if sec_ret > 0 {
                println!("Created table");
            } else {
                println!("Failed to create table");
            }
        }
    }
}).await {
        Ok(_) => {
            println!("Created table");
        }
        Err(_) => {
            println!("Failed to create table");
        }
    }
}


pub async fn gen_admin_schema() -> String{
     get_connection().await.interact(|conn| {
    conn.execute("CREATE TABLE IF NOT EXISTS login_schema (id INTEGER PRIMARY KEY AUTOINCREMENT, schema BLOB UNIQUE);", [])
        .expect("Failed to create schema table");
    let mut current_schema = conn.prepare("SELECT * FROM login_schema;").expect("Failed to get schema");
    let row_count = current_schema.column_count();
    if row_count == 0 {
        let mut schema = serde_json::to_string(&DefaultUserSchema::new()).expect("Failed to serialize schema");
        let schema = &mut schema; 
            conn.execute
        ("INSERT INTO login_schema (schema) VALUES (?);" , [schema.to_string()])
            .expect("Failed to insert schema");
        return schema.to_string();
    }
    let mut final_query = current_schema.query([]).expect("Failed to query schema");
    let final_rows = final_query.next();
    match final_rows {

        Ok(row) => {
            match row {
                Some(row) => {
                    let schema = row.get(1).expect("Failed to get schema");
                    return schema;
                }
                None => {
                    let mut schema = serde_json::to_string(&DefaultUserSchema::new()).expect("Failed to serialize schema");
                    let schema = &mut schema; 
                    conn.execute
                ("INSERT INTO login_schema (schema) VALUES (?);" , [schema.to_string()])
                    .expect("Failed to insert schema");
                    return schema.to_string();
                }
            }
        }
        Err(_) => {
            let mut schema = serde_json::to_string(&DefaultUserSchema::new()).expect("Failed to serialize schema");
            let schema = &mut schema; 
            conn.execute
        ("INSERT INTO login_schema (schema) VALUES (?);" , [schema.to_string()])
            .expect("Failed to insert schema");
            return schema.to_string();
        }
    }
}).await.expect("Failed to get schema")
}

pub async fn init_sqlite_db() -> deadpool_sqlite::Pool {
    let cfg = Config::new("./colibase.db");
    cfg.create_pool(Runtime::Tokio1)
        .expect("Failed to create pool")
}

pub async fn get_connection() -> Object {
    init_sqlite_db()
        .await
        .get()
        .await
        .expect("Failed to get connection")
}

pub async fn is_user_initialized() {
    get_connection().await.interact(|conn| {
        let rows_returned = conn
            .execute("SELECT id FROM users LIMIT 1;", [])
            .expect("Failed to prepare query");
          if rows_returned > 0 {
               return true;
           } else {
               return false;
           }
    }).await.expect("Failed to check if user is initialized");
}