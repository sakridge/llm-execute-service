use {
    actix_web::{HttpResponse, HttpRequest, error, post, web, App, HttpServer, Responder},
    derive_more::{Display, Error},
    std::{time::Duration, collections::HashMap},
    crossbeam_channel::{Sender, unbounded, RecvTimeoutError},
    log::*,
};

#[derive(Debug, Display, Error)]
#[display(fmt = "my error: {}", name)]
pub struct MyError {
    name: &'static str,
}

// Use default implementation for `error_response()` method
impl error::ResponseError for MyError {}

/*#[post("/addjob")]
async fn add_job(req: HttpRequest) -> Result<&'static str, MyError> {
//async fn add_job(body: String) -> Result<&'static str, MyError> {
    println!("job added? \"{}\"", req.body());
    let my_json: HashMap<String, String> = serde_json::from_str(&req.body()).map_err(|e| {
        println!("json error: {:?}", e);
        MyError { name: "bad json" }
    })?;
    println!("job url: {:?}", my_json.get("url"));
    Ok("job added!")
}*/

pub type JobData = HashMap<String, String>;

async fn handle_request(
    body: web::Bytes,
    tx: web::Data<Sender<JobData>>
) -> HttpResponse {
    info!("received req: {:?}", body);
    if let Ok(json) = serde_json::from_slice::<JobData>(&body) {
        //TODO: validate parameters
        info!("good json: {:?}", json.get("url"));
        tx.send(json).unwrap();
        return HttpResponse::Ok().body("Job was added!");
    }
    HttpResponse::NoContent().into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let (sender, receiver) = unbounded::<()>();
    let (tx, rx) = unbounded::<Vec<u8>>();
    let worker = std::thread::Builder::new().spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(p) => {
                    info!("received a job: {:?}", p);
                },
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                },
                Err(RecvTimeoutError::Timeout) => {
                },
                Err(e) => {
                    info!("error: {:?}", e);
                }
            }
        }
    }).unwrap();
    let sender = web::Data::new(sender);


    HttpServer::new(move || {
        App::new()
            .data(tx.clone())
            .service(web::scope("/api")
                .route("/addjob", web::post().to(handle_request)))  // Set the endpoint to /addjob
    })
    /*HttpServer::new(move || {
        App::new()
            .app_data(sender.clone())
            .service(
            web::scope("/api").service(add_job)
        )
    })*/
    .bind(("127.0.0.1", 9100))?
    .run()
    .await
}


