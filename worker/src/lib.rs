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
    let coordinates = req.cf()
        .and_then(|cf| cf.coordinates())
        .unwrap_or_default();
    let region = req.cf()
        .and_then(|cf| cf.region())
        .unwrap_or_else(|| "unknown region".to_string());
    
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        coordinates,
        region
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
        (Method::Get, "/list") => handle_list(req, &host, &env).await,
        (Method::Get, p) if p.starts_with("/stats/") => handle_stats(req, &host, &p[7..], &env).await,
        (Method::Post, "/create") => handle_create(req, &host, &env).await,
        (Method::Delete, p) if p.starts_with("/delete/") => handle_delete(req, &host, &p[8..], &env).await,
        (Method::Get, p) => handle_redirect(&host, &p[1..], &env).await,
        _ => Response::error("Not Found", 404),
    }
}

fn handle_root(host: &str) -> Result<Response> {
    let root_domain = match host {
        "bua.pe" => "buape.com",
        host if host.ends_with(".bua.pe") => "bua.pe",
        host => {
            let re = Regex::new(r"^[^.]+\.(.+)$").map_err(|e| worker::Error::from(e.to_string()))?;
            if let Some(captures) = re.captures(host) {
                let extracted = captures.get(1).map_or(host, |m| m.as_str());
                if extracted.contains('.') {
                    extracted
                } else {
                    host
                }
            } else {
                host
            }
        }
    };

    let redirect_url = format!("https://{}/", root_domain);
    Response::redirect(Url::parse(&redirect_url)?)
}

async fn handle_list(req: Request, host: &str, env: &Env) -> Result<Response> {
    let auth_header = req.headers().get("Authorization")?.unwrap_or_default();
    let access_key = env.secret("ACCESS_KEY")?.to_string();
    if access_key.is_empty() {
        return Response::error("Internal server error", 500);
    }

    if auth_header != format!("Bearer {}", access_key) {
        return Response::error("This maze wasn't meant for you", 401);
    }

    let kv = env.kv("SHORT_LINKS")?;
    let prefix = format!("{}:", host);
    let keys = kv.list().prefix(prefix.clone()).execute().await?;

    let mut links = Vec::new();
    for key in keys.keys {
        let full_key = key.name;
        if let Some(link_str) = kv.get(&full_key).text().await? {
            let link: ShortLink = serde_json::from_str(&link_str)?;
            // Extract slug by removing the prefix
            let slug = full_key.strip_prefix(&prefix)
                .unwrap_or("")
                .to_string();
            
            links.push(json!({
                "host": host,
                "slug": slug,
                "redirect_url": link.redirect_url,
                "hits": link.hits,
            }));
        }
    }

    Response::from_json(&links)
}

async fn handle_stats(req: Request, host: &str, slug: &str, env: &Env) -> Result<Response> {
    let auth_header = req.headers().get("Authorization")?.unwrap_or_default();
    let access_key = env.secret("ACCESS_KEY")?.to_string();
    if access_key.is_empty() {
        return Response::error("Internal server error", 500);
    }

    if auth_header != format!("Bearer {}", access_key) {
        return Response::error("This maze wasn't meant for you", 401);
    }

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
    let auth_header = req.headers().get("Authorization")?.unwrap_or_default();
    let access_key = env.secret("ACCESS_KEY")?.to_string();
    if access_key.is_empty() {
        return Response::error("Internal server error", 500);
    }

    if auth_header != format!("Bearer {}", access_key) {
        return Response::error("This maze wasn't meant for you", 401);
    }

    let kv = env.kv("SHORT_LINKS")?;

    let json_body = match req.json::<serde_json::Value>().await {
        Ok(json) => json,
        Err(_) => return Response::error("Invalid JSON body", 400)
    };

    let slug = json_body["slug"].as_str().unwrap_or("");
    let redirect_url = json_body["url"].as_str().unwrap_or("");

    if slug.is_empty() || redirect_url.is_empty() {
        return Response::error("Missing required fields: 'slug' and 'url' are required", 400);
    }

    let cleaned_slug = slug.trim_matches('/');
    let key = format!("{}:{}", host, cleaned_slug);

    // Check if link already exists
    if kv.get(&key).text().await?.is_some() {
        return Response::error("Slug already exists", 409);
    }

    let short_link = ShortLink {
        redirect_url: redirect_url.to_string(),
        hits: 0,
    };

    match kv.put(&key, &serde_json::to_string(&short_link)?)?.execute().await {
        Ok(_) => {
            Response::ok(json!({
                "message": "Short link created successfully",
                "short_url": format!("https://{}/{}", host, cleaned_slug)
            }).to_string())
            .map(|mut resp| {
                resp.headers_mut().set("Content-Type", "application/json").unwrap();
                resp
            })
        },
        Err(_) => Response::error("Internal server error", 500)
    }
}

async fn handle_redirect(host: &str, path: &str, env: &Env) -> Result<Response> {
    if ["create", "stats", "list"].contains(&path) {
        return Response::error("Not Found", 404);
    }

    let kv = env.kv("SHORT_LINKS")?;
    let key = format!("{}:{}", host, path);

    if let Some(link_str) = kv.get(&key).text().await? {
        let mut link: ShortLink = serde_json::from_str(&link_str)?;
        link.hits += 1;

        kv.put(&key, &serde_json::to_string(&link)?)?
            .execute()
            .await?;

        Response::redirect(Url::parse(&link.redirect_url)?)
    } else {
        // If the full path doesn't match, try to find the longest matching prefix
        let parts: Vec<&str> = path.split('/').collect();
        for i in (0..parts.len()).rev() {
            let partial_path = parts[..=i].join("/");
            let partial_key = format!("{}:{}", host, partial_path);
            
            if let Some(link_str) = kv.get(&partial_key).text().await? {
                let mut link: ShortLink = serde_json::from_str(&link_str)?;
                link.hits += 1;

                kv.put(&partial_key, &serde_json::to_string(&link)?)?
                    .execute()
                    .await?;

                return Response::redirect(Url::parse(&link.redirect_url)?);
            }
        }

        Response::error("Short link not found", 404)
    }
}

async fn handle_delete(req: Request, host: &str, slug: &str, env: &Env) -> Result<Response> {
    let auth_header = req.headers().get("Authorization")?.unwrap_or_default();
    let access_key = env.secret("ACCESS_KEY")?.to_string();
    if access_key.is_empty() {
        return Response::error("Internal server error", 500);
    }

    if auth_header != format!("Bearer {}", access_key) {
        return Response::error("This maze wasn't meant for you", 401);
    }

    let kv = env.kv("SHORT_LINKS")?;

    let key = format!("{}:{}", host, slug);

    if kv.delete(&key).await.is_ok() {
        Response::ok("Short link deleted successfully")
    } else {
        Response::error("Short link not found", 404)
    }
}