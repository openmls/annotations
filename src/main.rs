use std::net::SocketAddr;

use annotations::{extract_annotation, Annotation, Config, GitHubStatus, Mode, ServerState};
use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use octocrab::{
    models,
    models::{issues::Issue, IssueState},
    params,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config: Config = {
        let config_data = std::fs::read_to_string("config.ron").unwrap();

        ron::from_str(&config_data).unwrap()
    };

    match &config.mode {
        Mode::ReadOnly => {
            octocrab::initialise(octocrab::Octocrab::builder()).unwrap();
        }
        Mode::ReadWrite(pat) => {
            // Read Personal Access Token (PAT)
            let pat = std::fs::read_to_string(pat).unwrap().trim().to_string();

            octocrab::initialise(octocrab::Octocrab::builder().personal_token(pat.into())).unwrap();
        }
    }

    let state = ServerState { config };

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(index))
        .route("/mode", get(get_mode))
        .route("/recogito/recogito.min.css", get(css))
        .route("/recogito/recogito.min.css.map", get(css_map))
        .route("/recogito/recogito.min.js", get(js))
        .route("/recogito/recogito.min.js.map", get(js_map))
        .route("/favicon.ico", get(favicon))
        .route("/annotations", get(get_annotations))
        .route("/annotation", post(post_annotation))
        .route("/annotation", delete(delete_annotation))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Please head over to http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index(State(state): State<ServerState>) -> impl IntoResponse {
    let page = {
        let index = std::fs::read_to_string("frontend/index.html").unwrap();
        let document = std::fs::read_to_string(&state.config.document).unwrap();

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

async fn get_mode(State(state): State<ServerState>) -> impl IntoResponse {
    let data = match state.config.mode {
        Mode::ReadOnly => "{ \"readOnly\": true }",
        Mode::ReadWrite(_) => "{ \"readOnly\": false }",
    };

    Response::builder()
        .header("Content-Type", "application/json")
        .body(data.to_string())
        .unwrap()
}

async fn get_annotations(State(state): State<ServerState>) -> impl IntoResponse {
    let octocrab = octocrab::instance();

    let mut annotations = Vec::new();

    let mut page = octocrab
        .issues(state.config.owner, state.config.repo)
        .list()
        .labels(&[String::from("annotation")])
        .state(params::State::All)
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if let Some(body) = &issue.body {
                if let Some((_, Ok(mut annotation), _)) = extract_annotation(body) {
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

async fn post_annotation(
    State(state): State<ServerState>,
    Json(annotation): Json<Annotation>,
) -> impl IntoResponse {
    let octocrab = octocrab::instance();

    if let Some((issue, (prefix, _, suffix))) = find_issue(&state, &annotation).await {
        let body = format!(
            "{}```annotation\r\n{}\r\n```{}",
            prefix,
            serde_json::to_string_pretty(&annotation).unwrap(),
            suffix,
        );

        match octocrab
            .issues(state.config.owner, state.config.repo)
            .update(issue.number)
            .title(&format!("[Annotation] {}", annotation.title()))
            .body(&body)
            .send()
            .await
        {
            Ok(_) => StatusCode::ACCEPTED,
            Err(_) => StatusCode::NOT_ACCEPTABLE,
        }
    } else {
        let body = format!(
            "{}\r\n\r\n---\r\n\r\n<details><summary>Annotation</summary>\r\n\r\n```annotation\r\n{}\r\n```\r\n</details>",
            annotation
                .text_quote_selector()
                .unwrap_or("```\r\n<no quote>\r\n```".into()),
            serde_json::to_string_pretty(&annotation).unwrap()
        );

        match octocrab
            .issues(state.config.owner, state.config.repo)
            .create(format!("[Annotation] {}", annotation.title()))
            .labels(Some(vec!["annotation".to_string()]))
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

async fn find_issue(
    state: &ServerState,
    annotation: &Annotation,
) -> Option<(Issue, (String, Annotation, String))> {
    let octocrab = octocrab::instance();

    let mut page = octocrab
        .issues(&state.config.owner, &state.config.repo)
        .list()
        .labels(&[String::from("annotation")])
        .state(params::State::All)
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if let Some(body) = &issue.body {
                if let Some((prefix, Ok(issue_annotation), suffix)) = extract_annotation(body) {
                    if issue_annotation.id == annotation.id {
                        return Some((
                            issue.clone(),
                            (prefix.to_string(), issue_annotation, suffix.to_string()),
                        ));
                    }
                }
            }
        }
        page = match octocrab.get_page::<Issue>(&page.next).await.unwrap() {
            Some(next_page) => next_page,
            None => break,
        }
    }

    None
}
