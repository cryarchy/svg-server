mod usage_guide;

use std::{net::SocketAddr, path::PathBuf, sync::LazyLock};

use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use handlebars::Handlebars;
use regex::Regex;
use rust_embed::RustEmbed;
use structopt::StructOpt;
use usage_guide::USAGE_GUIDE;

#[derive(Debug, StructOpt)]
struct Opt {
    /// Bind address to listen on
    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on
    #[structopt(short = "p", long = "port", default_value = "5000")]
    port: u16,

    /// Route to redirect / to
    #[structopt(short = "i", long = "index", default_value = "/home")]
    index: String,

    /// Path to a directory containing the SVG files to be served
    #[structopt(parse(from_os_str))]
    path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct SvgPath(PathBuf);

#[derive(Debug, Clone)]
struct RedirectIndexTo(String);

#[derive(RustEmbed)]
#[folder = "templates"]
struct Assets;

static HEIGHT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"height\s*=\s*"[^"]*""#).unwrap());

static WIDTH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"width\s*=\s*"[^"]*""#).unwrap());

fn svg_size_full_width(svg_content: &str) -> Result<String, String> {
    let svg_start = svg_content
        .find("<svg")
        .ok_or("No SVG start found".to_owned())?;
    let svg_end = svg_content[svg_start..]
        .find('>')
        .ok_or("No SVG end found".to_owned())?;

    // Get the full SVG tag line
    let svg_tag_line = &svg_content[svg_start..svg_start + svg_end + 1];
    // Remove height attribute
    let new_svg_tag_line = HEIGHT_RE.replace_all(svg_tag_line, "").to_string();

    // Replace width attribute with 100%
    let new_svg_tag_line = WIDTH_RE
        .replace_all(&new_svg_tag_line, "width=\"100%\"")
        .to_string();

    Ok(svg_content.replace(svg_tag_line, &new_svg_tag_line))
}

#[get("/")]
async fn home_redirect(redirect_to: web::Data<RedirectIndexTo>) -> impl Responder {
    // Permanent redirect to /home
    println!("Redirecting / to {}", redirect_to.0);
    web::redirect("/", redirect_to.0.to_owned()).temporary()
}

#[get("/{page}")]
async fn render_svg(
    page: web::Path<String>,
    template_engine: web::Data<Handlebars<'_>>,
    opt: web::Data<SvgPath>,
) -> impl Responder {
    let page = page.into_inner().to_lowercase().replace(':', "/");
    let svg_path = format!("{}.svg", page.as_str());
    let full_svg_path = opt.0.join(svg_path);
    println!("Loading SVG at: {}", full_svg_path.display());

    // Read SVG file contents
    let svg_content = match std::fs::read_to_string(&full_svg_path) {
        Ok(content) => match svg_size_full_width(&content) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("{e}");
                return HttpResponse::InternalServerError().body(e);
            }
        },
        Err(e) => {
            eprintln!("{e}");
            return HttpResponse::InternalServerError().body("Failed to load SVG");
        }
    };

    // Prepare template data
    let data = serde_json::json!({
        "title": page,
        "svg_content": svg_content
    });

    // Render template
    match template_engine.render("layout", &data) {
        Ok(rendered) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(rendered),
        Err(e) => {
            eprintln!("{e}");
            HttpResponse::InternalServerError().body("Template rendering error")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("{USAGE_GUIDE}\n\n");

    // Parse command line arguments
    let opt = Opt::from_args();

    // Create socket address from bind address and port
    let addr = format!("{}:{}", opt.bind, opt.port);
    let socket_addr = addr.parse::<SocketAddr>().expect("Invalid address");

    // Get SVG folder path (use current directory if none provided)
    let svg_folder = match opt.path {
        Some(path) => SvgPath(path),
        None => SvgPath(PathBuf::from(".")),
    };

    // Verify SVG folder exists
    if !svg_folder.0.exists() {
        eprintln!(
            "Error: SVG folder '{}' does not exist",
            svg_folder.0.display()
        );
        return Ok(());
    }

    // Initialize Handlebars
    let mut hb = Handlebars::new();

    // Register templates from files
    hb.register_embed_templates_with_extension::<Assets>(".hbs")
        .unwrap();

    println!("Server started at http://{socket_addr}");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(hb.clone()))
            .app_data(web::Data::new(svg_folder.clone()))
            .app_data(web::Data::new(RedirectIndexTo(opt.index.to_owned())))
            .service(home_redirect)
            .service(render_svg)
    })
    .bind(socket_addr)?
    .run()
    .await
}
