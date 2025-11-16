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


// Imports to use instead of axum ones
use rocket::{get, launch, routes, catch, catchers, Build, Rocket};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::tokio;
use rocket::http::uri::PathBuf as RocketPathBuf;



use serde::Serialize;
use std::collections::HashMap;
mod search;
use flipkart_scraper::{search::SearchParams, Url};
use search::search_product;
mod product;
use axum::response::IntoResponse;
use product::product_details;
use serde_json::json;



// Helper alias for responses we return from Rocket handlers
type RJson = (Status, Json<Value>);

// Convenience constructors
fn json_response(status: Status, value: Value) -> RJson {
    (status, Json(value))
}

fn internal_error<E: std::fmt::Display>(e: E) -> RJson {
    let err = ApiError {
        error_message: "Internal Server Error".into(),
        more_details: Some(format!(
            "There was some internal server error. {e}. \
            Report issues at https://github.com/dvishal485/flipkart-scraper-api"
        )),
    };
    json_response(Status::InternalServerError, json!({ "error": err }))
}

// INDEX route (equivalent to your "/" route)
#[get("/")]
fn index() -> RJson {
    json_response(Status::Ok, json!({ "message": "Flipkart Scraper API (Rocket)" }))
}

// ---------- SEARCH routes ----------
// Two Rocket routes: /search (no path) and /search/<query..>
//
// We accept query params as Option<HashMap<String,String>> for flexibility.
// If you have a `SearchParams` struct that implements FromForm, you can change this.
#[get("/search?<params..>")]
async fn search_root(params: Option<HashMap<String, String>>) -> RJson {
    // Convert query params map to your SearchParams if needed. Here we forward as map.
    // If your existing `search_product` expects a typed SearchParams struct, adapt here.
    let query = None::<String>;
    perform_search(query, params).await
}

#[get("/search/<query..>?<params..>")]
async fn search_with_query(query: RocketPathBuf, params: Option<HashMap<String, String>>) -> RJson {
    let q = query.to_string_lossy().to_string();
    perform_search(Some(q), params).await
}

// Shared logic extracted from your old `search_router`
async fn perform_search(query: Option<String>, params_map: Option<HashMap<String, String>>) -> RJson {
    // Convert params_map to your SearchParams type if you have one.
    // For compatibility, I'll assume your search_product accepts (String, SearchParams).
    // If it accepts a HashMap, call it directly. Adapt as needed.
    //
    // Example: if you have SearchParams and From<HashMap> impl:
    // let params = params_map.map(|m| SearchParams::from(m)).unwrap_or_default();

    // For now, we'll attempt to call a function `search_product(query, params_map)`:
    // If your actual function signature differs, change the call accordingly.

    let q = query.unwrap_or_default();

    // ------ Call your existing async function here ------
    // Replace the following line with the exact call to your actual search_product:
    // let data_res = search_product(q, params_struct).await;
    //
    // I'll call a placeholder `search_product_map` that should represent your real function:
    let data_res: Result<serde_json::Value, String> =
        match search_product(q.clone(), params_map.clone()).await {
            Ok(v) => Ok(serde_json::to_value(&v).unwrap_or(json!({"error": "serialize_fail"}))),
            Err(e) => Err(e.to_string()),
        };

    match data_res {
        Ok(v) => json_response(Status::Ok, v),
        Err(err) => json_response(Status::BadGateway, json!({ "error": err })),
    }
}

// ---------- PRODUCT route ----------
#[get("/product/<url..>?<params..>")]
async fn product_route(url: RocketPathBuf, params: Option<HashMap<String, String>>) -> RJson {
    let full = format!("https://www.flipkart.com/{}", url.to_string_lossy());

    // parse with params (if any)
    let parsed = Url::parse_with_params(&full, params.unwrap_or_default());
    let parsed = match parsed {
        Ok(u) => u,
        Err(e) => return json_response(Status::BadRequest, json!({"error": e.to_string()})),
    };

    // Call your product_details(parsed).await
    match product_details(parsed).await {
        Ok(data) => match serde_json::to_value(&data) {
            Ok(v) => json_response(Status::Ok, v),
            Err(e) => internal_error(e),
        },
        Err(e) => json_response(Status::BadGateway, json!({ "error": e })),
    }
}

// ---------- Catcher for 404 -> redirect to "/" ----------
#[catch(404)]
fn not_found() -> Redirect {
    Redirect::permanent("/")
}


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
                search_with_query,
                product_route
        ])
        .register("/", catchers![not_found])
        .launch()
        .await?;

    Ok(())
}

