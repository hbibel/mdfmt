use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
    time::Duration,
};

fn main() {
    if let Ok(mode) = std::env::var("MDFMT_FIXTURE_MODE") {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap();
        if let Ok(log) = std::env::var("MDFMT_FIXTURE_LOG") {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .unwrap();
            writeln!(
                file,
                "CALL {:?}\nINPUT {input:?}",
                std::env::args().skip(1).collect::<Vec<_>>()
            )
            .unwrap();
        }
        let mode = if mode == "dispatch" {
            if input.contains("bad") {
                "partial"
            } else if input.contains("unsafe") {
                "unsafe"
            } else {
                "json"
            }
        } else {
            mode.as_str()
        };
        match mode {
            "json" => print!("{{\n  \"one\": 1,\n  \"two\": 2,\n  \"three\": 3\n}}\n"),
            "jsonblank" => print!("{{\n\n  \"one\": 1,\n  \"two\": 2,\n  \"three\": 3\n}}\n"),
            "partial" => {
                print!("partial invalid output");
                std::process::exit(1);
            }
            "sleep" => std::thread::sleep(Duration::from_secs(10)),
            "unsafe" => print!("```\nOutside changed code\n```\n"),
            "invalid" => io::stdout().write_all(&[0xff]).unwrap(),
            "empty" => {}
            "nul" => io::stdout().write_all(b"code\0\n").unwrap(),
            "unstable" => {
                let log = std::env::var("MDFMT_FIXTURE_LOG").unwrap();
                let count = std::fs::read_to_string(log)
                    .unwrap()
                    .matches("CALL ")
                    .count();
                println!("version{}", count % 2);
            }
            "flood" => {
                let mut output = io::stdout().lock();
                loop {
                    if output.write_all(&[b'x'; 8192]).is_err() {
                        break;
                    }
                }
            }
            "uppercase" => print!("{}", input.to_uppercase()),
            _ => panic!("unknown CLI fixture mode"),
        }
        return;
    }
    let arguments: Vec<_> = std::env::args().collect();
    match arguments[1].as_str() {
        "sleep" => {
            std::thread::sleep(Duration::from_secs(10));
        }
        "descendant" => {
            Command::new(std::env::current_exe().unwrap())
                .arg("sleep")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
        }
        "partial" => {
            print!("partial invalid output");
            eprintln!("failed to parse code");
            std::process::exit(1);
        }
        "both" => {
            let thread =
                std::thread::spawn(|| io::stderr().write_all(&vec![b'e'; 64 * 1024]).unwrap());
            io::stdout().write_all(&vec![b'o'; 64 * 1024]).unwrap();
            thread.join().unwrap();
        }
        "flood" => {
            let mut output = io::stdout().lock();
            loop {
                if output.write_all(&[b'x'; 8192]).is_err() {
                    break;
                }
            }
        }
        mode => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input).unwrap();
            match mode {
                "echo" => print!("{input}"),
                "uppercase" => print!("{}", input.to_uppercase()),
                "invalid" => io::stdout().write_all(&[0xff]).unwrap(),
                "empty" => {}
                "nul" => io::stdout().write_all(b"code\0\n").unwrap(),
                "alternating" => {
                    let path = &arguments[2];
                    let count = std::fs::read_to_string(path)
                        .ok()
                        .and_then(|text| text.parse::<usize>().ok())
                        .unwrap_or(0)
                        + 1;
                    std::fs::write(path, count.to_string()).unwrap();
                    println!("version{count}");
                }
                _ => panic!("unknown fixture mode"),
            }
        }
    }
}
