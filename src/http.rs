extern crate reqwest;
extern crate async_trait;

use reqwest::{Client, Error as ReqwestError};
use async_trait::async_trait;

#[async_trait]
pub trait HttpRequest {
    async fn send_request(
        &self, 
        client: &Client, 
        method: &str, 
        url: &str,
        headers: &[String],
        data: &String
    ) -> Result<(), ReqwestError>;
}

pub struct HttpMethodRequest;

#[async_trait]
impl HttpRequest for HttpMethodRequest {
    async fn send_request(
        &self,
        client: &Client,
        method: &str,
        url: &str,
        headers: &[String],
        data: &String
    ) -> Result<(), ReqwestError> {
        let mut request = match method {
            "get" => client.get(url),
            "post" => client.post(url),
            "patch" => client.patch(url),
            "put" => client.put(url),
            "delete" => client.delete(url),
            _ => {
                eprintln!("Invalid HTTP method: {}", method);
                return Ok(());
            }
        };

        for header in headers {
            let parts: Vec<&str> = header.splitn(2, ':').collect();
            if parts.len() == 2 {
                request = request.header(parts[0], parts[1]);
            } else {
                eprintln!("Invalid header format: {}", header);
            }
        }

        let body = reqwest::Body::from(data.clone());
        request = request.body(body);

        
        request.send().await?;

        Ok(())
    }
}

