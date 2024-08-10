use parking_lot::Mutex;
use tokio::sync::oneshot::{Receiver, Sender};
use warp::Filter;

const HTTP_PORT: u16 = 80;

pub struct ChallengeTokenServer {
    content: String,
    domain: String,
    challenge_token: String,
    stop: Mutex<Option<Receiver<bool>>>,
    stopper: Mutex<Option<Sender<bool>>>,
}

impl ChallengeTokenServer {
    pub fn new(content: String, domain: String, challenge_token: String) -> Self {
        let (stopper, stop) = tokio::sync::oneshot::channel();
        Self {
            challenge_token,
            content,
            domain,
            stop: Mutex::new(Some(stop)),
            stopper: Mutex::new(Some(stopper)),
        }
    }

    pub async fn serve(&self) {
        let content = self.content.clone();
        let domain = self.domain.clone();
        let expected_token = self.challenge_token.clone();
        let route = warp::path!(".well-known" / "acme-challenge" / String)
            .map(move |token| {
                log::debug!("Serving challenge token for domain {}", domain);
                if token != expected_token {
                    log::error!("Received unknown challenge token {}", token);
                    warp::reply::with_status(
                        format!("token {} was not the expected value", token),
                        warp::http::StatusCode::NOT_FOUND,
                    )
                } else {
                    warp::reply::with_status(content.clone(), warp::http::StatusCode::OK)
                }
            })
            .with(warp::log("challenge_token_server"))
            .boxed();
        let stop;
        {
            let mut lock = self.stop.lock();
            stop = lock
                .take()
                .expect("No stop signal receiver found. Is this server already running?");
        }

        let (addr, fut) =
            warp::serve(route).bind_with_graceful_shutdown(([0, 0, 0, 0], HTTP_PORT), async move {
                stop.await.expect("Could not get stop signal");
            });
        log::debug!("Challenge token server listening on {}", addr);
        log::debug!(
            "Challenge token server path /.well-known/acme-challenge/{}",
            self.challenge_token.clone()
        );
        fut.await;
    }

    pub fn stop(&self) {
        let mut lock = self.stopper.lock();
        lock.take()
            .expect("No stop signal sender found. Is this server already stopped?")
            .send(true)
            .expect("Could not send stop event in challenge token server.");
    }
}
