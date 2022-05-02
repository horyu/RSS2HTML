#![feature(let_chains)]

use actix_web::{
    dev, error, get,
    http::header::ContentType,
    http::StatusCode,
    middleware::{ErrorHandlerResponse, ErrorHandlers},
    web, App, HttpResponse, HttpServer, Responder,
};
use derive_more::{Display, Error};
use sailfish::TemplateOnce;
use serde::{Deserialize, Serialize};

const INDEX_HTML: &str = include_str!("../files/index.html");

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .append_header(ContentType::html())
        .body(INDEX_HTML)
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("../static/favicon.svg"))
}

#[derive(Debug, Deserialize)]
pub struct RSSRequest {
    url: String,
}

#[derive(Debug, Display, Error)]
enum MyError {
    #[display(fmt = "NotFoundError: {}", text)]
    NotFound { text: String },
    #[display(fmt = "ReqwestError: {}", text)]
    Reqwest { text: String },
    #[display(fmt = "RssError: {}", text)]
    Rss { text: String },
    #[display(fmt = "RenderError: {}", text)]
    Render { text: String },
}

impl error::ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        let error_text = format! {"<p>{}</p>", self};
        let body = INDEX_HTML.replacen("<!--COMMENT_TO_REPLACE-->", &error_text, 1);
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(body)
    }
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Render { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[get("/rss.stpl")]
async fn rss_stpl(rss_request: web::Query<RSSRequest>) -> actix_web::Result<HttpResponse> {
    let rss_text = get_rss_text(&rss_request.url)
        .await
        .map_err(|e| MyError::Reqwest {
            text: e.to_string(),
        })?;
    let channel = make_rss_channel(&rss_text).map_err(|e| MyError::Rss {
        text: e.to_string(),
    })?;

    let items = channel
        .items()
        .iter()
        .map(|i| Item {
            title: i.title().unwrap_or_default().to_string(),
            link: i.link().unwrap_or_default().to_string(),
            pub_date: extract_date_string(i),
        })
        .collect();

    let body = ItemsWrapper { items }
        .render_once()
        .map_err(|e| MyError::Render {
            text: e.to_string(),
        })?;

    Ok(HttpResponse::Ok()
        .append_header(ContentType::html())
        .body(body))
}

#[derive(sailfish::TemplateOnce)]
#[template(path = "../files/rss.stpl")]
struct ItemsWrapper {
    items: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    title: String,
    link: String,
    pub_date: String,
}

async fn get_rss_text(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url).await?.text().await
}

fn make_rss_channel(rss_text: &str) -> Result<rss::Channel, rss::Error> {
    rss::Channel::read_from(rss_text.as_bytes())
}

fn extract_date_string(item: &rss::Item) -> String {
    if let Some(d) = item.pub_date() {
        return chrono::DateTime::parse_from_rfc2822(d)
            .map(|d| d.format("%F %R").to_string())
            .unwrap_or_else(|_| d.to_string());
    }
    if let Some(x) = item.dublin_core_ext() && let Some(d) = x.dates().first() {
        return chrono::DateTime::parse_from_rfc3339(d)
            .map(|d| d.format("%F %R").to_string())
            .unwrap_or_else(|_| d.to_string());
    }

    String::new()
}

async fn robots_txt() -> impl Responder {
    HttpResponse::Ok()
        .append_header(ContentType::plaintext())
        .body(include_str!("../static/robots.txt"))
}

fn not_found<B>(res: dev::ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    Err(MyError::NotFound {
        text: res.request().path().to_string(),
    }
    .into())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let error_handlers = ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found);
        App::new()
            .wrap(error_handlers)
            .service(index)
            .service(favicon)
            .service(rss_stpl)
            .route("/robots.txt", web::get().to(robots_txt))
    })
    .bind(("127.0.0.1", 8080))?
    .workers(1)
    .run()
    .await
}
