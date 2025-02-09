use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
use std::sync::{mpsc, Arc};
use threadpool::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    println!("Listener started on {:?}", listener);

    let pool = ThreadPool::build(4).expect("could not start up thread pool");

    /*
    quite complicated workaround so we can cancel without an actual request coming in:
    instead of directly iterating over listener.incoming() and observing some flag for shutdown (which only gets noticed when an actual connection comes in),
    we introduce another channel which transports StreamOrStop. our main thread consumes from this channel and either
    - forwards actual connections to ThreadPool
    - stops reading from the channel and returns from main. this causes drop of pool which causes threads to stop (and waits for them)
     */
    let (sender, receiver) = mpsc::channel();
    let sender = Arc::new(sender);
    let sender_clone = Arc::clone(&sender);

    ctrlc::
        set_handler(move || {
            println!("Received Ctrl-C, shutting down...");
            sender_clone.send(StreamOrStop::Stop).unwrap();
        })
        .expect("Error setting Ctrl-C handler");

    // this thread is not stopping. we spawn it so we can leave main()
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            #[cfg(debug_assertions)]
            println!("Connection established with {:?}", stream);

            if let Err(e) = sender.send(StreamOrStop::Stream(stream)) {
                eprintln!("Error forwarding stream, likely a new connection came in during shutdown: {e}");
                break;
            }
        }
    });

    for stream in receiver.iter() {
        match stream {
            StreamOrStop::Stream(s) => pool.execute(|| handle_connection(s)),
            StreamOrStop::Stop => break,
        }
    }
}

enum StreamOrStop {
    Stream(TcpStream),
    Stop,
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // println!("Request: {http_request:#?}");

    let first_request_line = match http_request.first() {
        Some(l) => l,
        None => {
            eprintln!("Received empty request from {:?}, dropping connection", stream.peer_addr());
            return;
        }
    };

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

    #[cfg(debug_assertions)]
    println!("Connection closing with {:?}", stream);
}

struct HttpResult {
    status_line: String,
    filename: Option<String>,
}