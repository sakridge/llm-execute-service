use {
    actix_web::{error, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder},
    crossbeam_channel::{unbounded, RecvTimeoutError, Sender},
    derive_more::{Display, Error},
    libloading::{Library, Symbol},
    log::*,
    std::{
        collections::HashMap,
        ffi::{c_char, c_int, CString},
        time::Duration,
    },
};

type RunLlmFunc = unsafe fn(argc: c_int, argv: *const *const c_char) -> c_int;

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

async fn handle_request(body: web::Bytes, tx: web::Data<Sender<JobData>>) -> HttpResponse {
    info!("received req: {:?}", body);
    if let Ok(json) = serde_json::from_slice::<JobData>(&body) {
        //TODO: validate parameters
        info!("good json: {:?}", json.get("url"));
        tx.send(json).unwrap();
        return HttpResponse::Ok().body("Job was added!");
    }
    HttpResponse::NoContent().into()
}

fn handle_job(library: &Option<Library>) -> std::io::Result<()> {
    if let Some(ref l) = library {
        let model = "./models/7B/ggml-model-q4_0.bin";
        let prompt = "give me a website";
        let seed = "12345";
        let args = format!("-m {} -p {} -s {}", model, prompt, seed);
        let args = [CString::new("-c")?.as_ptr()];
        unsafe {
            let func_run_llm: Symbol<RunLlmFunc> = l.get(b"run_llm").unwrap();
            func_run_llm(1, args.as_ptr());
        }
    }
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let (tx, rx) = unbounded::<JobData>();
    let library = unsafe {
        match Library::new("libllama.dylib") {
            Ok(l) => {
                info!("loaded libllama! {:?}", l);
                Some(l)
            }
            Err(e) => {
                info!("loading error: {:?}", e);
                None
            }
        }
    };
    let worker = std::thread::Builder::new()
        .spawn(move || loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(p) => {
                    info!("received a job: {:?}", p);
                    handle_job(&library);
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(e) => {
                    info!("error: {:?}", e);
                }
            }
        })
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .data(tx.clone())
            .service(web::scope("/api").route("/addjob", web::post().to(handle_request)))
        // Set the endpoint to /addjob
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
