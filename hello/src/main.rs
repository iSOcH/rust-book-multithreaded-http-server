use std::io::Write;
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Listener started on {:?}", listener);
    
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Connection established with {:?}", stream);
        
        stream.write_fmt(format_args!("Hello World!\n")).unwrap();
        
        println!("Connection closing with {:?}", stream);
    }
}