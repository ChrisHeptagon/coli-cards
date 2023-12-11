pub mod models;
pub mod server;
use dotenvy::dotenv;
use server::server::main_server;
use std::env;

#[tokio::main]
async fn main() {
  dotenv().expect("Failed to read .env file");
  // println!("Starting node server");
  // match tokio::process::Command::new("node")
  //   .arg(format!(
  //     "{}{}",
  //     env::current_dir()
  //       .expect("Failed to get current directory")
  //       .to_str()
  //       .expect("Failed to convert current directory to string"),
  //     "/admin-ui/dist/server/entry.mjs"
  //   ))
  //   .envs(env::vars())
  //   .spawn()
  // {
  //   Ok(_) => println!("Node server started"),
  //   Err(e) => println!("Failed to start node server: {}", e),
  // }
  main_server().await
}
