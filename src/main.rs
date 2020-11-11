#![allow(unused_imports)]

extern crate nix;
extern crate ansi_term;
extern crate hyper;
extern crate tokio;
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
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{CONTENT_TYPE};
use futures::*;
use std::str;
use std::i64;
use std::u128;
use hyper::Body;
use std::sync::mpsc::channel;
use std::future::Future;
use tokio::prelude::*;

static RPC_ENDPOINT: &str = "http://localhost:8545";

struct Checker {
    client: Client<hyper::client::HttpConnector, Body>,
    last_hash: String,
}

impl Checker {
    fn new() -> Checker {
        let client = Client::new();
        Checker {
            client,
            last_hash: "".into(),
        }
    }

    async fn request_block(&mut self) -> Result<Value, MyError> {
        let json: &str = r#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest", true],"id":1}"#;
        let uri: hyper::Uri = RPC_ENDPOINT.parse().unwrap();
        let req: Request<Body> = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(json))
            .unwrap();

        let resp = self.client.request(req).await?;
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
        let result_json = str::from_utf8(&body_bytes).map_err(|e| e.to_string())?;

        let res: Value = serde_json::from_str(result_json).map_err(|e| e.to_string())?;
        Ok(res)
    }

    async fn run(&mut self) -> Result<(), MyError> {
        let res = self.request_block().await?;

        let result = res["result"].as_object().unwrap();
        let size = &result["size"];

        let size_str: &str = size.as_str().unwrap();
        let z = i64::from_str_radix(&size_str[2..], 16)?;

        let number_str: &str = result["number"].as_str().unwrap();
        let number = i64::from_str_radix(&number_str[2..], 16)?;

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
            let value = u128::from_str_radix(&value_str[2..], 16)?;
            let finney = value / 1000000000000000;
            sum = sum + finney;
        }
        println!("Sum of value: {} ETH", sum / 1000);
        println!("Waiting for new block...");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut checker = Checker::new();
    let (tx, rx) = channel();

    let timer = Timer::new();
    let _guard = timer.schedule_repeating(Duration::seconds(1), move || {
        let _ignored = tx.send(());
    });

    loop {
        rx.recv().unwrap();
        let result = checker.run().await;
        if result.is_err() {
            println!("error={:?}", result);
        }
    }
}

#[derive(Debug)]
pub enum MyError {
    Request(hyper::Error),
    Other(String),
}
impl From<hyper::Error> for MyError {
    #[inline]
    fn from(error: hyper::Error) -> MyError { MyError::Request(error) }
}
impl From<String> for MyError {
    #[inline]
    fn from(error: String) -> MyError { MyError::Other(error) }
}
impl From<std::num::ParseIntError> for MyError {
    #[inline]
    fn from(error: std::num::ParseIntError) -> MyError { MyError::Other(error.to_string()) }
}