use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::env;
use std::fs;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref DIRECTORY: Mutex<String> = Mutex::new(".".to_string());
}

fn main() {
    parse_arguments();
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
            }
        }
    }
}

fn parse_arguments() {
    let args: Vec<String> = env::args().collect();

    for (index, arg) in args.iter().enumerate() {
        if arg == "--directory" {
            if index + 1 < args.len() {
                let mut directory = DIRECTORY.lock().unwrap();
                *directory = args[index + 1].clone();
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer);
    let mut response = String::from("HTTP/1.1 200 OK\r\n\r\n");  // default response

    let lines: Vec<&str> = request.lines().collect();
    if let Some(first_line) = lines.get(0) {
        let parts: Vec<&str> = first_line.split_whitespace().collect();

        if parts.len() >= 2 {
            let request_method = parts[0];
            let request_path = parts[1];

            if request_path.starts_with("/echo/") { // return string after /echo/
                let response_body = request_path.split("/echo/").nth(1).unwrap();
                let response_length = response_body.len();
                response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n", response_length, response_body);
            } else if request_path.starts_with("/user-agent") { // return User-Agent from request
                let response_body = parse_user_agent(lines);
                let response_length = response_body.len();
                response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n", response_length, response_body);
            } else if request_path.starts_with("/files/") {
                let filename = request_path.split("/files/").nth(1).unwrap();
                let directory = DIRECTORY.lock().unwrap().clone();
                let path = format!("{}{}", directory, filename);

                if request_method == "GET" { // retrieve a file from the server
                    if fs::metadata(&path).is_ok() {
                        match fs::read_to_string(&path) {
                            Ok(file_content) => {
                                let file_length = file_content.len();
                                response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n", file_length, file_content);
                                println!("File {} exists in the directory", filename);
                            }
                            Err(err) => {
                                response = String::from("HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n");
                                eprintln!("Failed to read file content: {}", err);
                            }
                        }
                    } else {
                        response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
                        println!("File {} does not exist in the directory", filename);
                    }
                } else if request_method == "POST" { // store a file on the server
                    let contents = lines[lines.len() - 1].trim_matches(char::from(0));

                    // Write the file content to the specified file
                    if let Err(err) = fs::write(&path, &contents) {
                        response = String::from("HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n");
                        eprintln!("Failed to save the file: {}", err);
                    } else {
                        response = format!("HTTP/1.1 201 Created\r\n\r\n");
                        println!("File saved at: {}", path);
                    }
                } else {
                    response = String::from("HTTP/1.1 400 Bad Request\r\n\r\n");
                }
            } else if request_path != "/" {
                response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
            } // default 200 OK
        } else {
            response = String::from("HTTP/1.1 400 Bad Request\r\n\r\n");
        }
    } else {
        response = String::from("HTTP/1.1 400 Bad Request\r\n\r\n");
    }

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn parse_user_agent(request: Vec<&str>) -> &str {
    let user_agent_line = request.iter().find(|line| line.to_lowercase().starts_with("user-agent")).unwrap();
    let user_agent = user_agent_line.split("User-Agent: ").nth(1).unwrap();

    return user_agent;
}
