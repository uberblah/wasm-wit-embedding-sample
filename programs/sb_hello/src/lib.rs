#![no_std]

use core::fmt::Write;
use heapless::String;
use sha2::Digest;
use sha2::Sha256;
use wit_bindgen::generate;

generate!("program" in "../../wit/shellbound.wit");

fn fib(n: u32) -> (u32, u32) {
    let result;
    let mut visits = 1;
    if n <= 1 {
        result = n;
    } else {
        let left = fib(n - 1);
        let right = fib(n - 2);
        result = left.0 + right.0;
        visits += left.1 + right.1;
    }

    (result, visits)
}

fn random_byte(s: &[u8]) -> u8 {
    let mut hasher = Sha256::new();
    hasher.update(&s);
    let result = hasher.finalize();
    result
        .to_vec()
        .into_iter()
        .fold(0, |acc, byte| acc ^ byte as u8)
}

struct HelloProgram;

impl Guest for HelloProgram {
    fn process_run() -> u32 {
        let what = read_stdin(100);
        let fib_depth = (random_byte(&what) % 10) + 10;
        let fib_result = fib(fib_depth as u32);
        let args = format_args!(
            "Hello, world! fib({}) = {}, n_visits = {}",
            fib_depth, fib_result.0, fib_result.1
        );

        let mut buffer: String<256> = String::new();

        buffer.clear();
        if let Err(_e) = buffer.write_fmt(args) {
            return 1; // Error writing to buffer
        }
        write_stdout(buffer.as_bytes());
        write_stdout("Hello from sb_hello!".as_bytes());
        0
    }
}

export!(HelloProgram);
