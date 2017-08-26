#![allow(unused_imports)]
#![feature(i128)]
#![feature(i128_type)]

extern crate nix;
extern crate ansi_term;
extern crate hyper;
extern crate tokio_core;
extern crate futures;
extern crate serde_json;
extern crate timer;
extern crate chrono;

use timer::Timer;
use chrono::Duration;
use std::thread;
use serde_json::{Value, Error};
use std::str::FromStr;
use ansi_term::Colour::Red;
use nix::unistd::*;
use hyper::Client;
use hyper::header::Connection;
use hyper::header::Basic;
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{ContentType};
use futures::*;
use std::str;
use std::i64;
use std::u128;
use hyper::Body;
use std::sync::mpsc::channel;


static RPC_ENDPOINT: &str = "http://localhost:8545";

struct Checker {
    client: Client<hyper::client::HttpConnector, Body>,
    core: Core,
    last_hash: String
}
impl Checker {

    fn new() -> Checker {

        let core = Core::new().expect("core");
        let handle = core.handle();

        let client = Client::new(&handle);

        Checker {
            client: client,
            core: core,
            last_hash: "".into()
        }
    }

    fn request_block(&mut self) -> Result<Value, String> {

        let json = r#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest", true],"id":1}"#;
        let uri: hyper::Uri = RPC_ENDPOINT.parse().unwrap(); //.map_err(|e| e.to_string())?;
        let mut req = Request::new(Method::Post, uri);
        req.headers_mut().set(ContentType::json());
        req.set_body(json);

        let post = self.client.request(req).and_then(|res| {

            res.body().concat2()
        });

        let block = self.core.run(post).map_err(|e| e.to_string())?;

        let result_json = str::from_utf8(&block).map_err(|e| e.to_string())?;

        let res: Value = serde_json::from_str(result_json).map_err(|e| e.to_string())?;
        Ok(res)
    }

    fn run(&mut self) -> Result<(), String>  {
        let res = self.request_block()?;

        let result = res["result"].as_object().unwrap();
        let size = &result["size"];

        let  size_str: &str = size.as_str().unwrap();
        let z = i64::from_str_radix(&size_str[2..], 16).unwrap();

        let number_str: &str = result["number"].as_str().unwrap();
        let number = i64::from_str_radix(&number_str[2..], 16).unwrap();

        let new_hash = result["hash"].as_str().unwrap();
        let new_h_st: String = new_hash.into();
        if self.last_hash == new_h_st {
            return Ok(());
        }
        self.last_hash = new_h_st;

        println!("Hash: {}", Red.paint(new_hash));
        println!("Number: {}", number);
        println!("Size: {:?}", z);

        println!("Transactions: {}", result["transactions"].as_array().unwrap().len());

        let mut sum: u128 = 0;
        for transaction in result["transactions"].as_array().unwrap() {
            let value_str = transaction["value"].as_str().unwrap();
            let value = u128::from_str_radix(&value_str[2..], 16).unwrap();
            let finney = value / 1000000000000000;
            sum = sum + finney;

        }
        println!("Sum of value: {} ETH", sum/1000);
        println!("Waiting for new block...");
        Ok(())
    }
}


fn main() {

    let mut checker = Checker::new();
    let (tx, rx) = channel();

    let timer = Timer::new();
    let _guard = timer.schedule_repeating(Duration::seconds(1), move || {
         let _ignored = tx.send(());
     });

    loop {
        rx.recv().unwrap();
        checker.run().unwrap();
    }
}
