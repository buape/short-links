use serde_json::json;
use worker::*;
use serde::{Deserialize, Serialize};
use regex::Regex;

mod utils;

#[derive(Serialize, Deserialize)]
struct ShortLink {
    redirect_url: String,
    hits: u64,
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    let path = req.path();
    let method = req.method();
    let host = req.headers().get("Host")?.unwrap_or_default();

    match (method, path.as_str()) {
        (Method::Get, "/") => handle_root(&host),
        (Method::Get, "/list") => handle_list(&host, &env).await,
        (Method::Get, p) if p.starts_with("/stats/") => handle_stats(&host, &p[7..], &env).await,
        (Method::Post, "/create") => handle_create(req, &host, &env).await,
        (Method::Get, p) => handle_redirect(&host, &p[1..], &env).await,
        _ => Response::error("Not Found", 404),
    }
}

fn handle_root(host: &str) -> Result<Response> {
    // Note: this is a pretty hacky way to strip the sub-domain, and redirect to the apex domain.
    // Given that we can assume all Buape Go domains will be sub.domain.tld, this works.
    let re = Regex::new(r"^[^.]+\.(.+)$").map_err(|e| worker::Error::from(e.to_string()))?;

    let root_domain = if let Some(captures) = re.captures(host) {
        captures.get(1).map_or(host, |m| m.as_str())
    } else {
        host
    };

    let redirect_url = format!("https://{}/", root_domain);
    Response::redirect(Url::parse(&redirect_url)?)
}

async fn handle_list(host: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("SHORT_LINKS")?;
    let keys = kv.list().prefix(format!("{}:", host)).execute().await?;

    let mut links = Vec::new();
    for key in keys.keys {
        let full_key = key.name;
        let slug = full_key.split(':').nth(1).unwrap_or("");
        if let Some(link_str) = kv.get(&full_key).text().await? {
            let link: ShortLink = serde_json::from_str(&link_str)?;
            links.push(json!({
                "slug": slug,
                "redirect_url": link.redirect_url,
                "hits": link.hits,
            }));
        }
    }

    Response::from_json(&links)
}

async fn handle_stats(host: &str, slug: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("SHORT_LINKS")?;
    let key = format!("{}:{}", host, slug);

    if let Some(link_str) = kv.get(&key).text().await? {
        let link: ShortLink = serde_json::from_str(&link_str)?;
        Response::from_json(&json!({
            "slug": slug,
            "redirect_url": link.redirect_url,
            "hits": link.hits,
        }))
    } else {
        Response::error("Short link not found", 404)
    }
}

async fn handle_create(mut req: Request, host: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("SHORT_LINKS")?;

    let json_body = match req.json::<serde_json::Value>().await {
        Ok(json) => json,
        Err(_e) => {
            return Response::error("Invalid JSON body", 400);
        }
    };

    let slug = json_body["slug"].as_str().unwrap_or("");
    let redirect_url = json_body["url"].as_str().unwrap_or("");

    if slug.is_empty() || redirect_url.is_empty() {
        return Response::error("Missing required fields: 'slug' and 'url' are required", 400);
    }

    let key = format!("{}:{}", host, slug);
    let short_link = ShortLink {
        redirect_url: redirect_url.to_string(),
        hits: 0,
    };

    match kv.put(&key, &serde_json::to_string(&short_link)?)?.execute().await {
        Ok(_) => {
            Response::ok(json!({
                "message": "Short link created successfully",
                "short_url": format!("https://{}/{}", host, slug)
            }).to_string())
            .map(|mut resp| {
                resp.headers_mut().set("Content-Type", "application/json").unwrap();
                resp
            })
        },
        Err(_e) => {
            Response::error("Internal server error", 500)
        }
    }
}

async fn handle_redirect(host: &str, slug: &str, env: &Env) -> Result<Response> {
    if ["create", "stats", "list"].contains(&slug) {
        return Response::error("Not Found", 404);
    }

    let kv = env.kv("SHORT_LINKS")?;
    let key = format!("{}:{}", host, slug);

    if let Some(link_str) = kv.get(&key).text().await? {
        let mut link: ShortLink = serde_json::from_str(&link_str)?;
        link.hits += 1;

        kv.put(&key, &serde_json::to_string(&link)?)?
            .execute()
            .await?;

        Response::redirect(Url::parse(&link.redirect_url)?)
    } else {
        Response::error("Short link not found", 404)
    }
}