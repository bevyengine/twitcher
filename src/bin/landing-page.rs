use tera::Tera;

fn main() {
    let tera = Tera::new("templates/*").unwrap();
    // Prepare the context with some data
    let context = tera::Context::new();

    // Render the template with the given context
    let rendered = tera.render("landing-page.html", &context).unwrap();
    std::fs::write("./index.html", &rendered).unwrap();
}
