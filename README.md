# Annotations

The tool at hand provides an interface that allows to create text annotations on a plaintext document that are synced through GitHub issues.

## Example

![image](https://user-images.githubusercontent.com/8997731/211362554-41bcd634-cf07-4163-bd6e-d6e34b08deac.png)

# Setup

First, you need to create a `pat.txt` in the root of the repository.
This file must contain a (fine-grained) [Personal Access Token (PAT)] with the capability to manage issues on the repository you intend to sync this tool with.
We recommend that you give this token the least possible privileges, i.e., to manage issues in a single repository.

(Note, the repository as well as the document are currently hard-coded to `openmls/annotations` and `draft-ietf-mls-protocol-17.txt` -- will fix later.)

Then, execute ...

```sh
cargo run
```

... on your local machine and head to http://127.0.0.1:3000 to view your document.

# Usage

When a document is loaded, the tool extracts all annotations from GitHub and shows them as annotations in the document.
For this to work, the tool reads the `annotation` meta data that is attached to every issue body (and does not need to be edited).

Annotations have different colors:

* red annotations refer to issues that are still open, i.e., need to be handled (or closed).
* green annotations refer to closed, i.e., already handled issues.
* orange annotations refer to newly created issues that were not obtained from GitHub.
  Note: orange annotations are still automatically created on GitHub.
  If you want to make sure that there is no bug, reload the page and verify that the annotations you created become red or green.

New annotations can be created by selecting text in the browser.
When the comment is empty, the tool will use the selected text as issue title.
Otherwise, it will use the provided comment as issue title.
You can use tags such as "codec", "leaf_node", "credential", etc. to your liking to classify the annotations.

# What works and what doesn't?

Creating and updating issues works as expected. Note, however, that issues must be deleted (or closed) via GitHub.

Please don't judge the code quality. This is an experiment we wrote to verify if we like this workflow.

[personal access token (pat)]: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token
