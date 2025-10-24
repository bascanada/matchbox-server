use axum::{routing::get, Router, http::StatusCode, response::IntoResponse};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

mod files;
mod manifests;

#[derive(Clone)]
struct AppState {
    db_pool: sqlx::SqlitePool,
    storage_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Get environment variables
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let storage_path = std::env::var("STORAGE_PATH").expect("STORAGE_PATH must be set");

    // Set up database connection pool
    let db_pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Run database migrations
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    info!("Database migrations complete.");

    // Create the application state
    let app_state = AppState {
        db_pool,
        storage_path: PathBuf::from(storage_path),
    };

    // build our application with a route
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/files/:sha256", get(files::serve_file))
        .route(
            "/games/:game_slug/versions/:version/manifest",
            get(manifests::serve_manifest),
        )
        .with_state(app_state);

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
