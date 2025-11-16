#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

#[derive(FromFormField)]
enum Lang {
    #[field(value = "en")]
    English,
    #[field(value = "ru")]
    #[field(value = "ру")]
    Russian
}

#[derive(FromForm)]
struct Options<'r> {
    emoji: bool,
    name: Option<&'r str>,
}

use axum::{
    body::Body,
    extract::{Path, Query},
    http::Error,
    http::{Response, StatusCode},
    response::Redirect,
    routing::get,
    Router, ServiceExt,
};
use serde::Serialize;
use std::collections::HashMap;
mod search;
use flipkart_scraper::{search::SearchParams, Url};
use search::search_product;
mod product;
use axum::response::IntoResponse;
use product::product_details;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct ApiError {
    error_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    more_details: Option<String>,
}

fn default_error_response(e: Error) -> Response<Body> {
    let err = ApiError {
        error_message: "Internal Server Error".to_string(),
        more_details: Some(format!("There was some internal server error, make sure you are calling the API correctly. {e}. Report any issues at https://github.com/dvishal485/flipkart-scraper-api")),
    };

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"error": err}).to_string()))
        .unwrap_or_default()
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap_or_default()
}

fn internal_error<E: std::fmt::Display>(e: E) -> Response {
    let err = ApiError {
        error_message: "Internal Server Error".into(),
        more_details: Some(format!(
            "There was some internal server error. {e}. \
            Report issues at https://github.com/dvishal485/flipkart-scraper-api"
        )),
    };
    json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": err}))
}
/*
pub async fn search_router(
    query: Option<Path<String>>,
    params: Result<Query<SearchParams>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let q = query.map(|v| v.to_string()).unwrap_or_default();

    match params {
        Err(e) => {
            let err = ApiError {
                error_message: "Invalid query parameters".into(),
                more_details: Some(e.to_string()),
            };
            return json_response(StatusCode::BAD_REQUEST, json!({"error": err}));
        }
        Ok(Query(params)) => {
            match search_product(q, params).await {
                Ok(data) => match serde_json::to_value(&data) {
                    Ok(v) => json_response(StatusCode::OK, v),
                    Err(e) => internal_error(e),
                },
                Err(e) => json_response(
                    StatusCode::BAD_GATEWAY,
                    json!({"error": e.to_string()}),
                ),
            }
        }
    }
}

pub async fn product_router(
    Path(url): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Response {
    let url = Url::parse_with_params(
        &format!("https://www.flipkart.com/{url}"),
        query_params,
    );

    let Ok(parsed) = url else {
        return json_response(
            StatusCode::BAD_GATEWAY,
            json!({"error": "Invalid URL"}),
        );
    };

    match product_details(parsed).await {
        Ok(data) => match serde_json::to_value(&data) {
            Ok(v) => json_response(StatusCode::OK, v),
            Err(e) => internal_error(e),
        },
        Err(e) => json_response(StatusCode::BAD_GATEWAY, json!({"error": e})),
    }
}
*/
#[get("/search?<params..>")]
async fn search_no_query(params: Option<Form<SearchParams>>) -> Json<Value> {
    search_router(None, params).await

}
#[get("/search/<query..>?<params..>")]
async fn search_with_query(
    query: RocketPathBuf,
    params: Option<Form<SearchParams>>,
) -> Json<Value> {
    let q = query.to_string_lossy().to_string();
    search_router(Some(q), params).await
}

#[get("/product/<url..>?<params..>")]
async fn product_route(
    url: RocketPathBuf,
    params: Option<HashMap<String, String>>,
) -> Json<Value> {
    let full_url = format!("https://www.flipkart.com/{}", url.to_string_lossy());

    // Parse URL with query params
    let parsed = Url::parse_with_params(&full_url, params.unwrap_or_default());

    match parsed {
        Ok(actual_url) => match product_details(actual_url).await {
            Ok(data) => Json(json!(data)),
            Err(e) => Json(json!({ "error": e })),
        },
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/*
async fn search_router(
    query: Option<Path<String>>,
    params_result: Result<Query<SearchParams>, axum::extract::rejection::QueryRejection>,
) -> Response<Body> {
    match params_result {
        Ok(Query(params)) => {
            let query = query.map(|q| q.to_string()).unwrap_or_default();
            let data = search_product(query, params).await;

            match data {
                Ok(data) => Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&data).unwrap())),
                Err(err) => Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({"error": err}).to_string())),
            }
        }
        Err(err) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!(ApiError {
                    error_message: "Invalid query parameters".to_string(),
                    more_details: Some(err.to_string()),
                })
                .to_string(),
            )),
    }
    .unwrap_or_else(|e| default_error_response(e))
}

async fn product_router(
    Path(url): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Response<Body> {
    let url = Url::parse_with_params(
        format!("https://www.flipkart.com/{url}").as_str(),
        query_params,
    );

    match url {
        Ok(url) => {
            let data = product_details(url).await;

            match data {
                Err(e) => Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({"error": e}).to_string())),
                Ok(data) => Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&data).unwrap())),
            }
        }
        Err(e) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("Content-Type", "application/json")
            .body(Body::from(json!({"error": e.to_string()}).to_string())),
    }
    .unwrap_or_else(|e| default_error_response(e))
}
*/
const DEFAULT_DEPLOYMENT_URL: &str = "https://0.0.0.0:10000";

// Try visiting:
//   http://127.0.0.1:8000/hello/world
#[get("/world")]
fn world() -> &'static str {
    "Hello, world!"
}

// Try visiting:
//   http://127.0.0.1:8000/hello/мир
#[get("/мир")]
fn mir() -> &'static str {
    "Привет, мир!"
}

// Try visiting:
//   http://127.0.0.1:8000/wave/Rocketeer/100
#[get("/<name>/<age>", rank = 2)]
fn wave(name: &str, age: u8) -> String {
    format!("👋 Hello, {} year old named {}!", age, name)
}


// Note: without the `..` in `opt..`, we'd need to pass `opt.emoji`, `opt.name`.
//
// Try visiting:
//   http://127.0.0.1:8000/?emoji
//   http://127.0.0.1:8000/?name=Rocketeer
//   http://127.0.0.1:8000/?lang=ру
//   http://127.0.0.1:8000/?lang=ру&emoji
//   http://127.0.0.1:8000/?emoji&lang=en
//   http://127.0.0.1:8000/?name=Rocketeer&lang=en
//   http://127.0.0.1:8000/?emoji&name=Rocketeer
//   http://127.0.0.1:8000/?name=Rocketeer&lang=en&emoji
//   http://127.0.0.1:8000/?lang=ru&emoji&name=Rocketeer
#[get("/?<lang>&<opt..>")]
fn hello(lang: Option<Lang>, opt: Options<'_>) -> String {
    let mut greeting = String::new();
    if opt.emoji {
        greeting.push_str("👋 ");
    }

    match lang {
        Some(Lang::Russian) => greeting.push_str("Привет"),
        Some(Lang::English) => greeting.push_str("Hello"),
        None => greeting.push_str("Hi"),
    }

    if let Some(name) = opt.name {
        greeting.push_str(", ");
        greeting.push_str(name);
    }

    greeting.push('!');
    greeting
}

#[get("/")]
fn index() -> (Status, (rocket::http::ContentType, String)) {
     let deploy_url =
        std::env::var("DEPLOYMENT_URL").unwrap_or_else(|_| DEFAULT_DEPLOYMENT_URL.to_string());

    let description: Value = json!({
        "name": env!("CARGO_PKG_NAME"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "version": env!("CARGO_PKG_VERSION"),
        "authors": env!("CARGO_PKG_AUTHORS"),
        "repository": env!("CARGO_PKG_REPOSITORY"),
        "license": env!("CARGO_PKG_LICENSE"),
        "usage": {
            "search_api": format!("{deploy_url}/search/{{product_name}}"),
            "product_api": format!("{deploy_url}/product/{{product_link_argument}}"),
        }
    });

    let body = description.to_string(); // Replace with your `description`
    (
        Status::Ok,
        (rocket::http::ContentType::JSON, body)
    )
}

#[get("/search")]
#[get("/search/")]
fn search_root() -> Json<SearchResponse> {
    search_router(None)
}
/*
#[get("/search/<query..>")]
fn search_handler(query: std::path::PathBuf) -> Json<SearchResponse> {
    let q = query.to_string_lossy().to_string();
    search_router(Some(q))
}
 
#[get("/search?<query..>")]
fn search_router(query: Option<String>) -> Json<SearchResponse>

#[get("/product/<url..>?<params..>")]
fn product_router(url: PathBuf, params: Form<HashMap<String, String>>) -> Json<ProductResponse>

#[get("/product/<url..>")]
fn product_handler(url: std::path::PathBuf) -> Json<ProductResponse> {
    let product_url = url.to_string_lossy().to_string();
    product_router(product_url)
}
*/
/// Fallback catch-all route
#[catch(404)]
fn not_found() -> Redirect {
    Redirect::permanent("/")
}


#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let figment = rocket::Config::figment()
        .merge(("port", port))
        .merge(("address", "0.0.0.0"));

    rocket::custom(figment)
        .mount("/", routes![hello, index])
        .mount("/hello", routes![world, mir])
        .mount("/wave", routes![wave])
        .mount("/", routes![
	        search_root,
	        search_handler,
	        product_handler
        ])
        .register("/", catchers![not_found])
        .launch()
        .await?;

    Ok(())
}
/*
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![hello])
        .mount("/hello", routes![world, mir])
        .mount("/wave", routes![wave])
}

#[tokio::main]
async fn main() {
    let deploy_url =
        std::env::var("DEPLOYMENT_URL").unwrap_or_else(|_| DEFAULT_DEPLOYMENT_URL.to_string());

    let description: Value = json!({
        "name": env!("CARGO_PKG_NAME"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "version": env!("CARGO_PKG_VERSION"),
        "authors": env!("CARGO_PKG_AUTHORS"),
        "repository": env!("CARGO_PKG_REPOSITORY"),
        "license": env!("CARGO_PKG_LICENSE"),
        "usage": {
            "search_api": format!("{deploy_url}/search/{{product_name}}"),
            "product_api": format!("{deploy_url}/product/{{product_link_argument}}"),
        }
    });

    let app = Router::new()
        .route(
            "/",
            get(|| async move {
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Body::from((description).to_string()))
                    .unwrap()
            }),
        )
        .route("/search/{*query}", get(search_router))
        .route("/search", get(search_router))
        .route("/search/", get(search_router))
        .route("/product/{*url}", get(product_router))
        .fallback(get(|| async {
            (StatusCode::PERMANENT_REDIRECT, Redirect::permanent("/")).into_response()
        }));

    println!("Starting server on {}", deploy_url);

    let listener = tokio::net::TcpListener::bind(deploy_url).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
*/
