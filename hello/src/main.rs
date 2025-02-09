use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
use std::ops::{Deref};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

fn main() {
    let stopped = Arc::new(Mutex::new(false));
    let stopped_clone = Arc::clone(&stopped);
    ctrlc::set_handler(move || *stopped_clone.lock().unwrap() = true).expect("Error setting Ctrl-C handler");

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Listener started on {:?}", listener);

    let pool = ThreadPool::build(4).expect("could not start up thread pool");
    
    for stream in listener.incoming().take_while(|_| !stopped.lock().unwrap().deref()) {
        let stream = stream.unwrap();
        println!("Connection established with {:?}", stream);

        pool.execute(|| handle_connection(stream));
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // println!("Request: {http_request:#?}");

    let first_request_line = http_request.first();

    if first_request_line.is_none() {
        eprintln!("Received empty request from {:?}, dropping connection", stream.peer_addr());
        return;
    }

    let first_request_line = first_request_line.unwrap();

    // [..] gets the (complete) string as slice (String -> str), since string literals are &str we also need the &
    let result = match &first_request_line[..] {
        "GET / HTTP/1.1" => {
            HttpResult {
                status_line: "HTTP/1.1 200 OK".to_owned(),
                filename: Some("hello.html".to_owned())
            }
        },
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            HttpResult { status_line: "HTTP/1.1 200 OK".to_owned(), filename: None }
        },
        "GET /nofile HTTP/1.1" => {
            HttpResult { status_line: "HTTP/1.1 200 OK".to_owned(), filename: None }
        },
        _ => {
            HttpResult {
                status_line: "HTTP/1.1 404 NOT FOUND".to_owned(),
                filename: Some("404.html".to_owned())
            }
        }
    };

    let mut response = result.status_line;

    match result.filename {
        Some(file_name) => {
            let file_contents = fs::read_to_string(file_name).unwrap();
            let len = file_contents.len();

            response.push_str(&format!("\r\nContent-Length: {len}\r\n\r\n{file_contents}"));
        }
        None => {
            response.push_str("\r\n\r\n");
        }
    };

    stream.write_all(response.as_bytes()).unwrap();

    println!("Connection closing with {:?}", stream);
}

struct HttpResult {
    status_line: String,
    filename: Option<String>,
}