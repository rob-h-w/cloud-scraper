use warp::{reply, Filter};

pub fn router() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    root()
}

fn root() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path::end().map(move || reply::html(include_str!("../../resources/html/index.html")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    #[tokio::test]
    async fn test_filter() {
        let filter = router();
        let res = request().method("GET").path("/").reply(&filter).await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.body(), include_str!("../../resources/html/index.html"));
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
    }
}
