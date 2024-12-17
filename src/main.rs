use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod create_image;
use create_image::create_haiku_image;

#[derive(Deserialize)]
struct HaikuRequest {
    text: String,
}

async fn api_key_middleware(req: Request<Body>, next: Next, api_key: String) -> Response {
    // API-Key validieren
    let key = req
        .headers()
        .get("x-api-key")
        .and_then(|key| key.to_str().ok());

    if key == Some(&api_key) {
        // API-Key korrekt -> Request weiterleiten
        return next.run(req).await;
    }

    // Fehlerhafte oder fehlende API-Schlüssel -> Blockieren
    eprintln!(
        "Ungültiger oder fehlender API-Schlüssel: {:?}",
        key.unwrap_or("None")
    );
    (
        StatusCode::UNAUTHORIZED,
        "Unauthorized: Missing or invalid API key",
    )
        .into_response()
}

// HTTP-Handler
async fn serve_haiku_image(Json(payload): Json<HaikuRequest>) -> Response {
    let haiku = &payload.text;

    match create_haiku_image(haiku) {
        Ok(buffer) => {
            let body = buffer.into_inner();
            ([("Content-Type", "image/webp")], body).into_response()
        }
        Err(e) => {
            // Fehlertext aus dem Fehler-Object extrahieren
            let error_message = format!("Failed to generate image: {}", e);
            eprint!("{}", error_message);

            // Fehlerantwort mit Klartext zurückgeben
            ([("Content-Type", "text/plain")], error_message).into_response()
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // .env-Datei laden

    let server_addr =
        env::var("SERVER_ADDR").expect("Server-IP und -Port müssen in .env definiert sein!");
    let api_key = env::var("API_KEY").expect("API_KEY muss in .env definiert sein!");

    // Router erstellen
    let app = Router::new()
        .route("/haiku", post(serve_haiku_image))
        .layer(middleware::from_fn(move |req, next| {
            api_key_middleware(req, next, api_key.clone())
        }));

    // Server starten
    let addr: SocketAddr = server_addr.parse().expect("Ungültige Serveradresse");
    let listener = TcpListener::bind(addr)
        .await
        .expect("Konnte nicht an Adresse binden");
    println!("Server läuft unter http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
