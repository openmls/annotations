use octocrab::{models::issues::Issue, OctocrabBuilder, Page};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = std::fs::read_to_string("pat.txt")
        .unwrap()
        .trim()
        .to_string();

    octocrab::initialise(OctocrabBuilder::new().personal_token(token)).unwrap();

    // update_issue_title().await;
    // update_issue_labels().await;
}

#[allow(unused)]
async fn update_issue_title() {
    let oc = octocrab::instance();

    let mut page: Page<Issue> = oc
        .issues("openmls", "annotations")
        .list()
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if issue.title.starts_with("[Validation]") {
                println!("Updating: {}", issue.title);

                let new_title = issue
                    .title
                    .as_str()
                    .replace("[Validation]", "[Annotation]")
                    .to_string();

                oc.issues("openmls", "annotations")
                    .update(issue.number)
                    .title(&new_title)
                    .send()
                    .await
                    .unwrap();
            }
        }

        page = match oc.get_page::<Issue>(&page.next).await.unwrap() {
            Some(next_page) => next_page,
            None => break,
        }
    }
}

#[allow(unused)]
async fn update_issue_labels() {
    let oc = octocrab::instance();

    let mut page: Page<Issue> = oc
        .issues("openmls", "annotations")
        .list()
        .per_page(50)
        .send()
        .await
        .unwrap();

    loop {
        for issue in &page {
            if issue.title.starts_with("[Annotation]") {
                println!("Updating: {}", issue.title);

                let validation_label = oc
                    .issues("openmls", "annotations")
                    .get_label("validation")
                    .await
                    .unwrap();
                let annotation_label = oc
                    .issues("openmls", "annotations")
                    .get_label("annotation")
                    .await
                    .unwrap();

                let mut labels = issue.labels.to_vec();

                for label in labels.iter_mut() {
                    if label.id == validation_label.id {
                        *label = annotation_label.clone();
                    }
                }

                let labels = labels.iter().map(|l| l.name.clone()).collect::<Vec<_>>();

                oc.issues("openmls", "annotations")
                    .update(issue.number)
                    .labels(&labels)
                    .send()
                    .await
                    .unwrap();
            }
        }

        page = match oc.get_page::<Issue>(&page.next).await.unwrap() {
            Some(next_page) => next_page,
            None => break,
        }
    }
}
