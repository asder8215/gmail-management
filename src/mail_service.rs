extern crate google_gmail1 as gmail1;

use core::str;
use gmail1::api::{Message, UserMessageListCall};
use gmail1::hyper::client::HttpConnector;
use gmail1::hyper_rustls::HttpsConnector;
use gmail1::{hyper, hyper_rustls, oauth2, Gmail};
use is_empty::IsEmpty;
use lettre::message::{Attachment, Body, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::Message as email;
use lettre::{SmtpTransport, Transport};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, read, read_to_string, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as tokio_mutex;

use crate::cmd_args::{Filter, Send, SendInfo};
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
pub async fn get_message(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_id: &str,
) -> Result<Message, Box<dyn std::error::Error>> {
    let result = hub
        .users()
        .messages_get("me", msg_id)
        .add_scope("https://mail.google.com/")
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
    send: Send,
    json_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut email = email::builder();
    let mut send_details = SendInfo {
        from: send.from,
        to: send.to,
        cc: send.cc,
        bcc: send.bcc,
        subject: send.subject,
        description: send.description,
        attachment: send.attachment,
    };

    // if json file provided, then use json info
    if let Some(json_file) = json_file {
        let send_string = fs::read_to_string(json_file).expect("Unable to read file.");
        send_details = serde_json::from_str(&send_string).expect("JSON was not well-formatted");
    }

    // Setting up to, from, cc, bcc, subject field
    if let Some(from) = send_details.from {
        let from = from.parse::<Mailbox>()
            .map_err(|e|
                format!("The address {} in from field is not parsable. Check if you have correctly type the email address.\nError Received: {}", from, e)
            )?;
        email = email.from(from);
    }

    if let Some(to) = send_details.to {
        for t in to {
            let to = t.parse::<Mailbox>()
                .map_err(|e|
                    format!("The address {} in to field is not parsable. Check if you have correctly type the email address.\nError Received: {}", t, e)
                )?;
            email = email.to(to);
        }
    }

    if let Some(cc) = send_details.cc {
        for c in cc {
            let cc = c.parse::<Mailbox>()
                .map_err(|e|
                    format!("The address {} in cc field is not parsable. Check if you have correctly type the email address.\nError Received: {}", c, e)
                )?;
            email = email.cc(cc);
        }
    }

    if let Some(bcc) = send_details.bcc {
        for b in bcc {
            let bcc = b.parse::<Mailbox>()
                .map_err(|e|
                    format!("The address {} in bcc field is not parsable. Check if you have correctly type the email address.\nError Received: {}", b, e)
                )?;
            email = email.bcc(bcc);
        }
    }

    if let Some(subject) = send_details.subject {
        email = email.subject(subject);
    }

    // doing the body of the email and attachment together
    // need to check mime type of the attachment to set up right content_type in email
    // if attachment isn't able to be mime_guessed error on email
    let desc_multipart;
    if let Some(desc) = send_details.description {
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
    if let Some(attachments) = send_details.attachment {
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
    if let (Some(username), Some(password)) = (send.username, send.password) {
        creds = Credentials::new(username.to_owned(), password.to_owned());
        let credentials_json = r#json!({send.relay.clone(): {"user": username, "pass": password}});
        fs::write(
            "credentials.json",
            serde_json::to_string_pretty(&credentials_json).unwrap(),
        )?;
    } else {
        let cred_file = fs::File::open("credentials.json").expect("File should open read only");
        let cred_json: serde_json::Value =
            serde_json::from_reader(cred_file).expect("JSON was not well-formatted");
        let relay_val = cred_json
            .get(send.relay.clone())
            .ok_or("Couldn't get user from credentials.json")?;
        let (username, password) = (
            relay_val
                .get("user")
                .ok_or("Couldn't get user from credentials.json")?,
            relay_val
                .get("pass")
                .ok_or("Couldn't get user from credentials.json")?,
        );
        creds = Credentials::new(
            username.as_str().unwrap().to_owned(),
            password.as_str().unwrap().to_owned(),
        );
    }

    // Open a secure connection to the SMTP server using STARTTLS
    let mailer = SmtpTransport::starttls_relay(&send.relay)
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

/// Parse query given all filled in field from Filter
async fn query_parse(filter: Filter) -> Result<String, Box<dyn std::error::Error>> {
    let mut result = "".to_string();

    if let Some(words) = filter.words {
        for word in words {
            result.push_str(format!("{} ", word).as_str());
        }
    }

    if let Some(from) = filter.from {
        for f in from {
            result.push_str(format!("from:{} ", f).as_str());
        }
    }

    if let Some(to) = filter.to {
        for t in to {
            result.push_str(format!("to:{} ", t).as_str());
        }
    }

    if let Some(cc) = filter.cc {
        for c in cc {
            result.push_str(format!("cc:{} ", c).as_str());
        }
    }

    if let Some(bcc) = filter.bcc {
        for b in bcc {
            result.push_str(format!("bcc:{} ", b).as_str());
        }
    }

    if let Some(subject) = filter.subject {
        for s in subject {
            result.push_str(format!("subject:{} ", s).as_str());
        }
    }

    if let Some(remove_words) = filter.remove_words {
        for remove_word in remove_words {
            result.push_str(format!("-{} ", remove_word).as_str());
        }
    }

    if let Some(labels) = filter.labels {
        for l in labels {
            result.push_str(format!("label:{} ", l).as_str());
        }
    }

    if let Some(has) = filter.has {
        for h in has {
            result.push_str(format!("has:{} ", h).as_str());
        }
    }

    if let Some(list) = filter.list {
        for l in list {
            result.push_str(format!("list:{} ", l).as_str());
        }
    }

    if let Some(filename) = filter.filename {
        for f in filename {
            result.push_str(format!("filename:{} ", f).as_str());
        }
    }

    if let Some(r#in) = filter.r#in {
        for i in r#in {
            result.push_str(format!("in:{} ", i).as_str());
        }
    }

    if let Some(is) = filter.is {
        for i in is {
            result.push_str(format!("is:{} ", i).as_str());
        }
    }

    if let Some(after) = filter.after {
        result.push_str(format!("after:{} ", after).as_str());
    }

    if let Some(before) = filter.before {
        result.push_str(format!("before:{} ", before).as_str());
    }

    if let Some(older_than) = filter.older_than {
        result.push_str(format!("older_than:{} ", older_than).as_str());
    }

    if let Some(newer_than) = filter.newer_than {
        result.push_str(format!("newer_than:{} ", newer_than).as_str());
    }

    if let Some(deliveredto) = filter.deliveredto {
        for d in deliveredto {
            result.push_str(format!("deliveredto:{} ", d).as_str());
        }
    }

    if let Some(category) = filter.category {
        for c in category {
            result.push_str(format!("category:{} ", c).as_str());
        }
    }

    if let Some(rfc822msgid) = filter.rfc822msgid {
        for r in rfc822msgid {
            result.push_str(format!("rfc822msgid:{} ", r).as_str());
        }
    }

    if let Some(size) = filter.size {
        result.push_str(format!("size:{} ", size).as_str());
    }

    if let Some(larger) = filter.larger {
        result.push_str(format!("larger:{} ", larger).as_str());
    }

    if let Some(smaller) = filter.smaller {
        result.push_str(format!("smaller:{} ", smaller).as_str());
    }

    Ok(result)
}

/// Parse query given in a text file
async fn text_query_parse(text_file_path: String) -> Result<String, Box<dyn std::error::Error>> {
    let result = read_to_string(text_file_path)?;
    Ok(result)
}

/// Parse query given in a json file
async fn json_query_parse(json_file_path: String) -> Result<String, Box<dyn std::error::Error>> {
    let filter_string = fs::read_to_string(json_file_path).expect("Unable to read file.");
    let filter: Filter = serde_json::from_str(&filter_string).expect("JSON was not well-formatted");
    query_parse(filter).await
}

/// Return a Message List of all containing all Messages related to the page_token or query provided.
pub async fn list_messages<'a>(
    hub: &'a Gmail<HttpsConnector<HttpConnector>>,
    page_token: Option<&'a String>,
    filter: Option<Filter>,
) -> UserMessageListCall<'a, HttpsConnector<HttpConnector>> {
    let mut result = hub.users().messages_list("me");

    if let Some(page_token) = page_token {
        result = result.page_token(page_token);
    }

    if let Some(filter) = filter {
        let query_result;

        if let Some(text_file) = filter.text_file.clone() {
            query_result = text_query_parse(text_file).await;
        } else if let Some(json_file) = filter.json_file.clone() {
            query_result = json_query_parse(json_file).await;
        } else {
            query_result = query_parse(filter.clone()).await;
        }
        match query_result {
            Ok(res) => {
                let query_str = &res;
                if filter.is_empty() {
                    result = result.max_results(0);
                }
                else{
                    // query up search with given user inputs from either text, json, or manual querying.
                    result = result.q(query_str).max_results(500);
                }
            }
            Err(e) => {
                println!(
                    "The query search does not contain proper query information.\n Error received: {}",
                    e
                );
                result = result.max_results(0);
            }
        }
    }

    result = result.add_scope("https://mail.google.com/");

    result
}

/// Modifies the given Arc<tokio_mutex<BTreeSet>> with all email message id from label id
pub async fn get_msg_ids_from_messages(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    label_id: Option<&str>,
    filter: Option<Filter>,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    let mut fetch_emails: bool = true;
    let mut message_list: UserMessageListCall<HttpsConnector<HttpConnector>> =
        list_messages(hub, None, filter.clone()).await;

    if let Some(label_id) = label_id {
        message_list = message_list.add_label_ids(label_id);
    }

    let mut result = message_list.doit().await;

    while fetch_emails {
        // Displays whether the result indicates a successful connection or a failed one
        let messages = match result {
            Err(e) => {
                println!("{}", e);
                return;
            }
            Ok(ref res) => res.1.clone(),
        };

        if let Some(gmail_messages) = messages.messages.to_owned() {
            for msg in gmail_messages {
                let mut msg_id_bts_lock = msg_id_bts.lock().await;
                msg_id_bts_lock.insert(Some(msg.id.clone().unwrap()));
            }
        }

        if let Some(page_token) = &messages.next_page_token {
            let mut message_list: UserMessageListCall<HttpsConnector<HttpConnector>> =
                list_messages(hub, Some(page_token), filter.clone()).await;

            if let Some(label_id) = label_id {
                message_list = message_list.add_label_ids(label_id);
            }

            result = message_list.doit().await;
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

/// Checks if a label name provided by user exists and passes the label id of the label name
/// to retrieve all messages and add it to the BTreeSet
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
        get_msg_ids_from_messages(hub, label_id.as_deref(), None, msg_id_bts.clone()).await;
    }
}

/// Add msgs ids to the BTreeSet from provided message ids
pub async fn add_msg_ids_from_ids(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_ids: Vec<String>,
    msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>>,
) {
    for msg_id in msg_ids {
        // The if statement is intentional in order to check if the msg_id points to a valid message in user's gmail
        if let Ok(_msg) = get_message(hub, &msg_id).await {
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
                // Displays whether the message was trashed or something failed
                match result {
                    Ok(_res) => {}
                    Err(e) => println!(
                        "Could not trash message with id {}.\nError Received: {}",
                        msg_id, e
                    ),
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
    counter
}

/// Dequerer threads in the filter command utilize this method to grab the msg id
/// from the ring buffer and get message content to write to output txt file
pub async fn print_msgs(
    hub: &Gmail<HttpsConnector<HttpConnector>>,
    msg_id_rb: &MultiThreadedRingBuffer<String>,
    output_file: String,
    file_lock: Arc<Mutex<i32>>,
) -> usize {
    let mut counter: usize = 0;
    loop {
        let msg_id = msg_id_rb.dequeue().await;
        match msg_id {
            Some(msg_id) => {
                counter += 1;
                let result = get_message(hub, &msg_id).await;
                // Displays whether the message was received or not
                match result {
                    Ok(res) => {
                        let mut output_file_clone = output_file.clone();
                        output_file_clone.push_str(".txt");
                        let mut file;

                        // Lock so that data races between threads don't happen on writing to the
                        // file
                        let file_lock = file_lock.lock().unwrap();

                        // Check if file exist; if not, create it, if yes, append to it
                        if !Path::new(&output_file_clone).exists() {
                            file = File::create(output_file_clone).expect("Creating file failed");
                        } else {
                            file = OpenOptions::new()
                                .append(true)
                                .open(output_file_clone)
                                .expect("Could not open file");
                        }

                        // Creating message details for txt file
                        let mut msg_id = "Not found".to_string();
                        let mut from = "Not found".to_string();
                        let mut to = "Not found".to_string();
                        let mut subject = "Not found".to_string();
                        let mut date = "Not found".to_string();
                        let mut description = "Not found".to_string();

                        if let Some(id) = res.id {
                            msg_id = id;
                        }

                        if let Some(payload) = res.payload {
                            if let Some(headers) = payload.headers {
                                // Grabbing to, from, subject, and date info
                                for header in headers {
                                    if let (Some(name), Some(value)) = (header.name, header.value) {
                                        match name.as_str() {
                                            "To" => to = value.clone(),
                                            "From" => from = value.clone(),
                                            "Date" => date = value.clone(),
                                            "Subject" => subject = value.clone(),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            // Grabbing description of the email
                            if let Some(parts) = payload.parts {
                                'parts: for part in parts {
                                    if let Some(headers) = part.headers {
                                        for header in headers {
                                            if let (Some(name), Some(value)) =
                                                (header.name, header.value)
                                            {
                                                if name == "Content-Type"
                                                    && value.starts_with("text/plain")
                                                {
                                                    if let Some(body) = &part.body {
                                                        if let Some(data) = &body.data {
                                                            description = str::from_utf8(data)
                                                                .expect("Invalid utf8 data")
                                                                .to_string();
                                                        }
                                                    }
                                                    // breaks at the specific label
                                                    break 'parts;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        file.write_all(format!("Message ID: {}\nFrom: {}\nTo: {}\nDate: {}\nSubject: {}\nBody: {}\n\n", msg_id, from, to, date, subject, description).as_bytes()).expect(
                            "Couldn't write to file.");
                        drop(file_lock)
                    }
                    Err(e) => println!(
                        "Could not find message with id {}.\nError Received: {}",
                        msg_id, e
                    ),
                };
            }
            None => {
                break;
            }
        }
    }
    counter
}
