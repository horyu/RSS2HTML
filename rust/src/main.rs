#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use rocket::{
    config::{Config, Environment},
    http::{uri::Uri, ContentType, RawStr, Status},
    response,
    response::content
};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate rust_embed;
#[derive(RustEmbed)]
#[folder = "files/"]
struct RustAsset;
#[derive(RustEmbed)]
#[folder = "../common_files"]
struct CommonAsset;

use chrono::DateTime;
use rss::Channel;
use tera::{Context, Tera};

use serde::{Deserialize, Serialize};

#[get("/")]
fn index<'r>() -> response::Result<'r> {
    RustAsset::get("index.html").map_or_else(
        || Err(Status::NotFound),
        |d| {
            response::Response::build()
                .header(ContentType::HTML)
                .sized_body(std::io::Cursor::new(d))
                .ok()
        },
    )
}

#[get("/robots.txt")]
fn robots<'r>() -> response::Result<'r> {
    CommonAsset::get("robots.txt").map_or_else(
        || Err(Status::NotFound),
        |d| {
            response::Response::build()
                .header(ContentType::Plain)
                .sized_body(std::io::Cursor::new(d))
                .ok()
        },
    )
}

#[get("/favicon.svg")]
fn favicon<'r>() -> response::Result<'r> {
    CommonAsset::get("favicon.svg").map_or_else(
        || Err(Status::NotFound),
        |d| {
            response::Response::build()
                .header(ContentType::SVG)
                .sized_body(std::io::Cursor::new(d))
                .ok()
        },
    )
}

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.autoescape_on(vec!["one_off"]);
        let template_u8 = RustAsset::get("rss.tera").unwrap();
        let template_str = std::str::from_utf8(&template_u8).unwrap();
        tera.add_raw_template("one_off", template_str).unwrap();
        tera
    };
}

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    title: String,
    link: String,
    pub_date: String,
}

#[get("/rss.tera?<url>")]
fn rss_tera(url: &RawStr) -> content::Html<String> {
    let items = get_items(url);
    let mut context = Context::new();
    context.insert("items", &items);
    content::Html(TEMPLATES.render("one_off", &context).unwrap())
}

fn get_items(url: &RawStr) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let rss_url = match Uri::percent_decode(&url.as_bytes()) {
        Ok(x) => x.into_owned(),
        _ => return items,
    };
    let res = match reqwest::get(&rss_url) {
        Ok(mut x) => x.text().unwrap(),
        _ => return items,
    };
    let channel = match Channel::read_from(std::io::BufReader::new(res.as_bytes())) {
        Ok(x) => x,
        _ => return items,
    };
    for item in channel.items() {
        items.push(Item {
            title: extract_string(item.title()),
            link: extract_string(item.link()),
            pub_date: extract_date_string(item),
        })
    }
    items
}

fn extract_string(opstr: Option<&str>) -> String {
    opstr.unwrap_or_else(|| "").to_string()
}

fn extract_date_string(item: &rss::Item) -> String {
    let date = item.pub_date().unwrap_or_else(|| {
        if let Some(x) = item.dublin_core_ext() {
            if let Some(y) = x.dates().first() {
                return y;
            }
        }
        ""
    });
    if date.is_empty() {
        return date.to_string();
    }
    if let Ok(x) = DateTime::parse_from_rfc2822(date) {
        x.format("%F %R").to_string()
    } else if let Ok(y) = DateTime::parse_from_rfc3339(date) {
        y.format("%F %R").to_string()
    } else {
        date.to_string()
    }
}

fn main() {
    let config = Config::build(Environment::Staging)
        .port(8080)
        .finalize()
        .unwrap();
    rocket::custom(config)
        .mount("/", routes![index, robots, favicon, rss_tera])
        .launch();
}
