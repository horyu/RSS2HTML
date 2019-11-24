#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use rocket::http::uri::Uri;
use rocket::http::RawStr;
use rocket::response::NamedFile;
use rocket_contrib::templates::Template;

use tera::Context;

extern crate reqwest;

use rss::Channel;
use std::io::BufReader;

use chrono::DateTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    title: String,
    link: String,
    pub_date: String,
}

#[get("/")]
fn index() -> Option<NamedFile> {
    NamedFile::open(std::path::Path::new("index.html")).ok()
}

#[get("/rss.tera?<url>")]
fn rss_tera(url: &RawStr) -> Template {
    let mut context = Context::new();
    let rss_url = Uri::percent_decode(&url.as_bytes()).unwrap().into_owned();
    let res = match reqwest::get(&rss_url) {
        Ok(mut x) => x.text().unwrap(),
        Err(_) => return Template::render("rss", &context),
    };
    let channel = match Channel::read_from(BufReader::new(res.as_bytes())) {
        Ok(x) => x,
        Err(_) => return Template::render("rss", &context),
    };
    let mut items: Vec<Item> = Vec::new();
    for item in channel.items() {
        items.push(Item {
            title: extract_string(item.title()),
            link: extract_string(item.link()),
            pub_date: extract_date_string(item),
        })
    }
    context.insert("items", &items);
    Template::render("rss", &context)
}

fn extract_string(opstr: Option<&str>) -> String {
    opstr.unwrap_or_else(|| "").to_string()
}

fn extract_date_string(item: &rss::Item) -> String {
    let date: &str = if let Some(x) = item.pub_date() {
        x
    } else if let Some(y) = item.dublin_core_ext() {
        match y.dates().first() {
            Some(z) => z,
            None => "",
        }
    } else {
        ""
    };
    if date.is_empty() {
        return date.to_string();
    }
    if let Ok(x) = DateTime::parse_from_rfc2822(date) {
        x.format("%F %R").to_string()
    } else if let Ok(y) = DateTime::parse_from_rfc3339(date) {
        y.format("%F %R").to_string()
    } else {
        "".to_string()
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, rss_tera])
        .attach(Template::fairing())
        .launch();
}
