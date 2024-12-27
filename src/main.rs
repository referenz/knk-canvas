use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::{env, fs};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

mod create_image;
use create_image::create_haiku_image;

mod files;
use files::save_image_to_directory;

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
            let image_data = buffer.into_inner();

            // Bild speichern, wenn die Umgebungsvariable gesetzt ist
            if let Ok(save_enabled) = env::var("SAVE_IMAGES") {
                if save_enabled == "true" {
                    if let Err(e) = save_image_to_directory(image_data.clone(), haiku) {
                        eprintln!("Fehler beim Speichern des Bildes: {}", e);
                    }
                }
            }

            // Bild per HTTP zurückgeben
            ([("Content-Type", "image/webp")], image_data).into_response()
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

async fn get_json() -> Json<Value> {
    // Pfad zur JSON-Datei
    let file_path = "haikus/images.json";

    // Datei einlesen
    match fs::read_to_string(file_path) {
        Ok(content) => {
            // Inhalt in JSON umwandeln
            match serde_json::from_str::<Value>(&content) {
                Ok(json_data) => Json(json_data),
                Err(_) => Json(json!({ "error": "Ungültiges JSON-Format in Datei" })),
            }
        }
        Err(_) => Json(json!({ "error": "Datei konnte nicht gelesen werden" })),
    }
}

async fn serve_images() -> Html<String> {
    let template: &str = include_str!("../assets/index.html");

    let addr = env::var("SERVER_ADDR").expect("Server-IP und -Port müssen in .env definiert sein!");
    let html_content = template.replace("{{ addr }}", &addr);

    Html(html_content)
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // .env-Datei laden

    let server_addr =
        env::var("SERVER_ADDR").expect("Server-IP und -Port müssen in .env definiert sein!");
    let api_key = env::var("API_KEY").expect("API_KEY muss in .env definiert sein!");

    // Router erstellen
    let haiku_router = Router::new()
        .route("/haiku", post(serve_haiku_image))
        .layer(middleware::from_fn(move |req, next| {
            api_key_middleware(req, next, api_key.clone())
        }));

    let general_router = Router::new()
        .route("/haiku/json", get(get_json))
        .route("/haiku/images", get(serve_images))
        .nest_service("/haiku/files", ServeDir::new("haikus")); // Bilder aus dem Verzeichnis "haikus" bereitstellen

    let app = haiku_router.merge(general_router);

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
