use argon2::{
  password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
  Argon2,
};
use futures::StreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, BodyStream, Full};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use multer::Multipart;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

use crate::models::models::{
  gen_admin_schema, gen_admin_table, insert_form_data, query_admin_table, Field, HTMLFieldType,
};

pub struct Server {
  address: SocketAddr,
}

impl Server {
  pub fn new(address: SocketAddr) -> Self {
    Self { address }
  }
  pub async fn handle(
    req: Request<Incoming>,
  ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let method = req.method();
    match method {
      &Method::GET => {
        let path = req.uri().path().split("/").collect::<Vec<&str>>();
        match path[1] {
          "api" => match path[2] {
            "login-schema" => {
              return login_schema(req).await;
            }
            _ => {
              return handle_invalid_path(req).await;
            }
          },
          "ui" => match path[2] {
            "dashboard" => {
              return frontend_ssr_handler(req).await;
            }
            _ => {
              return frontend_ssr_handler(req).await;
            }
          },
          "entry" => match path[2] {
            "login" => {
              return frontend_ssr_handler(req).await;
            }
            "init" => {
              return frontend_ssr_handler(req).await;
            }
            _ => {
              return frontend_ssr_handler(req).await;
            }
          },
          _ => {
            return frontend_ssr_handler(req).await;
          }
        }
      }
      &Method::POST => {
        let path = req.uri().path().split("/").collect::<Vec<&str>>();
        match path[1] {
          "api" => match path[2] {
            _ => {
              return handle_invalid_path(req).await;
            }
          },
          "ui" => match path[2] {
            _ => {
              return frontend_ssr_handler(req).await;
            }
          },
          "entry" => match path[2] {
            "login" => {
              return handle_user_login(&mut req.into()).await;
            }
            "init" => {
              return handle_user_init(&mut req.into()).await;
            }
            _ => {
              return frontend_ssr_handler(req).await;
            }
          },
          _ => {
            return frontend_ssr_handler(req).await;
          }
        }
      }
      _ => {
        return handle_invalid_method(req).await;
      }
    }
  }

  pub async fn start(&self) -> Result<(), hyper::Error> {
    let listener = TcpListener::bind(&self.address)
      .await
      .expect("Failed to bind address");
    println!("Listening on {}", self.address);
    while let Ok((stream, _)) = listener.accept().await {
      let io = TokioIo::new(stream);
      tokio::task::spawn(async move {
        http1::Builder::new()
          .serve_connection(
            io,
            service_fn(move |req| {
              Ok::<_, hyper::Error>(Self::handle(req)).expect("Failed to handle request")
            }),
          )
          .await
          .expect("Failed to serve connection")
      })
      .await
      .expect("Failed to spawn task")
    }
    Ok(())
  }
}

async fn handle_user_login(
  req: &mut Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  let boundary = req
    .headers()
    .get(CONTENT_TYPE)
    .and_then(|ct| ct.to_str().ok())
    .and_then(|ct| multer::parse_boundary(ct).ok());

  if boundary.is_none() {
    return Ok(
      Response::builder()
        .status(hyper::StatusCode::BAD_REQUEST)
        .body(
          Full::new("Invalid boundary".into())
            .map_err(|e| match e {})
            .boxed(),
        )
        .expect("Failed to build response"),
    );
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
    return Ok(
      Response::builder()
        .status(hyper::StatusCode::BAD_REQUEST)
        .body(
          Full::new(err_vec.join("\n").into())
            .map_err(|e| match e {})
            .boxed(),
        )
        .expect("Failed to build response"),
    );
  }

  if !form_data.is_empty() {
    println!("Form data: {:?}", form_data);
    query_admin_table(form_data).await;
  }

  Ok(
    Response::builder()
      .status(200)
      .body(
        Full::new("Hello, World!".into())
          .map_err(|e| match e {})
          .boxed(),
      )
      .expect("Failed to build response"),
  )
}

async fn handle_invalid_path(
  _: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  Ok(
    Response::builder()
      .status(hyper::StatusCode::NOT_FOUND)
      .body(
        Full::new("Invalid path".into())
          .map_err(|e| match e {})
          .boxed(),
      )
      .expect("Failed to build response"),
  )
}

async fn handle_invalid_method(
  _: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  Ok(
    Response::builder()
      .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
      .body(
        Full::new("Invalid method".into())
          .map_err(|e| match e {})
          .boxed(),
      )
      .expect("Failed to build response"),
  )
}

async fn login_schema(
  _: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  let schema = gen_admin_schema().await;
  Ok(
    Response::builder()
      .status(200)
      .body(Full::new(schema.into()).map_err(|e| match e {}).boxed())
      .expect("Failed to build response"),
  )
}

async fn handle_user_init(
  req: &mut Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  let boundary = req
    .headers()
    .get(CONTENT_TYPE)
    .and_then(|ct| ct.to_str().ok())
    .and_then(|ct| multer::parse_boundary(ct).ok());

  if boundary.is_none() {
    return Ok(
      Response::builder()
        .status(hyper::StatusCode::BAD_REQUEST)
        .body(
          Full::new("Invalid boundary".into())
            .map_err(|e| match e {})
            .boxed(),
        )
        .expect("Failed to build response"),
    );
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
    return Ok(
      Response::builder()
        .status(hyper::StatusCode::BAD_REQUEST)
        .body(
          Full::new(err_vec.join("\n").into())
            .map_err(|e| match e {})
            .boxed(),
        )
        .expect("Failed to build response"),
    );
  }

  if !form_data.is_empty() {
    println!("Form data: {:?}", form_data);
  }
  gen_admin_table().await;
  insert_form_data(form_data).await;
  Ok(
    Response::builder()
      .status(200)
      .body(
        Full::new("Hello, World!".into())
          .map_err(|e| match e {})
          .boxed(),
      )
      .expect("Failed to build response"),
  )
}

async fn frontend_ssr_handler(
  req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  match std::env::var("MODE") {
    Ok(mode) => match mode.as_str() {
      "DEV" => Ok(
        dev_server_handler(req)
          .await
          .expect("Failed to handle request"),
      ),
      "PROD" => Ok(
        prod_server_handler(req)
          .await
          .expect("Failed to handle request"),
      ),
      _ => Ok(
        no_mode_handler(req)
          .await
          .expect("Failed to handle request"),
      ),
    },
    Err(_) => Ok(
      no_mode_handler(req)
        .await
        .expect("Failed to handle request"),
    ),
  }
}
async fn dev_server_handler(
  main_req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  let dev_server_url = format!(
    "http://localhost:{}{}",
    env::var("DEV_PORT").expect("Failed to get dev port"),
    main_req.uri().path()
  );
  let url = dev_server_url
    .parse::<hyper::Uri>()
    .expect("Failed to parse dev server url");
  let host = url.host().expect("uri has no host");
  let port = url.port_u16().unwrap_or(80);
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
  let resp = sender
    .send_request(main_req)
    .await
    .expect("Failed to send request to dev server");
  Ok(resp.map(|body| body.boxed()))
}

async fn prod_server_handler(
  main_req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  let dev_server_url = format!(
    "http://localhost:{}{}",
    env::var("PROD_PORT").expect("Failed to get PROD port"),
    main_req.uri().path()
  );
  let url = dev_server_url
    .parse::<hyper::Uri>()
    .expect("Failed to parse prod server url");
  let host = url.host().expect("uri has no host");
  let port = url.port_u16().unwrap_or(80);
  let stream = TcpStream::connect((host, port))
    .await
    .expect("Failed to connect to prod server");
  let io = TokioIo::new(stream);
  let (mut sender, conn) = Builder::new()
    .preserve_header_case(true)
    .title_case_headers(true)
    .handshake(io)
    .await
    .expect("Failed to handshake with prod server");
  tokio::task::spawn(async move {
    if let Err(err) = conn.await {
      println!("Error serving connection: {:?}", err);
    }
  });
  let resp = sender
    .send_request(main_req)
    .await
    .expect("Failed to send request to prod server");
  Ok(resp.map(|body| body.boxed()))
}

async fn no_mode_handler(
  _: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
  Ok(
    Response::builder()
      .status(200)
      .body(
        Full::new("Hello, World!".into())
          .map_err(|e| match e {})
          .boxed(),
      )
      .expect("Failed to build response"),
  )
}
