use std::net::SocketAddr;

use annotations::{extract_annotation, Annotation, GitHubStatus};
use axum::{
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use octocrab::{models, models::IssueState, params};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Read Personal Access Token (PAT)
    let pat = std::fs::read_to_string("pat.txt")
        .unwrap()
        .trim()
        .to_string();

    octocrab::initialise(octocrab::Octocrab::builder().personal_token(pat.into())).unwrap();

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(index))
        .route("/recogito/recogito.min.css", get(css))
        .route("/recogito/recogito.min.css.map", get(css_map))
        .route("/recogito/recogito.min.js", get(js))
        .route("/recogito/recogito.min.js.map", get(js_map))
        .route("/favicon.ico", get(favicon))
        .route("/annotations", get(get_annotations))
        .route("/annotation", post(post_annotation))
        .route("/annotation", delete(delete_annotation));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> impl IntoResponse {
    let page = {
        let index = std::fs::read_to_string("frontend/index.html").unwrap();
        let document = std::fs::read_to_string("assets/draft-ietf-mls-protocol-17.txt").unwrap();

        index.replace("{DOCUMENT}", &document)
    };

    Response::builder()
        .header("Content-Type", "text/html")
        .body(page)
        .unwrap()
}

async fn css() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "text/css")
        .body(std::fs::read_to_string("frontend/recogito/recogito.min.css").unwrap())
        .unwrap()
}

async fn css_map() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "application/json")
        .body(std::fs::read_to_string("frontend/recogito/recogito.min.css.map").unwrap())
        .unwrap()
}

async fn js() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "text/javascript")
        .body(std::fs::read_to_string("frontend/recogito/recogito.min.js").unwrap())
        .unwrap()
}

async fn js_map() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "application/json")
        .body(std::fs::read_to_string("frontend/recogito/recogito.min.js.map").unwrap())
        .unwrap()
}

async fn favicon() -> impl IntoResponse {
    let favicon = std::fs::read("frontend/assets/favicon.ico").unwrap();

    let mut response = IntoResponse::into_response(favicon);
    response
        .headers_mut()
        .insert("Content-Type", HeaderValue::from_static("image/x-icon"));

    (StatusCode::OK, response)
}

async fn get_annotations() -> impl IntoResponse {
    let octocrab = octocrab::instance();

    let mut annotations = Vec::new();

    let mut page = octocrab
        .issues("openmls", "annotations")
        .list()
        .labels(&[String::from("validation")])
        .state(params::State::All)
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if let Some(body) = &issue.body {
                if let Some(Ok(mut annotation)) = extract_annotation(body) {
                    if issue.state == IssueState::Open {
                        annotation.meta = Some(GitHubStatus::Open);
                    } else {
                        annotation.meta = Some(GitHubStatus::Closed);
                    }

                    annotations.push(annotation);
                }
            }
        }
        page = match octocrab
            .get_page::<models::issues::Issue>(&page.next)
            .await
            .unwrap()
        {
            Some(next_page) => next_page,
            None => break,
        }
    }

    (StatusCode::CREATED, Json(annotations))
}

async fn post_annotation(Json(annotation): Json<Annotation>) -> impl IntoResponse {
    let octocrab = octocrab::instance();

    let body = format!(
        "> {}\r\n\r\n\r\n---\r\n<details><summary>Annotation</summary>\r\n\r\n```annotation\r\n{}\r\n```\r\n</details>",
        annotation
            .text_quote_selector()
            .unwrap_or("```\r\n<no quote>\r\n```".into()),
        serde_json::to_string_pretty(&annotation).unwrap()
    );

    if let Some(issue_number) = find_issue(&annotation).await {
        match octocrab
            .issues("openmls", "annotations")
            .update(issue_number)
            .title(&format!("[Validation] {}", annotation.title()))
            .body(&body)
            .send()
            .await
        {
            Ok(_) => StatusCode::ACCEPTED,
            Err(_) => StatusCode::NOT_ACCEPTABLE,
        }
    } else {
        match octocrab
            .issues("openmls", "annotations")
            .create(format!("[Validation] {}", annotation.title()))
            .labels(Some(vec!["validation".to_string()]))
            .body(body)
            .send()
            .await
        {
            Ok(_) => StatusCode::CREATED,
            Err(_) => StatusCode::NOT_ACCEPTABLE,
        }
    }
}

async fn delete_annotation() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn find_issue(annotation: &Annotation) -> Option<u64> {
    let octocrab = octocrab::instance();

    let mut page = octocrab
        .issues("openmls", "annotations")
        .list()
        .labels(&[String::from("validation")])
        .state(params::State::All)
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if let Some(body) = &issue.body {
                if let Some(Ok(issue_annotation)) = extract_annotation(body) {
                    if issue_annotation.id == annotation.id {
                        return Some(issue.number);
                    }
                }
            }
        }
        page = match octocrab
            .get_page::<models::issues::Issue>(&page.next)
            .await
            .unwrap()
        {
            Some(next_page) => next_page,
            None => break,
        }
    }

    None
}
