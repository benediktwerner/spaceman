use std::{boxed::Box, pin::Pin, time::Duration};

use anyhow::Result;
use clap::Parser;
use futures::{Stream, StreamExt};
use rand::prelude::*;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming, transport::{Identity, Server, ServerTlsConfig}};

use pb::{
    CountdownRequest,
    CountdownResponse,
    HangmanRequest, HangmanResponse, HashRequest, HashResponse, math_request::Op,
    MathRequest, MathResponse, playground_server::{Playground, PlaygroundServer}, SecretRequest, SecretResponse,
};

mod pb {
    tonic::include_proto!("playground");
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Default)]
struct PlaygroundImpl;

#[tonic::async_trait]
impl Playground for PlaygroundImpl {
    async fn math(&self, req: Request<MathRequest>) -> Result<Response<MathResponse>, Status> {
        let (lhs, rhs) = (req.get_ref().lhs, req.get_ref().rhs);
        let result = match req.get_ref().op() {
            Op::Add => lhs + rhs,
            Op::Subtract => lhs - rhs,
            Op::Multiply => lhs * rhs,
            Op::Divide => lhs / rhs,
        };
        Ok(Response::new(MathResponse { result }))
    }

    type CountdownStream = ResponseStream<CountdownResponse>;
    async fn countdown(
        &self,
        req: Request<CountdownRequest>,
    ) -> Result<Response<Self::CountdownStream>, Status> {
        let mut left = req.get_ref().seconds;
        let (tx, rx) = mpsc::channel(4);

        tokio::spawn(async move {
            loop {
                match tx.send(Ok(CountdownResponse { left })).await {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("[COUNTDOWN] {}", err.to_string());
                        return;
                    }
                }

                if left == 0 {
                    break;
                }

                left -= 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            println!("[COUNTDOWN] Done");
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn hash(
        &self,
        mut req: Request<Streaming<HashRequest>>,
    ) -> Result<Response<HashResponse>, Status> {
        let mut hasher = Sha256::new();
        while let Some(piece) = req.get_mut().next().await {
            match piece {
                Ok(piece) => {
                    hasher.update(&piece.piece);
                },
                Err(err) => {
                    eprintln!("[HASH] {}", err.to_string());
                    return Err(err);
                }
            }
        }
        println!("[HASH] Done");

        Ok(Response::new(HashResponse {
            hash: hasher.finalize().to_vec(),
        }))
    }

    type HangmanStream = ResponseStream<HangmanResponse>;
    async fn hangman(
        &self,
        req: Request<Streaming<HangmanRequest>>,
    ) -> Result<Response<Self::HangmanStream>, Status> {
        let games = vec![
            (
                "_u_t".chars().collect::<Vec<_>>(),
                "Rust".chars().collect::<Vec<_>>(),
            ),
            (
                "gR__".chars().collect::<Vec<_>>(),
                "gRPC".chars().collect::<Vec<_>>(),
            ),
        ];
        let (mut game, target) = games
            .choose(&mut thread_rng())
            .cloned()
            .expect("can pick random game");

        let mut in_stream = req.into_inner();

        let mut lives_left = 3;

        let (tx, rx) = mpsc::channel(4);
        tokio::spawn(async move {
            loop {
                match in_stream.next().await {
                    Some(Ok(HangmanRequest { letter: None })) => {}
                    Some(Ok(HangmanRequest {
                        letter: Some(letter),
                    })) => {
                        match &letter.chars().collect::<Vec<_>>()[..] {
                            &[] => {}
                            &[letter] => {
                                let mut changed_anything = false;
                                for i in 0..target.len() {
                                    match game[i] {
                                        '_' if target[i].to_ascii_lowercase()
                                            == letter.to_ascii_lowercase() =>
                                        {
                                            game[i] = target[i];
                                            changed_anything = true;
                                        }
                                        _ => {}
                                    }
                                }
                                if !changed_anything {
                                    lives_left -= 1;
                                }
                            }
                            _ => {
                                let _ = tx
                                    .send(Err(Status::invalid_argument("only a letter at a time")))
                                    .await;
                                break;
                            }
                        };
                    }
                    Some(Err(err)) => {
                        eprintln!("[HANGMAN READING] {}", err.to_string());
                        return;
                    },
                    None => break,
                }

                match tx
                    .send(Ok(HangmanResponse {
                        lives_left,
                        state: game.iter().collect(),
                    }))
                    .await {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("[HANGMAN WRITING] {}", err.to_string());
                        return;
                    }
                }

                if game == target || lives_left < 0 {
                    break;
                }
            }
            println!("[HANGMAN] Done");
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn secret(
        &self,
        req: Request<SecretRequest>,
    ) -> Result<Response<SecretResponse>, Status> {
        let want_password =
            hex::decode("d71030b438c47fe930c7e4e1bf5f8945629f5500994b6d4a722f1207e333d989")
                .unwrap();

        let got_password = if let Some(password) = req.metadata().get("password") {
            hex::decode(password).map_err(|err| Status::invalid_argument(err.to_string()))?
        } else if let Some(password_bin) = req.metadata().get_bin("password-bin") {
            password_bin
                .to_bytes()
                .map_err(|err| Status::invalid_argument(err.to_string()))?
                .to_vec()
        } else {
            return Err(Status::permission_denied("missing authentication"));
        };

        if got_password == want_password {
            Ok(Response::new(SecretResponse {
                secret: "the secret ingredient for the krabby patty is bbq sauce".to_string(),
            }))
        } else {
            Err(Status::permission_denied("wrong password"))
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Address to serve on
    #[clap(long, value_parser, default_value_t = String::from("0.0.0.0:7575"))]
    addr: String,
    /// Enable tls
    #[clap(long, value_parser)]
    tls: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    let addr = cli.addr.parse()?;
    let pg = PlaygroundImpl::default();

    println!("Listening on {}", &addr);

    let mut srv = Server::builder();

    if cli.tls {
        srv = srv.tls_config(ServerTlsConfig::new().identity(Identity::from_pem(
            include_bytes!("../tls/server-cert.pem"),
            include_bytes!("../tls/server-key.pem")
        )))?;
    }

    srv.add_service(PlaygroundServer::new(pg))
    .serve(addr)
    .await?;
    Ok(())
}
