pub mod models;
pub mod server;
use dotenvy::dotenv;
use server::server::Server;
use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to read .env file");
    println!("Starting node server");
    tokio::process::Command::new("node")
        .arg(format!(
            "{}{}",
            env::current_dir()
                .expect("Failed to get current directory")
                .to_str()
                .expect("Failed to convert current directory to string"),
            "/admin-ui/dist/server/entry.mjs"
        ))
        .envs(env::vars())
        .spawn()
        .expect("Failed to start node server");
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let server = Server::new(addr);
    match server.start().await {
        Ok(_) => {
            println!("Server started");
        }
        Err(_) => {
            println!("Failed to start server");
        }
    }
}
