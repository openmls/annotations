use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub owner: String,
    pub repo: String,
    pub pat: String,
    pub document: String,
}

#[derive(Clone, Debug)]
pub struct ServerState {
    pub config: Config,
}

// -------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Annotation {
    #[serde(rename = "@context")]
    pub context: String,
    pub r#type: String,
    pub body: Vec<Body>,
    pub target: Target,
    pub id: String,
    pub meta: Option<GitHubStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Body {
    TextualBody { purpose: String, value: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Target {
    selector: Vec<Selector>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Selector {
    TextQuoteSelector { exact: String },
    TextPositionSelector { start: usize, end: usize },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GitHubStatus {
    Open,
    Closed,
}

impl Annotation {
    pub fn title(&self) -> String {
        if let Some(comment) = self.comment() {
            comment
        } else if let Some(quote) = self.quote() {
            quote
        } else {
            "<error: no title>".to_string()
        }
    }

    pub fn quote(&self) -> Option<String> {
        let mut quote = None;

        for selector in self.target.selector.iter() {
            match selector {
                Selector::TextQuoteSelector { exact } => {
                    let my_quote: String = exact
                        .lines()
                        .map(|line| format!("{} ", line.trim()))
                        .collect();

                    quote = Some(my_quote.trim().to_string())
                }
                _ => {}
            }
        }

        quote
    }

    pub fn comment(&self) -> Option<String> {
        let mut comment = None;

        for body in self.body.iter() {
            match body {
                Body::TextualBody { purpose, value } if purpose == "commenting" => {
                    comment = Some(value.into())
                }
                _ => {}
            }
        }

        comment
    }

    pub fn text_quote_selector(&self) -> Option<String> {
        match self.target.selector.first()? {
            Selector::TextQuoteSelector { exact } => {
                let quote: String = exact
                    .lines()
                    .map(|line| format!("> {}\r\n", line.trim()))
                    .collect();
                Some(quote.trim().to_string())
            }
            _ => None,
        }
    }
}

// -------------------------------------------------------------------------------------------------

// Return a (prefix, Annotation, suffix) tuple.
pub fn extract_annotation(text: &str) -> Option<(&str, serde_json::Result<Annotation>, &str)> {
    if let Some((prefix, remaining)) = text.split_once("```annotation\r\n") {
        if let Some((annotation, suffix)) = remaining.split_once("\r\n```") {
            return Some((prefix, serde_json::from_str(annotation), suffix));
        }
    }

    None
}
