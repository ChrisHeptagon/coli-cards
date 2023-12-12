use argon2::{
  password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
  Argon2,
};
use axum::{
  body::Body,
  extract::{ws::WebSocket, Request, WebSocketUpgrade},
  http::uri::Scheme,
  response::{IntoResponse, Response},
  routing::get,
  Router,
};
use futures::{SinkExt, StreamExt};
use http_body_util::BodyStream;
use hyper::client::conn::http1::Builder;
use hyper::header::CONTENT_TYPE;
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use multer::Multipart;
use std::env;
use std::{collections::HashMap, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::connect_async;

use crate::models::models::{
  gen_admin_schema, gen_admin_table, insert_form_data, query_admin_table, Field, HTMLFieldType,
};

pub async fn main_server() {
  let addr = "0.0.0.0:3006";
  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  let app = Router::new()
    .route("/", get(frontend_ssr_handler))
    .route("/*wildcard", get(frontend_ssr_handler));
  println!("Listening on http://{}", addr);
  axum::serve(listener, app).await.unwrap();
}

async fn handle_user_login(req: &mut Request<Body>) -> Response {
  let boundary = req
    .headers()
    .get(CONTENT_TYPE)
    .and_then(|ct| ct.to_str().ok())
    .and_then(|ct| multer::parse_boundary(ct).ok());

  if boundary.is_none() {
    return Response::builder()
      .status(hyper::StatusCode::BAD_REQUEST)
      .body(Body::from("Invalid boundary"))
      .expect("Failed to build response");
  };

  let body_stream = BodyStream::new(req.body_mut())
    .filter_map(|result| async move { result.map(|frame| frame.into_data().ok()).transpose() });

  let mut multipart = Multipart::new(body_stream, boundary.expect("Failed to get boundary"));
  let schema: String = gen_admin_schema().await;
  let parsed_schema: HashMap<String, Field> =
    serde_json::from_str(&schema).expect("Failed to parse schema");
  let mut err_vec: Vec<String> = Vec::new();
  let mut form_data: HashMap<String, String> = HashMap::new();

  while let Some(field) = multipart
    .next_field()
    .await
    .expect("Failed to get next field")
  {
    let name = field.name().expect("Failed to get field name").to_string();
    match parsed_schema.get(&name.to_string()) {
      Some(schema_field) => match schema_field.required {
        true => {
          let value = field
            .text()
            .await
            .expect("Failed to get field value")
            .to_string();
          match value.is_empty() {
            true => {
              err_vec.push(format!("{} is required", name));
            }
            false => {
              let field_regex =
                regex::Regex::new(&schema_field.pattern).expect("Failed to parse field regex");
              match field_regex.is_match(&value) {
                true => match schema_field.form_type {
                  HTMLFieldType::Text => {
                    form_data.insert(name, value);
                  }
                  HTMLFieldType::Email => {
                    form_data.insert(name, value);
                  }
                  HTMLFieldType::Password => {
                    let password = &value.into_bytes();
                    let salt = SaltString::generate(&mut OsRng);
                    let argon2 = Argon2::default();
                    let hash = argon2
                      .hash_password(password, &salt)
                      .expect("Failed to hash password")
                      .to_string();
                    println!("Hash: {}", hash);
                    form_data.insert(name, hash);
                  }
                },
                false => {
                  err_vec.push(format!("{} is not valid", name));
                }
              }
            }
          }
        }
        false => {
          continue;
        }
      },
      None => {
        err_vec.push(format!("{} is not a valid field", name));
      }
    }
  }
  if !err_vec.is_empty() {
    return Response::builder()
      .status(hyper::StatusCode::BAD_REQUEST)
      .body(Body::from(err_vec.join("\n")))
      .expect("Failed to build response");
  };

  if !form_data.is_empty() {
    println!("Form data: {:?}", form_data);
    query_admin_table(form_data).await;
  }

  Response::builder()
    .status(200)
    .body(Body::from("Hello, World!"))
    .expect("Failed to build response")
}

async fn handle_invalid_path() -> Response {
  Response::builder()
    .status(hyper::StatusCode::NOT_FOUND)
    .body(Body::from("Invalid path"))
    .expect("Failed to build response")
}

async fn handle_invalid_method() -> Response {
  Response::builder()
    .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
    .body(Body::from("Invalid method"))
    .expect("Failed to build response")
}

async fn login_schema() -> Response {
  let schema = gen_admin_schema().await;
  Response::builder()
    .status(200)
    .body(Body::new(schema))
    .expect("Failed to build response")
}

async fn handle_user_init(req: &mut Request) -> Response {
  let boundary = req
    .headers()
    .get(CONTENT_TYPE)
    .and_then(|ct| ct.to_str().ok())
    .and_then(|ct| multer::parse_boundary(ct).ok());

  if boundary.is_none() {
    return Response::builder()
      .status(hyper::StatusCode::BAD_REQUEST)
      .body(Body::from("Invalid boundary"))
      .expect("Failed to build response");
  }

  let body_stream = BodyStream::new(req.body_mut())
    .filter_map(|result| async move { result.map(|frame| frame.into_data().ok()).transpose() });

  let mut multipart = Multipart::new(body_stream, boundary.expect("Failed to get boundary"));
  let schema: String = gen_admin_schema().await;
  let parsed_schema: HashMap<String, Field> =
    serde_json::from_str(&schema).expect("Failed to parse schema");
  let mut err_vec: Vec<String> = Vec::new();
  let mut form_data: HashMap<String, String> = HashMap::new();

  while let Some(field) = multipart
    .next_field()
    .await
    .expect("Failed to get next field")
  {
    let name = field.name().expect("Failed to get field name").to_string();
    match parsed_schema.get(&name.to_string()) {
      Some(schema_field) => match schema_field.required {
        true => {
          let value = field
            .text()
            .await
            .expect("Failed to get field value")
            .to_string();
          match value.is_empty() {
            true => {
              err_vec.push(format!("{} is required", name));
            }
            false => {
              let field_regex =
                regex::Regex::new(&schema_field.pattern).expect("Failed to parse field regex");
              match field_regex.is_match(&value) {
                true => match schema_field.form_type {
                  HTMLFieldType::Text => {
                    form_data.insert(name, value);
                  }
                  HTMLFieldType::Email => {
                    form_data.insert(name, value);
                  }
                  HTMLFieldType::Password => {
                    let password = &value.into_bytes();
                    let salt = SaltString::generate(&mut OsRng);
                    let argon2 = Argon2::default();
                    let hash = argon2
                      .hash_password(password, &salt)
                      .expect("Failed to hash password")
                      .to_string();
                    println!("Hash: {}", hash);
                    form_data.insert(name, hash);
                  }
                },
                false => {
                  err_vec.push(format!("{} is not valid", name));
                }
              }
            }
          }
        }
        false => {
          continue;
        }
      },
      None => {
        err_vec.push(format!("{} is not a valid field", name));
      }
    }
  }
  if !err_vec.is_empty() {
    return Response::builder()
      .status(hyper::StatusCode::BAD_REQUEST)
      .body(Body::from(err_vec.join("\n")))
      .expect("Failed to build response");
  }

  if !form_data.is_empty() {
    println!("Form data: {:?}", form_data);
  }
  gen_admin_table().await;
  insert_form_data(form_data).await;
  Response::builder()
    .status(200)
    .body(Body::from("Hello, World!"))
    .expect("Failed to build response")
}

async fn frontend_ssr_handler(request: Request<Body>) -> impl IntoResponse {
  let dev_port = env::var("DEV_PORT")
    .expect("Failed to get dev server port")
    .parse::<u16>()
    .expect("Failed to parse dev server port");
  let prod_port = env::var("PROD_PORT")
    .expect("Failed to get prod server port")
    .parse::<u16>()
    .expect("Failed to parse prod server port");
  match std::env::var("MODE") {
    Ok(mode) => match mode.as_str() {
      "DEV" => proxy_handler(request, dev_port).await.into_response(),
      "PROD" => proxy_handler(request, prod_port).await.into_response(),
      _ => no_mode_handler(request).await.into_response(),
    },
    Err(_) => no_mode_handler(request).await.into_response(),
  }
}

async fn proxy_handler(mut main_req: Request<Body>, port: u16) -> impl IntoResponse {
  let dev_server_url = format!("http://localhost:{}{}", port, main_req.uri().path());
  println!("Proxying to {}", dev_server_url);
  let url = url::Url::parse(&dev_server_url).expect("Failed to parse dev server url");
  let host = url.host_str().expect("uri has no host");
  let port = url.port().expect("uri has no port");

  let stream = TcpStream::connect((host, port))
    .await
    .expect("Failed to connect to dev server");
  let io = TokioIo::new(stream);
  let (mut sender, conn) = Builder::new()
    .preserve_header_case(true)
    .title_case_headers(true)
    .handshake(io)
    .await
    .expect("Failed to handshake with dev server");
  tokio::task::spawn(async move {
    if let Err(err) = conn.await {
      println!("Error serving connection: {:?}", err);
    }
  });

  if std::env::var("MODE").expect("Failed to get mode") == "DEV"
    && main_req.uri().to_string() == "/_next/webpack-hmr"
  {
    println!("HMR request");
    if hyper_tungstenite::is_upgrade_request(&main_req) {
      if let Ok((response, websocket)) = hyper_tungstenite::upgrade(&mut main_req, None) {
        tokio::task::spawn(async move {
          if let Err(err) = serve_proxy_ws(websocket).await {
            println!("Error serving websocket: {:?}", err);
          }
        });
        return response.into_response();
      }
    }
  };
  let resp = sender
    .send_request(main_req)
    .await
    .expect("Failed to send request to dev server");
  resp.into_response()
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
async fn serve_proxy_ws(ws: HyperWebsocket) -> Result<(), Error> {
  let mut websocket = Arc::new(Mutex::new(ws.await.expect("Failed to get websocket")));
  let (wss, _) = connect_async(format!(
    "ws://localhost:{}/_next/webpack-hmr",
    env::var("DEV_PORT").expect("Failed to get dev server port")
  ))
  .await
  .expect("Failed to connect");

  let mut ws_stream = Arc::new(Mutex::new(wss));

  while let Some(msg) = {
    let mut websocket = websocket.lock().await;
    websocket.next().await
  } {
    let mut websocket = websocket.lock().await;
    let mut ws_stream = ws_stream.lock().await;
    let msg = msg.expect("Failed to get message");
    ws_stream.send(msg).await.expect("Failed to send message");
    let msg = ws_stream.next().await.expect("Failed to get message");
    websocket.send(msg?).await.expect("Failed to send message");
  }

  Ok(())
}

async fn no_mode_handler(_: Request<Body>) -> impl IntoResponse {
  Response::builder()
    .status(hyper::StatusCode::NOT_FOUND)
    .body(Body::from("No mode set"))
    .expect("Failed to build response")
}
