use axum::{
  body::Body,
  extract::Request,
  response::{IntoResponse, Response},
  routing::get,
  Router,
};
use futures::{SinkExt, StreamExt};
use hyper::client::conn::http1::Builder;
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use std::env;
use std::sync::Arc;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::connect_async;

pub async fn main_server() {
  let addr = "0.0.0.0:3006";
  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  let app = Router::new()
    .route("/", get(frontend_ssr_handler))
    .route("/*wildcard", get(frontend_ssr_handler));
  println!("Listening on http://{}", addr);
  axum::serve(listener, app).await.unwrap();
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

  if std::env::var("MODE").expect("Failed to get mode") == "DEV" {
    if hyper_tungstenite::is_upgrade_request(&main_req) {
      if let Ok((response, websocket)) = hyper_tungstenite::upgrade(&mut main_req, None) {
        tokio::task::spawn(async move {
          if let Err(err) = serve_proxy_ws(websocket, main_req).await {
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
async fn serve_proxy_ws(ws: HyperWebsocket, req: Request<Body>) -> Result<(), Error> {
  let websocket = Arc::new(Mutex::new(ws.await.expect("Failed to get websocket")));
  let (wss, _) = connect_async(format!(
    "ws://localhost:{}{}",
    env::var("DEV_PORT").expect("Failed to get dev server port"),
    req.uri().path()
  ))
  .await
  .expect("Failed to connect");

  let ws_stream = Arc::new(Mutex::new(wss));

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
