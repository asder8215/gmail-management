extern crate google_gmail1 as gmail1;

use gmail1::api::Message;
use gmail1::hyper::client::HttpConnector;
use gmail1::hyper_rustls::HttpsConnector;
use gmail1::{hyper, hyper_rustls, oauth2, Gmail};
use lettre::message::{Attachment, Body, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::Message as email;
use lettre::{SmtpTransport, Transport};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, read};
use std::sync::Arc;
use tokio::sync::Mutex as tokio_mutex;

use crate::ringbuffer::MultiThreadedRingBuffer;

/// Attempts to authenticate and connect to user's email; returns the connected client on success
/// Need to create a service account on Google Cloud Platform Console and put the client id in a client_secret.json
///
/// You can follow this for more info: [Google Cloud Help](https://support.google.com/cloud/answer/6158849?hl=en#:~:text=Go%20to%20the%20Google%20Cloud%20Platform%20Console%20Credentials%20page.,to%20add%20a%20new%20secret.)
///
/// Much of this code inspired from: [Google Gmail1 Doc](https://docs.rs/google-gmail1/latest/google_gmail1/index.html)
pub async fn create_client(
) -> Result<Gmail<HttpsConnector<HttpConnector>>, Box<dyn std::error::Error>> {
    // Get an ApplicationSecret instance by some means. It contains the `client_id` and
    // `client_secret`, among other things.

    let secret = oauth2::read_application_secret("./client_secret.json")
        .await
        .map_err(|e| format! {"No client_secret.json.\nError Received: {}", e})?;

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file named tokencache.json. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("./tokencache.json")
    .build()
    .await?;

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();

    let hub = Gmail::new(hyper::Client::builder().build(https), auth);

    // Test connection to see if user is authenticated and info can be retrieved
    hub.users()
        .get_profile("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await?;

    println!("Successful authenticated connection\n");

    Ok(hub)
}

/// Fetches message from authenticated user's email given a message id
/// Returns None if the message is nonexistent
pub async fn get_messages(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_ids: &str,
) -> Result<Message, Box<dyn std::error::Error>> {
    let result = hub
        .users()
        .messages_get("me", msg_ids)
        .add_scope("https://mail.google.com/")
        .format("RAW")
        .doit()
        .await?;

    Ok(result.1)
}

/// Send an email message to up to 100 users in to, cc, and bcc field respectively from a given mail sending host service using SMTP protocol.
///
/// Code for building an email and sending mostly inspired by [Mailtrap](https://mailtrap.io/blog/rust-send-email/#How-to-send-an-email-with-attachments-in-Rust)
///
/// Storing and using credentials inspired by this [Stackoverflow post](https://stackoverflow.com/questions/30292752/how-do-i-parse-a-json-file)
///
/// mime_guess library used to have a flexible way of resolving content-type of the attachments to an email
pub async fn send_message(
    username: Option<String>,
    password: Option<String>,
    relay: String,
    from: String,
    to_names: Vec<String>,
    cc_names: Vec<String>,
    bcc_names: Vec<String>,
    subject: Option<String>,
    desc: Option<String>,
    attachments: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut email = email::builder();

    // Setting up to, from, cc, bcc, subject field
    let from = from.parse::<Mailbox>()
        .map_err(|e|
            format!("The address {} in from field is not parsable. Check if you have correctly type the email address.\nError Received: {}", from, e)
        )?;
    email = email.from(from);

    for to in to_names {
        let to = to.parse::<Mailbox>()
            .map_err(|e|
                format!("The address {} in to field is not parsable. Check if you have correctly type the email address.\nError Received: {}", to, e)
            )?;
        email = email.to(to);
    }

    for cc in cc_names {
        let cc = cc.parse::<Mailbox>()
            .map_err(|e|
                format!("The address {} in cc field is not parsable. Check if you have correctly type the email address.\nError Received: {}", cc, e)
            )?;
        email = email.cc(cc);
    }

    for bcc in bcc_names {
        let bcc = bcc.parse::<Mailbox>()
            .map_err(|e|
                format!("The address {} in bcc field is not parsable. Check if you have correctly type the email address.\nError Received: {}", bcc, e)
            )?;
        email = email.bcc(bcc);
    }

    if let Some(subject) = subject {
        email = email.subject(subject);
    }

    // doing the body of the email and attachment together
    // need to check mime type of the attachment to set up right content_type in email
    // if attachment isn't able to be mime_guessed error on email
    let desc_multipart;
    if let Some(desc) = desc {
        desc_multipart = MultiPart::mixed().multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(desc.clone()))
                .multipart(
                    MultiPart::related().singlepart(SinglePart::html(format!("<p>{}</p>", desc))),
                ),
        )
    } else {
        desc_multipart = MultiPart::mixed().multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain("".to_string()))
                .multipart(
                    MultiPart::related().singlepart(SinglePart::html("<p></p>".to_string())),
                ),
        )
    }

    let mut attachment_singleparts: Vec<SinglePart> = Vec::new();
    if let Some(attachments) = attachments {
        for attachment in attachments {
            let attachment_file = read(attachment.clone())?;
            let attachment_body = Body::new(attachment_file);
            let guess = mime_guess::from_path(attachment.clone()).first_raw();
            if let Some(guess) = guess {
                attachment_singleparts.push(
                    Attachment::new(attachment.clone())
                        .body(attachment_body, guess.parse().unwrap()),
                );
            } else {
                return Err("Unable to mime guess this attachment file.".into());
            }
        }
    }

    let mut desc_and_attachment_parts = desc_multipart;

    for attachment_part in attachment_singleparts {
        desc_and_attachment_parts = desc_and_attachment_parts.singlepart(attachment_part);
    }

    let lettre_msg = email.multipart(desc_and_attachment_parts.clone())?;

    // Create SMTP client credentials using username and password
    // Stores the last used username and password in credentials.json so it's not necessary for
    // users of this program to relogin
    let creds: Credentials;
    if let (Some(username), Some(password)) = (username, password) {
        creds = Credentials::new(username.to_owned(), password.to_owned());
        let credentials_json = r#json!({"user": username, "pass": password});
        fs::write(
            "credentials.json",
            serde_json::to_string_pretty(&credentials_json).unwrap(),
        )?;
    } else {
        let cred_file = fs::File::open("credentials.json").expect("File should open read only");
        let cred_json: serde_json::Value =
            serde_json::from_reader(cred_file).expect("JSON was not well-formatted");
        let username = cred_json
            .get("user")
            .ok_or("Couldn't get user from credentials.json")?;
        let password = cred_json
            .get("pass")
            .ok_or("Couldn't get pass from credentials.json")?;
        creds = Credentials::new(
            username.as_str().unwrap().to_owned(),
            password.as_str().unwrap().to_owned(),
        );
    }

    // Open a secure connection to the SMTP server using STARTTLS
    let mailer = SmtpTransport::starttls_relay(&relay)
        .unwrap() // Unwrap the Result, panics in case of error
        .credentials(creds) // Provide the credentials to the transport
        .build(); // Construct the transport

    // Attempt to send the email via the SMTP transport
    mailer
        .send(&lettre_msg)
        .map_err(|e| format!("Could not send email: {:?}", e))?;
    println!("Email sent successfully! Check your mail service platform just in case the email bounced or got rejected.");

    Ok(())
}

#[allow(dead_code)]
/// Return a BTreeSet of all email message ids. Unused for now in scheme of program, but good for debugging purposes.
pub async fn list_messages(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    let result = hub
        .users()
        .messages_list("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await;

    let (mut _res, messages);
    // Displays whether the result indicates a successful connection or a failed one
    match result {
        Err(e) => {
            println!("{}", e);
            return;
        }
        Ok(res) => {
            (_res, messages) = res;
        }
    };

    if let Some(gmail_messages) = messages.messages.to_owned() {
        for msg in gmail_messages {
            let mut msg_id_bts_lock = msg_id_bts.lock().await;
            msg_id_bts_lock.insert(Some(msg.id.clone().unwrap()));
        }
    }
}

/// Modifies the given Arc tokio_mutex BTreeSet with all email message id from label id
async fn list_messages_by_label(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    label_id: &str,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    let mut fetch_emails = true;

    let result = hub
        .users()
        .messages_list("me")
        .add_label_ids(label_id)
        .add_scope("https://mail.google.com/")
        .doit()
        .await;

    let (mut _res, mut messages);
    // Displays whether the result indicates a successful connection or a failed one
    match result {
        Err(e) => {
            println!("{}", e);
            return;
        }
        Ok(res) => {
            (_res, messages) = res;
        }
    };

    while fetch_emails {
        if let Some(gmail_messages) = messages.messages.to_owned() {
            for msg in gmail_messages {
                let mut msg_id_bts_lock = msg_id_bts.lock().await;
                msg_id_bts_lock.insert(Some(msg.id.clone().unwrap()));
            }
        }

        if let Some(page_token) = &messages.next_page_token {
            let result = hub
                .users()
                .messages_list("me")
                .page_token(page_token)
                .add_label_ids(label_id)
                .add_scope("https://mail.google.com/")
                .doit()
                .await;

            // Displays whether the result indicates a successful connection or a failed one
            match result {
                Err(e) => {
                    println!("{}", e);
                    return;
                }
                Ok(res) => {
                    (_res, messages) = res;
                }
            };
        } else {
            fetch_emails = false;
        }
    }
}

/// Return a BTreeMap of label names and ids within user's email
pub async fn list_labels(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let result = hub.users().labels_list("me").doit().await?;
    let (_res, labels_list) = result;
    let mut label_map = BTreeMap::<String, String>::default();
    for label in labels_list.labels.as_ref().unwrap() {
        if let (Some(label_name), Some(label_id)) = (label.name.to_owned(), label.id.to_owned()) {
            label_map.insert(label_name, label_id);
        }
    }

    Ok(label_map)
}

/// Retrieves label id given label name
pub async fn get_label_id(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    label_name: &String,
) -> Option<String> {
    let label_id = list_labels(hub).await;

    match label_id {
        Err(e) => {
            println!("Labels list unsuccessfully received.\nError Received {}", e);
            None
        }
        Ok(res) => res.get(label_name).cloned(),
    }
}

/// Trashes all emails from given labels
pub async fn add_msg_ids_from_labels(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    label_names: Vec<String>,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    for label in label_names {
        let label_id = get_label_id(hub, &label).await;

        if label_id.is_none() {
            println!("{} is a nonexistent label name", label);
            continue;
        }
        list_messages_by_label(hub, &label_id.unwrap(), msg_id_bts.clone()).await;
    }
}

/// Trash emails from provided message ids
pub async fn add_msg_ids_from_ids(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_ids: Vec<String>,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    for msg_id in msg_ids {
        // The if statement is intentional in order to check if the msg_id points to a valid message in user's gmail
        if let Ok(_msg) = get_messages(hub, &msg_id).await {
            let mut msg_id_bts_lock = msg_id_bts.lock().await;
            msg_id_bts_lock.insert(Some(msg_id));
        } else {
            println!("{} is a nonexistent message id", msg_id);
            continue;
        }
    }
}

/// Dequerer threads in the trash command utilize this method to grab the msg id
/// from the ring buffer and trash it
pub async fn trash_msgs(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_id_rb: &MultiThreadedRingBuffer<String>,
) -> usize {
    let mut counter: usize = 0;
    loop {
        let msg_id = msg_id_rb.dequeue().await;
        match msg_id {
            Some(msg_id) => {
                counter += 1;
                let result = hub.users().messages_trash("me", &msg_id).doit().await;
                // Displays whether the result indicates a successful connection or a failed one
                match result {
                    Err(e) => println!(
                        "Could not trash message with id {}.\nError Received: {}",
                        msg_id, e
                    ),
                    Ok(_res) => {}
                };
            }
            None => {
                break;
            }
        }
    }
    counter
}

/// Enquerer threads from the trash command use this method to fetch msg ids
/// as it's being added to the BTS and enqueues it to the ring buffer
pub async fn add_msgs(
    msg_ids: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
    msg_id_rb: &MultiThreadedRingBuffer<String>,
) -> usize {
    let mut counter: usize = 0;

    loop {
        // Lock the bts so that you can read popped item and remove it from the bts (read/write lock)
        let mut msg_id_bts_lock = msg_ids.lock().await;
        match msg_id_bts_lock.pop_first() {
            Some(msg_id) => {
                // enqueue the msg_id
                if let Some(msg_id) = msg_id {
                    counter += 1;
                    msg_id_rb.enqueue(msg_id).await;
                }
                // item is None here
                else {
                    break;
                }
            }
            // Unless the item explicitly given to me is None, keep continuing on popping the msg_id_bts
            None => {
                continue;
            }
        }
    }
    // return
    counter
}

#[allow(dead_code)]
#[allow(unused_variables)]
pub async fn find_messages(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    pattern: String,
    label_names: Option<Vec<String>>,
) {
    todo!()
}
