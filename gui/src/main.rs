#[macro_use] extern crate rocket;

use rocket::form::Form;
use rocket::fs::{FileServer, relative};
use rocket::response::{Flash, Redirect};

use rocket_dyn_templates::{Template, context};

use lbf::lbf_run::solve_json;

#[derive(Debug, FromForm)]
pub struct Json {
    pub json_str: String,
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {field: "value"})
}

#[post("/", data = "<form_data>")]
async fn json(form_data: Form<Json>) -> Flash<Redirect> {
    let json = form_data.into_inner();
    println!("Received JSON: {}", json.json_str);
    let result = solve_json(json.json_str);
    
    if json.json_str.is_empty() {
        Flash::error(Redirect::to("/"), "JSON cannot be empty.")
    } else {
        Flash::success(Redirect::to("/"), "JSON successfully submitted.")
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/json", routes![json])
        .mount("/", FileServer::from(relative!("/static")))
        .attach(Template::fairing())
}