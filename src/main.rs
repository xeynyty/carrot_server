use carrot_sdk::data::Data;
use inmemory::memory::Memory;
use carrot_sdk::Request;
use inmemory::Manager;
use anyhow::Result;
use carrot_sdk::result::Response;
use carrot_sdk::utils::Work;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {

    let cache = Manager::new()
        .run(None)
        .await;

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let cache = cache.clone();
        let (mut socket, _s) = listener.accept().await?;
        tokio::spawn(async move {

            let mut buf = [0u8; 4048];

            loop {
                 let _n = match socket.read(&mut buf).await {
                     Ok(n) if n == 0 => return,
                     Ok(n) => n,
                     Err(e) => {
                         eprintln!("failed to read from socket; err = {:?}", e);
                         return;
                     }
                 };

                let req: Request = match buf.as_slice().try_into() {
                    Ok(x) => {
                        // println!("{:?}", x);
                        x
                    },
                    Err(e) => {
                        eprintln!("error {}", e);
                        return;
                    }
                };

                let resp = response(req, &cache).await;
                let result: Vec<u8> = resp.try_into().unwrap_or(vec![6, 6, 6]);
                // println!("RES LEN {}", result.len());

                if let Err(e) = socket.write_all(&result).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}


async fn response(req: Request, cache: &Memory<u32, Data>) -> Response {
    return match req.work() {
        Work::Read => {
            let mut key: u32 = 0;
            let mut data: Data = Data::None;
            if let Some(x) = req.key() {
                key = x;
                data = cache.get(x).await.unwrap_or(Data::None);
            }
            Response::new(key, data)
        }
        Work::Write => {
            let mut key: u32 = 0;
            let data: Data = Data::None;

            if let Some(k) = req.key() {
                key = k;
                let _res = cache.add(k, req.data(), req.iat()).await;
            }

            Response::new(key, data)
        }
        Work::Remove => {
            let mut key: u32 = 0;
            let mut data: Data = Data::None;

            if let Some(k) = req.key() {
                key = k;
                if let Some(x) = cache.remove(k).await {
                    data = x.get().unwrap_or(Data::None)
                }
            }
            Response::new(key, data)
        }
        Work::None => {
            Response::new(0, Data::None)
        }
    };
}